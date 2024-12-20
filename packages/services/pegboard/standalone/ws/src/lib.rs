use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::Duration,
};

use chirp_workflow::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde_json::json;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{
	tungstenite::protocol::{
		frame::{coding::CloseCode, CloseFrame},
		Message,
	},
	WebSocketStream,
};

use pegboard::protocol;

const UPDATE_PING_INTERVAL: Duration = Duration::from_secs(3);
const CHECK_WORKFLOW_INTERVAL: Duration = Duration::from_secs(15);

struct Connection {
	protocol_version: u16,
	tx: Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>,
	update_ping: AtomicBool,
}

type Connections = HashMap<Uuid, Arc<Connection>>;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	run_from_env(config.clone(), pools.clone()).await
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-ws");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-ws",
	)
	.await?;

	let conns: Arc<RwLock<Connections>> = Arc::new(RwLock::new(HashMap::new()));

	let host = ctx.config().server()?.rivet.pegboard.host();
	let port = ctx.config().server()?.rivet.pegboard.port();
	let addr = SocketAddr::from((host, port));

	let listener = TcpListener::bind(addr).await?;
	tracing::info!(?port, ?port, "pegboard ws server listening");

	// None of these should ever exit
	//
	// If these do exit, then the `handle_connection` task will run indefinitely and never
	// send/receive anything to clients. Client workflows will then expire because of their ping,
	// their workflow will complete, and clients will be unusable unless they reconnect.
	tokio::join!(
		socket_thread(&ctx, conns.clone(), listener),
		msg_thread(&ctx, conns.clone()),
		update_ping_thread(&ctx, conns.clone()),
		check_workflows_thread(&ctx, conns.clone()),
	);

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn socket_thread(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
	listener: TcpListener,
) {
	loop {
		match listener.accept().await {
			Ok((stream, addr)) => handle_connection(ctx, conns.clone(), stream, addr).await,
			Err(err) => tracing::error!(?err, "failed to connect websocket"),
		}
	}
}

#[tracing::instrument(skip_all)]
async fn handle_connection(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
	raw_stream: TcpStream,
	addr: SocketAddr,
) {
	tracing::debug!(?addr, "new connection");

	let ctx = ctx.clone();

	tokio::spawn(async move {
		let (ws_stream, url_data) = match setup_connection(raw_stream, addr).await {
			Ok(x) => x,
			Err(err) => {
				tracing::error!(?addr, "{err}");
				return;
			}
		};

		if let Err(err) = handle_connection_inner(&ctx, conns.clone(), ws_stream, url_data).await {
			tracing::error!(?addr, "{err}");
		}

		// Clean up
		let conn = conns.write().await.remove(&url_data.client_id);
		if let Some(conn) = conn {
			let close_frame = CloseFrame {
				code: CloseCode::Normal,
				reason: "handle_connection_inner event loop closed".into(),
			};
			if let Err(err) = conn
				.tx
				.lock()
				.await
				.send(Message::Close(Some(close_frame)))
				.await
			{
				tracing::error!(?addr, "failed closing socket: {err}");
			}
		}
	});
}

#[tracing::instrument(skip_all)]
async fn setup_connection(
	raw_stream: TcpStream,
	addr: SocketAddr,
) -> GlobalResult<(WebSocketStream<TcpStream>, UrlData)> {
	let mut uri = None;
	let ws_stream = tokio_tungstenite::accept_hdr_async(
		raw_stream,
		|req: &tokio_tungstenite::tungstenite::handshake::server::Request, res| {
			// Bootleg way of reading the uri
			uri = Some(req.uri().clone());

			tracing::debug!(?addr, ?uri, "handshake");

			Ok(res)
		},
	)
	.await?;

	// Parse URL
	let uri = unwrap!(uri, "socket has no associated request");
	let url_data = parse_url(addr, uri)?;

	Ok((ws_stream, url_data))
}

