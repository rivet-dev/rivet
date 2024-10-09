use std::{
	collections::HashMap,
	net::{IpAddr, SocketAddr},
	sync::Arc,
};

use chirp_workflow::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde_json::json;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};

use pegboard::types::{ClientEvent, Command};

type Connections = HashMap<Uuid, Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-ws");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-ws",
	)
	.await?;

	let conns: Arc<RwLock<Connections>> = Arc::new(RwLock::new(HashMap::new()));

	tokio::try_join!(
		socket_thread(&ctx, conns.clone()),
		signal_thread(&ctx, conns.clone()),
	)?;

	Ok(())
}

async fn socket_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) -> GlobalResult<()> {
	// TODO:
	let addr = ("127.0.0.1", 8080);
	let listener = TcpListener::bind(&addr).await?;
	tracing::info!("Listening on: {:?}", addr);

	loop {
		match listener.accept().await {
			Ok((stream, addr)) => handle_connection(ctx, conns.clone(), stream, addr).await,
			Err(err) => tracing::error!(?err, "failed to connect websocket"),
		}
	}
}

async fn handle_connection(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
	raw_stream: TcpStream,
	addr: SocketAddr,
) {
	tracing::error!(?addr, "new connection");

	let ctx = ctx.clone();

	tokio::spawn(async move {
		let mut uri = None;
		let ws_stream = tokio_tungstenite::accept_hdr_async(
			raw_stream,
			|req: &tokio_tungstenite::tungstenite::handshake::server::Request, res| {
				// Bootleg way of reading the uri
				uri = Some(req.uri().clone());

				Ok(res)
			},
		)
		.await?;

		// Parse URI
		let uri = unwrap!(uri, "socket has no associated request");
		let url = url::Url::parse(&uri.to_string())?;

		let (tx, mut rx) = ws_stream.split();

		// Get protocol version from last path segment
		let protocol_version = unwrap!(
			unwrap!(url.path_segments(), "invalid url").last(),
			"no path segments"
		)
		.parse::<u16>()?;

		// Read client_id from query parameters
		let client_id = unwrap!(
			url.query_pairs()
				.find_map(|(n, v)| (n == "client_id").then_some(v)),
			"missing `client_id` query parameter"
		);
		let client_id = util::uuid::parse(client_id.as_ref())?;

		// Store connection
		{
			let mut conns = conns.write().await;

			if let Some(old_tx) = conns.insert(client_id, Mutex::new(tx)) {
				tracing::warn!(
					?client_id,
					"client already connected, closing old connection"
				);

				old_tx.lock().await.send(Message::Close(None)).await?;
			}
		}

		// Insert into db and spawn workflow (if not exists)
		upsert_client(&ctx, client_id, addr.ip()).await?;

		// Receive messages from socket
		loop {
			let Some(msg) = rx.next().await else {
				bail!(format!("stream closed {client_id}"));
			};

			match msg? {
				Message::Binary(buf) => {
					let event = ClientEvent::deserialize(protocol_version, &buf)?;

					ctx.signal(event).tag("client_id", client_id).send().await?;
				}
				Message::Close(_) => {
					bail!(format!("socket closed {client_id}"));
				}
				msg => {
					tracing::warn!(?client_id, ?msg, "unexpected message");
				}
			}
		}

		// Only way I could figure out to help the complier infer type
		#[allow(unreachable_code)]
		GlobalResult::Ok(())
	});
}

async fn upsert_client(ctx: &StandaloneCtx, client_id: Uuid, ip: IpAddr) -> GlobalResult<()> {
	// Inserting before creating the workflow prevents a race condition with using select + insert instead
	let inserted = sql_fetch_optional!(
		[ctx, (i64,)]
		"
		INSERT INTO db_pegboard.clients (client_id, ip, create_ts)
		VALUES ($1, $2, $3)
		ON CONFLICT (client_id) DO NOTHING
		RETURNING 1
		",
		client_id,
		ip,
		util::timestamp::now(),
	)
	.await?
	.is_some();

	// If the row was inserted, spawn a new client workflow
	if inserted {
		ctx.workflow(pegboard::workflows::client::Input { client_id })
			.tag("client_id", client_id)
			.dispatch()
			.await?;
	}

	Ok(())
}

async fn signal_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) -> GlobalResult<()> {
	// Listen for commands from client workflows
	let mut sub = ctx
		.subscribe::<Command>(&json!({
			"target": "ws",
		}))
		.await?;

	loop {
		let msg = sub.next().await?;

		{
			let conns = conns.read().await;

			// Send command to socket
			if let Some(write) = conns.get(&msg.client_id) {
				let buf = msg.inner.serialize()?;
				write.lock().await.send(Message::Binary(buf)).await?;
			} else {
				tracing::warn!(?client_id, "received command for client that isn't connected, ignoring");
			}
		}
	}
}
