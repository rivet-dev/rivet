use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use chirp_workflow::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde_json::json;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};

use pegboard::protocol;

struct Connection {
	protocol_version: u16,
	tx: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
}

type Connections = HashMap<Uuid, Connection>;

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
	let port = util::env::var("PORT")?.parse::<u16>()?;
	let addr = SocketAddr::from(([0, 0, 0, 0], port));

	let listener = TcpListener::bind(addr).await?;
	tracing::info!(?port, "server listening");

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
	tracing::info!(?addr, "new connection");

	let ctx = ctx.clone();

	tokio::spawn(async move {
		// TODO: This is an ugly way to improve error visibility
		let setup_res = async move {
			let mut uri = None;
			let ws_stream = tokio_tungstenite::accept_hdr_async(
				raw_stream,
				|req: &tokio_tungstenite::tungstenite::handshake::server::Request, res| {
					// Bootleg way of reading the uri
					uri = Some(req.uri().clone());

					tracing::info!(?addr, ?uri, "handshake");

					Ok(res)
				},
			)
			.await?;

			// Parse URL
			let uri = unwrap!(uri, "socket has no associated request");
			let (protocol_version, client_id) = parse_url(addr, uri)?;

			Ok((ws_stream, protocol_version, client_id))
		};

		// Print error
		let (ws_stream, protocol_version, client_id) = match setup_res.await {
			Ok(x) => x,
			Err(err) => {
				tracing::error!(?addr, "{err}");
				return Err(err);
			}
		};

		// Handle result for cleanup
		match handle_connection_inner(&ctx, conns.clone(), ws_stream, protocol_version, client_id)
			.await
		{
			Err(err) => {
				// Clean up connection
				conns.write().await.remove(&client_id);

				Err(err)
			}
			x => x,
		}
	});
}

async fn handle_connection_inner(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
	ws_stream: WebSocketStream<TcpStream>,
	protocol_version: u16,
	client_id: Uuid,
) -> GlobalResult<()> {
	let (tx, mut rx) = ws_stream.split();

	let tx = Arc::new(Mutex::new(tx));
	let conn = Connection {
		protocol_version,
		tx: tx.clone(),
	};

	// Store connection
	{
		let mut conns = conns.write().await;
		if let Some(old_conn) = conns.insert(client_id, conn) {
			tracing::warn!(
				?client_id,
				"client already connected, closing old connection"
			);

			old_conn.tx.lock().await.send(Message::Close(None)).await?;
		}
	}

	// Insert into db and spawn workflow (if not exists)
	upsert_client(ctx, client_id).await?;

	// Receive messages from socket
	while let Some(msg) = rx.next().await {
		match msg? {
			Message::Binary(buf) => {
				let packet = protocol::ToServer::deserialize(protocol_version, &buf)?;

				// Forward to client wf
				ctx.signal(packet)
					.tag("client_id", client_id)
					.send()
					.await?;
			}
			Message::Ping(_) => {
				update_ping(ctx, client_id).await?;
				tx.lock().await.send(Message::Pong(Vec::new())).await?;
			}
			Message::Close(_) => {
				bail!(format!("socket closed {client_id}"));
			}
			msg => {
				tracing::warn!(?client_id, ?msg, "unexpected message");
			}
		}
	}
	
	bail!(format!("stream closed {client_id}"));

	// Only way I could figure out to help the complier infer type
	#[allow(unreachable_code)]
	GlobalResult::Ok(())
}

async fn upsert_client(ctx: &StandaloneCtx, client_id: Uuid) -> GlobalResult<()> {
	// Inserting before creating the workflow prevents a race condition with using select + insert instead
	let inserted = sql_fetch_optional!(
		[ctx, (i64,)]
		"
		INSERT INTO db_pegboard.clients (client_id, create_ts, last_ping_ts)
		VALUES ($1, $2, $2)
		ON CONFLICT (client_id) DO NOTHING
		RETURNING 1
		",
		client_id,
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

async fn update_ping(ctx: &StandaloneCtx, client_id: Uuid) -> GlobalResult<()> {
	sql_execute!(
		[ctx, (i64,)]
		"
		UPDATE db_pegboard.clients
		SET last_ping_ts = $2
		WHERE client_id = $1
		",
		client_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}

async fn signal_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) -> GlobalResult<()> {
	// Listen for commands from client workflows
	let mut sub = ctx
		.subscribe::<pegboard::workflows::client::ToWs>(&json!({}))
		.await?;

	loop {
		let msg = sub.next().await?;

		{
			let conns = conns.read().await;

			// Send command to socket
			if let Some(conn) = conns.get(&msg.client_id) {
				let buf = msg.inner.serialize(conn.protocol_version)?;
				conn.tx.lock().await.send(Message::Binary(buf)).await?;
			} else {
				tracing::debug!(
					client_id=?msg.client_id,
					"received command for client that isn't connected, ignoring"
				);
			}
		}
	}
}

fn parse_url(addr: SocketAddr, uri: hyper::Uri) -> GlobalResult<(u16, Uuid)> {
	let url = url::Url::parse(&format!("ws://{addr}{uri}"))?;

	// Get protocol version from last path segment
	let last_segment = unwrap!(
		unwrap!(url.path_segments(), "invalid url").last(),
		"no path segments"
	);
	ensure!(last_segment.starts_with('v'), "invalid protocol version");
	let protocol_version = last_segment[1..].parse::<u16>()?;

	// Read client_id from query parameters
	let client_id = unwrap!(
		url.query_pairs()
			.find_map(|(n, v)| (n == "client_id").then_some(v)),
		"missing `client_id` query parameter"
	);
	let client_id = util::uuid::parse(client_id.as_ref())?;

	Ok((protocol_version, client_id))
}