#[tracing::instrument(skip_all)]
async fn handle_connection_inner(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
	ws_stream: WebSocketStream<TcpStream>,
	UrlData {
		protocol_version,
		client_id,
		datacenter_id,
		flavor,
	}: UrlData,
) -> GlobalResult<()> {
	let (tx, mut rx) = ws_stream.split();

	let conn = Arc::new(Connection {
		protocol_version,
		tx: Mutex::new(tx),
		update_ping: AtomicBool::new(false),
	});

	// Store connection
	{
		let mut conns = conns.write().await;
		if let Some(old_conn) = conns.insert(client_id, conn.clone()) {
			tracing::warn!(
				?client_id,
				"client already connected, closing old connection"
			);

			let close_frame = CloseFrame {
				code: CloseCode::Normal,
				reason: "client already connected, closing old connection".into(),
			};
			old_conn
				.tx
				.lock()
				.await
				.send(Message::Close(Some(close_frame)))
				.await?;
		}
	}

	// Only create the client after receiving the init packet to prevent a race condition
	if let Some(msg) = rx.next().await {
		match msg? {
			Message::Binary(buf) => {
				let packet = protocol::ToServer::deserialize(protocol_version, &buf)?;

				if let protocol::ToServer::Init { .. } = &packet {
					// Insert into db and spawn workflow (if not exists)
					upsert_client(ctx, client_id, datacenter_id, flavor).await?;
				} else {
					bail!("unexpected initial packet: {packet:?}");
				}

				// Forward to client wf
				ctx.signal(packet)
					.tag("client_id", client_id)
					.send()
					.await?;
			}
			Message::Close(_) => {
				bail!("socket closed {client_id}");
			}
			msg => bail!("unexpected initial message: {msg:?}"),
		}
	}

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
				conn.update_ping.store(true, Ordering::Relaxed);
			}
			Message::Close(_) => {
				bail!("socket closed {client_id}");
			}
			msg => tracing::warn!(?client_id, ?msg, "unexpected message"),
		}
	}

	bail!("stream closed {client_id}");

	// Only way I could figure out to help the complier infer type
	#[allow(unreachable_code)]
	GlobalResult::Ok(())
}

#[tracing::instrument(skip_all)]
async fn upsert_client(
	ctx: &StandaloneCtx,
	client_id: Uuid,
	datacenter_id: Uuid,
	flavor: protocol::ClientFlavor,
) -> GlobalResult<()> {
	// Inserting before creating the workflow prevents a race condition with using select + insert instead
	let (_, (workflow_exists,)) = tokio::try_join!(
		sql_execute!(
			[ctx]
			"
			INSERT INTO db_pegboard.clients (
				client_id, datacenter_id, flavor, create_ts, last_ping_ts
			)
			VALUES ($1, $2, $3, $4, $4)
			ON CONFLICT (client_id)
				DO UPDATE
				SET delete_ts = NULL
			",
			client_id,
			datacenter_id,
			flavor as i32,
			util::timestamp::now(),
		),
		// HACK(RVT-4458): Check if workflow already exists and spawn a new one if not
		sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS (
				SELECT 1
				FROM db_workflow.workflows
				WHERE
					tags->>'client_id' = $1::text AND
					workflow_name = 'pegboard_client' AND
					output IS NULL
			)
			",
			client_id,
		),
	)?;

	if !workflow_exists {
		tracing::info!(
			?client_id,
			?datacenter_id,
			?flavor,
			"creating client workflow"
		);
		ctx.workflow(pegboard::workflows::client::Input { client_id })
			.tag("client_id", client_id)
			.dispatch()
			.await?;
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn update_ping_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) {
	loop {
		match update_ping_thread_inner(ctx, conns.clone()).await {
			Ok(_) => {
				tracing::warn!("update ping thread thread exited early");
			}
			Err(err) => {
				tracing::error!(?err, "update ping thread error");
			}
		}

		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
	}
}

/// Updates the ping of all clients requesting a ping update at once.
#[tracing::instrument(skip_all)]
async fn update_ping_thread_inner(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
) -> GlobalResult<()> {
	loop {
		tokio::time::sleep(UPDATE_PING_INTERVAL).await;

		let client_ids = {
			let conns = conns.read().await;

			// Select all clients that required a ping update
			conns
				.iter()
				.filter_map(|(client_id, conn)| {
					conn.update_ping
						.swap(false, Ordering::Relaxed)
						.then_some(*client_id)
				})
				.collect::<Vec<_>>()
		};

		if client_ids.is_empty() {
			continue;
		}

		sql_execute!(
			[ctx]
			"
			UPDATE db_pegboard.clients
			SET last_ping_ts = $2
			WHERE client_id = ANY($1)
			",
			client_ids,
			util::timestamp::now(),
		)
		.await?;
	}
}

#[tracing::instrument(skip_all)]
async fn check_workflows_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) {
	loop {
		match check_workflows_thread_inner(ctx, conns.clone()).await {
			Ok(_) => {
				tracing::warn!("check workflows thread thread exited early");
			}
			Err(err) => {
				tracing::error!(?err, "check workflows thread error");
			}
		}

		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
	}
}

// HACK(RVT-4458):
/// Validates that workflows exist for the curation of the client running.
#[tracing::instrument(skip_all)]
async fn check_workflows_thread_inner(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
) -> GlobalResult<()> {
	loop {
		tokio::time::sleep(CHECK_WORKFLOW_INTERVAL).await;

		let client_ids = {
			let conns = conns.read().await;

			// Select all clients that required a ping update
			conns
				.iter()
				.map(|(client_id, _)| *client_id)
				.collect::<Vec<Uuid>>()
		};

		if client_ids.is_empty() {
			continue;
		}

		// Find clients that do not have a running workflow
		let clients_without_workflows = sql_fetch_all!(
			[ctx, (Uuid,)]
			"
			WITH client_ids AS (
			   SELECT unnest($1::uuid[]) as client_id
			)
			SELECT c.client_id
			FROM client_ids c
			LEFT JOIN db_workflow.workflows w ON
				w.tags->>'client_id' = c.client_id::text AND
				w.workflow_name = 'pegboard_client' AND
				w.output IS NULL
			WHERE w.workflow_id IS NULL
			",
			client_ids,
		)
		.await?;

		// Disconnect clients without running workflow
		if !clients_without_workflows.is_empty() {
			let conns = conns.read().await;

			for (client_id,) in clients_without_workflows {
				if let Some(conn) = conns.get(&client_id) {
					tracing::warn!(
						?client_id,
						"client does not have running workflow, closing socket"
					);

					let close_frame = CloseFrame {
						code: CloseCode::Normal,
						reason: "client does not have running workflow".into(),
					};
					conn.tx
						.lock()
						.await
						.send(Message::Close(Some(close_frame)))
						.await?;
				}
			}
		}
	}
}

#[tracing::instrument(skip_all)]
async fn msg_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) {
	loop {
		match msg_thread_inner(ctx, conns.clone()).await {
			Ok(_) => {
				tracing::warn!("msg thread exited early");
			}
			Err(err) => {
				tracing::error!(?err, "msg thread error");
			}
		}

		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
	}
}

#[tracing::instrument(skip_all)]
async fn msg_thread_inner(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
) -> GlobalResult<()> {
	// Listen for commands from client workflows
	let mut sub = ctx
		.subscribe::<pegboard::workflows::client::ToWs>(&json!({}))
		.await?;
	let mut close_sub = ctx
		.subscribe::<pegboard::workflows::client::CloseWs>(&json!({}))
		.await?;

	loop {
		tokio::select! {
			msg = sub.next() => {
				let msg = msg?;

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
			msg = close_sub.next() => {
				let msg = msg?;

				{
					let conns = conns.read().await;

					// Close socket
					if let Some(conn) = conns.get(&msg.client_id) {
						tracing::info!(client_id = ?msg.client_id, "received close ws event, closing socket");

						let close_frame = CloseFrame {
							code: CloseCode::Normal,
							reason: "received close ws event".into(),
						};
						conn.tx.lock().await.send(Message::Close(Some(close_frame))).await?;
					} else {
						tracing::debug!(
							client_id=?msg.client_id,
							"received close command for client that isn't connected, ignoring"
						);
					}
				}
			}
		}
	}
}

#[derive(Clone, Copy)]
struct UrlData {
	protocol_version: u16,
	client_id: Uuid,
	datacenter_id: Uuid,
	flavor: protocol::ClientFlavor,
}

fn parse_url(addr: SocketAddr, uri: hyper::Uri) -> GlobalResult<UrlData> {
	let url = url::Url::parse(&format!("ws://{addr}{uri}"))?;

	// Get protocol version from last path segment
	let last_segment = unwrap!(
		unwrap!(url.path_segments(), "invalid url").last(),
		"no path segments"
	);
	ensure!(last_segment.starts_with('v'), "invalid protocol version");
	let protocol_version = last_segment[1..].parse::<u16>()?;

	// Read client_id and datacenter_id from query parameters
	let client_id = unwrap!(
		url.query_pairs()
			.find_map(|(n, v)| (n == "client_id").then_some(v)),
		"missing `client_id` query parameter"
	);
	let client_id = util::uuid::parse(client_id.as_ref())?;

	let datacenter_id = unwrap!(
		url.query_pairs()
			.find_map(|(n, v)| (n == "datacenter_id").then_some(v)),
		"missing `datacenter_id` query parameter"
	);
	let datacenter_id = util::uuid::parse(datacenter_id.as_ref())?;

	let flavor = unwrap!(
		url.query_pairs()
			.find_map(|(n, v)| (n == "flavor").then_some(v)),
		"missing `flavor` query parameter"
	);
	let flavor = flavor.parse()?;

	Ok(UrlData {
		protocol_version,
		client_id,
		datacenter_id,
		flavor,
	})
}
