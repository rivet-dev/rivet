use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{
		Arc,
		atomic::{AtomicU32, Ordering},
	},
	time::Duration,
};

use futures_util::{
	SinkExt, StreamExt,
	stream::{SplitSink, SplitStream},
};
use gas::prelude::Id;
use gas::prelude::*;
use pegboard::ops::runner::update_alloc_idx::{Action, RunnerEligibility};
use pegboard_actor_kv as kv;
use rivet_error::*;
use rivet_runner_protocol::*;
use serde_json::json;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{
	WebSocketStream,
	tungstenite::protocol::{
		Message,
		frame::{CloseFrame, coding::CloseCode},
	},
};
use versioned_data_util::OwnedVersionedData;

const UPDATE_PING_INTERVAL: Duration = Duration::from_secs(3);

#[derive(RivetError, Debug)]
#[error("ws")]
enum WsError {
	#[error(
		"new_runner_connected",
		"New runner connected, closing old connection."
	)]
	NewRunnerConnected,
	#[error("connection_closed", "Normal connection close.")]
	ConnectionClosed,
	#[error(
		"eviction",
		"The websocket has been evicted and should not attempt to reconnect."
	)]
	Eviction,
	#[error(
		"timed_out_waiting_for_init",
		"Timed out waiting for the init packet to be sent."
	)]
	TimedOutWaitingForInit,
	#[error(
		"invalid_initial_packet",
		"The websocket could not process the initial packet.",
		"Invalid initial packet: {0}."
	)]
	InvalidInitialPacket(&'static str),
	#[error(
		"invalid_packet",
		"The websocket could not process the given packet.",
		"Invalid packet: {0}"
	)]
	InvalidPacket(String),
	#[error("invalid_url", "The connection URL is invalid.", "Invalid url: {0}")]
	InvalidUrl(String),
}

struct Connection {
	workflow_id: Id,
	protocol_version: u16,
	tx: Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>,
	last_rtt: AtomicU32,
}

type Connections = HashMap<Id, Arc<Connection>>;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::from_pools(pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"pegboard-runner-ws",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	let conns: Arc<RwLock<Connections>> = Arc::new(RwLock::new(HashMap::new()));

	let host = ctx.config().pegboard().host();
	let port = ctx.config().pegboard().port();
	let addr = SocketAddr::from((host, port));

	let listener = TcpListener::bind(addr).await?;
	tracing::info!(?host, ?port, "runner ws server listening");

	// None of these should ever exit
	//
	// If these do exit, then the `handle_connection` task will run indefinitely and never
	// send/receive anything to runners. Runner workflows will then expire because of their ping,
	// their workflow will complete, and runners will be unusable unless they reconnect.
	tokio::join!(
		socket_thread(&ctx, conns.clone(), listener),
		msg_thread(&ctx, conns.clone()),
		update_ping_thread(&ctx, conns.clone()),
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
		let (ws_stream, uri) = match setup_stream(raw_stream, addr).await {
			Ok(x) => x,
			Err(err) => {
				tracing::warn!(?addr, ?err, "setup stream failed");
				return;
			}
		};
		let (mut tx, mut rx) = ws_stream.split();

		let url_data = match parse_url(addr, uri) {
			Ok(x) => x,
			Err(err) => {
				tracing::warn!(?addr, ?err, "parse url failed");

				let close_frame = err_to_close_frame(WsError::InvalidUrl(err.to_string()).build());

				if let Err(err) = tx.send(Message::Close(Some(close_frame))).await {
					tracing::error!(?addr, ?err, "failed closing socket");
				}

				return;
			}
		};

		let mut tx = Some(tx);

		let (runner_id, conn) = match build_connection(&ctx, &mut tx, &mut rx, url_data).await {
			Ok(res) => res,
			Err(err) => {
				tracing::warn!(?addr, ?err, "failed to build connection");

				if let Some(mut tx) = tx {
					let close_frame = err_to_close_frame(err);

					if let Err(err) = tx.send(Message::Close(Some(close_frame))).await {
						tracing::error!(?addr, ?err, "failed closing socket");
					}
				}

				return;
			}
		};

		// Store connection
		{
			let mut conns = conns.write().await;
			if let Some(old_conn) = conns.insert(runner_id, conn.clone()) {
				tracing::warn!(
					?runner_id,
					"runner already connected, closing old connection"
				);

				let close_frame = err_to_close_frame(WsError::NewRunnerConnected.build());
				let mut tx = old_conn.tx.lock().await;

				if let Err(err) = tx.send(Message::Close(Some(close_frame))).await {
					tracing::error!(?runner_id, ?err, "failed closing old connection");
				}
			}
		}

		let err = if let Err(err) = handle_messages(&ctx, &mut rx, runner_id, &conn).await {
			tracing::warn!(?runner_id, ?err, "failed processing runner messages");

			err
		} else {
			tracing::info!(?runner_id, "runner connection closed");

			WsError::ConnectionClosed.build()
		};

		// Clean up
		{
			conns.write().await.remove(&runner_id);
		}

		// Make runner immediately ineligible when it disconnects
		if let Err(err) = ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input {
				runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id,
					action: Action::ClearIdx,
				}],
			})
			.await
		{
			tracing::error!(?runner_id, ?err, "failed evicting runner from alloc idx");
		}

		let close_frame = err_to_close_frame(err);
		let mut tx = conn.tx.lock().await;
		if let Err(err) = tx.send(Message::Close(Some(close_frame))).await {
			tracing::error!(?runner_id, ?err, "failed closing socket");
		}
	});
}

#[tracing::instrument(skip_all)]
async fn setup_stream(
	raw_stream: TcpStream,
	addr: SocketAddr,
) -> Result<(WebSocketStream<TcpStream>, hyper::Uri)> {
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

	let uri = uri.context("socket has no associated request")?;

	Ok((ws_stream, uri))
}

#[tracing::instrument(skip_all)]
async fn build_connection(
	ctx: &StandaloneCtx,
	tx: &mut Option<SplitSink<WebSocketStream<TcpStream>, Message>>,
	rx: &mut SplitStream<WebSocketStream<TcpStream>>,
	UrlData {
		protocol_version,
		namespace,
	}: UrlData,
) -> Result<(Id, Arc<Connection>)> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input { name: namespace })
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	tracing::debug!("new runner connection");

	// Receive init packet
	let (runner_id, workflow_id) = if let Some(msg) =
		tokio::time::timeout(Duration::from_secs(5), rx.next())
			.await
			.map_err(|_| WsError::TimedOutWaitingForInit.build())?
	{
		let buf = match msg? {
			Message::Binary(buf) => buf,
			Message::Close(_) => return Err(WsError::ConnectionClosed.build()),
			msg => {
				tracing::debug!(?msg, "invalid initial message");
				return Err(WsError::InvalidInitialPacket("must be a binary blob").build());
			}
		};

		let packet = versioned::ToServer::deserialize(&buf, protocol_version)
			.map_err(|err| WsError::InvalidPacket(err.to_string()).build())?
			.try_into()
			.map_err(|err: anyhow::Error| WsError::InvalidPacket(err.to_string()).build())?;

		let (runner_id, workflow_id) = if let protocol::ToServer::Init {
			runner_id,
			name,
			key,
			version,
			total_slots,
			addresses_http,
			addresses_tcp,
			addresses_udp,
			..
		} = &packet
		{
			let runner_id = if let Some(runner_id) = runner_id {
				// IMPORTANT: Before we spawn/get the workflow, we try to update the runner's last ping ts.
				// This ensures if the workflow is currently checking for expiry that it will not expire
				// (because we are about to send signals to it) and if it is already expired (but not
				// completed) we can choose a new runner id.
				let update_ping_res = ctx
					.op(pegboard::ops::runner::update_alloc_idx::Input {
						runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
							runner_id: *runner_id,
							action: Action::UpdatePing { rtt: 0 },
						}],
					})
					.await?;

				if update_ping_res
					.notifications
					.into_iter()
					.next()
					.map(|notif| matches!(notif.eligibility, RunnerEligibility::Expired))
					.unwrap_or_default()
				{
					Id::new_v1(ctx.config().dc_label())
				} else {
					*runner_id
				}
			} else {
				Id::new_v1(ctx.config().dc_label())
			};

			// Spawn a new runner workflow if one doesn't already exist
			let workflow_id = ctx
				.workflow(pegboard::workflows::runner::Input {
					runner_id,
					namespace_id: namespace.namespace_id,
					name: name.clone(),
					key: key.clone(),
					version: version.clone(),
					total_slots: *total_slots,

					addresses_http: addresses_http.clone().unwrap_or_default(),
					addresses_tcp: addresses_tcp.clone().unwrap_or_default(),
					addresses_udp: addresses_udp.clone().unwrap_or_default(),
				})
				.tag("runner_id", runner_id)
				.unique()
				.dispatch()
				.await?;

			(runner_id, workflow_id)
		} else {
			tracing::debug!(?packet, "invalid initial packet");
			return Err(WsError::InvalidInitialPacket("must be `ToServer::Init`").build());
		};

		// Forward to runner wf
		ctx.signal(packet)
			.to_workflow_id(workflow_id)
			.send()
			.await?;

		(runner_id, workflow_id)
	} else {
		return Err(WsError::ConnectionClosed.build());
	};

	let tx = tx.take().context("should exist")?;

	Ok((
		runner_id,
		Arc::new(Connection {
			workflow_id,
			protocol_version,
			tx: Mutex::new(tx),
			last_rtt: AtomicU32::new(0),
		}),
	))
}

async fn handle_messages(
	ctx: &StandaloneCtx,
	rx: &mut SplitStream<WebSocketStream<TcpStream>>,
	runner_id: Id,
	conn: &Connection,
) -> Result<()> {
	// Receive messages from socket
	while let Some(msg) = rx.next().await {
		let buf = match msg? {
			Message::Binary(buf) => buf,
			Message::Ping(_) => continue,
			Message::Close(_) => bail!("socket closed {}", runner_id),
			msg => {
				tracing::warn!(?runner_id, ?msg, "unexpected message");
				continue;
			}
		};

		let packet = versioned::ToServer::deserialize(&buf, conn.protocol_version)?;

		match packet {
			ToServer::ToServerPing(ping) => {
				let rtt = util::timestamp::now().saturating_sub(ping.ts).try_into()?;

				conn.last_rtt.store(rtt, Ordering::Relaxed);
			}
			// Process KV request
			ToServer::ToServerKvRequest(req) => {
				let actor_id = match Id::parse(&req.actor_id) {
					Ok(actor_id) => actor_id,
					Err(err) => {
						let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
							ToClientKvResponse {
								request_id: req.request_id,
								data: KvResponseData::KvErrorResponse(KvErrorResponse {
									message: err.to_string(),
								}),
							},
						));

						let buf = packet.serialize(conn.protocol_version)?;
						conn.tx
							.lock()
							.await
							.send(Message::Binary(buf.into()))
							.await?;

						continue;
					}
				};

				let actors_res = ctx
					.op(pegboard::ops::actor::get_runner::Input {
						actor_ids: vec![actor_id],
					})
					.await?;
				let actor_belongs = actors_res
					.actors
					.first()
					.map(|x| x.runner_id == runner_id)
					.unwrap_or_default();

				// Verify actor belongs to this runner
				if !actor_belongs {
					let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
						ToClientKvResponse {
							request_id: req.request_id,
							data: KvResponseData::KvErrorResponse(KvErrorResponse {
								message: "given actor does not belong to runner".to_string(),
							}),
						},
					));

					let buf = packet.serialize(conn.protocol_version)?;
					conn.tx
						.lock()
						.await
						.send(Message::Binary(buf.into()))
						.await?;

					continue;
				}

				// TODO: Add queue and bg thread for processing kv ops
				// Run kv operation
				match req.data {
					KvRequestData::KvGetRequest(body) => {
						let res = kv::get(&*ctx.udb()?, actor_id, body.keys).await;

						let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
							ToClientKvResponse {
								request_id: req.request_id,
								data: match res {
									Ok((keys, values, metadata)) => {
										KvResponseData::KvGetResponse(KvGetResponse {
											keys,
											values,
											metadata,
										})
									}
									Err(err) => KvResponseData::KvErrorResponse(KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									}),
								},
							},
						));

						let buf = packet.serialize(conn.protocol_version)?;
						conn.tx
							.lock()
							.await
							.send(Message::Binary(buf.into()))
							.await?;
					}
					KvRequestData::KvListRequest(body) => {
						let res = kv::list(
							&*ctx.udb()?,
							actor_id,
							body.query,
							body.reverse.unwrap_or_default(),
							body.limit.map(TryInto::try_into).transpose()?,
						)
						.await;

						let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
							ToClientKvResponse {
								request_id: req.request_id,
								data: match res {
									Ok((keys, values, metadata)) => {
										KvResponseData::KvListResponse(KvListResponse {
											keys,
											values,
											metadata,
										})
									}
									Err(err) => KvResponseData::KvErrorResponse(KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									}),
								},
							},
						));

						let buf = packet.serialize(conn.protocol_version)?;
						conn.tx
							.lock()
							.await
							.send(Message::Binary(buf.into()))
							.await?;
					}
					KvRequestData::KvPutRequest(body) => {
						let res = kv::put(&*ctx.udb()?, actor_id, body.keys, body.values).await;

						let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
							ToClientKvResponse {
								request_id: req.request_id,
								data: match res {
									Ok(()) => KvResponseData::KvPutResponse,
									Err(err) => KvResponseData::KvErrorResponse(KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									}),
								},
							},
						));

						let buf = packet.serialize(conn.protocol_version)?;
						conn.tx
							.lock()
							.await
							.send(Message::Binary(buf.into()))
							.await?;
					}
					KvRequestData::KvDeleteRequest(body) => {
						let res = kv::delete(&*ctx.udb()?, actor_id, body.keys).await;

						let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
							ToClientKvResponse {
								request_id: req.request_id,
								data: match res {
									Ok(()) => KvResponseData::KvDeleteResponse,
									Err(err) => KvResponseData::KvErrorResponse(KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									}),
								},
							},
						));

						let buf = packet.serialize(conn.protocol_version)?;
						conn.tx
							.lock()
							.await
							.send(Message::Binary(buf.into()))
							.await?;
					}
					KvRequestData::KvDropRequest => {
						let res = kv::delete_all(&*ctx.udb()?, actor_id).await;

						let packet = versioned::ToClient::latest(ToClient::ToClientKvResponse(
							ToClientKvResponse {
								request_id: req.request_id,
								data: match res {
									Ok(()) => KvResponseData::KvDropResponse,
									Err(err) => KvResponseData::KvErrorResponse(KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									}),
								},
							},
						));

						let buf = packet.serialize(conn.protocol_version)?;
						conn.tx
							.lock()
							.await
							.send(Message::Binary(buf.into()))
							.await?;
					}
				}
			}
			// Forward to runner wf
			_ => {
				ctx.signal(protocol::ToServer::try_from(packet)?)
					.to_workflow_id(conn.workflow_id)
					.send()
					.await?;
			}
		}
	}

	bail!("stream closed {runner_id}");
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

/// Updates the ping of all runners requesting a ping update at once.
#[tracing::instrument(skip_all)]
async fn update_ping_thread_inner(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
) -> Result<()> {
	loop {
		tokio::time::sleep(UPDATE_PING_INTERVAL).await;

		let runners = {
			let mut conns = conns.write().await;

			// Select all runners that required a ping update
			conns
				.iter_mut()
				.map(|(runner_id, conn)| {
					(
						*runner_id,
						conn.workflow_id,
						conn.last_rtt.load(Ordering::Relaxed),
					)
				})
				.collect::<Vec<_>>()
		};

		if runners.is_empty() {
			continue;
		}

		let mut runners2 = Vec::new();

		// TODO: Parallelize
		// Filter out dead wfs
		for (runner_id, workflow_id, rtt) in runners {
			let Some(wf) = ctx
				.workflow::<pegboard::workflows::runner::Input>(workflow_id)
				.get()
				.await?
			else {
				tracing::error!(?runner_id, "workflow does not exist");
				continue;
			};

			// Only update ping if the workflow is not dead
			if wf.has_wake_condition {
				runners2.push(pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id,
					action: Action::UpdatePing { rtt },
				});
			}
		}

		if runners2.is_empty() {
			continue;
		}

		let res = ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input { runners: runners2 })
			.await?;

		for notif in res.notifications {
			if let RunnerEligibility::ReEligible = notif.eligibility {
				tracing::debug!(runner_id=?notif.runner_id, "runner has become eligible again");

				ctx.signal(pegboard::workflows::runner::CheckQueue {})
					.to_workflow_id(notif.workflow_id)
					.send()
					.await?;
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
async fn msg_thread_inner(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) -> Result<()> {
	// Listen for commands from runner workflows
	let mut sub = ctx
		.subscribe::<pegboard::workflows::runner::ToWs>(&json!({}))
		.await?;
	let mut close_sub = ctx
		.subscribe::<pegboard::workflows::runner::CloseWs>(&json!({}))
		.await?;

	loop {
		tokio::select! {
			msg = sub.next() => {
				let msg = msg?.into_body();

				{
					let conns = conns.read().await;

					// Send command to socket
					if let Some(conn) = conns.get(&msg.runner_id) {
						let buf = versioned::ToClient::serialize(
							protocol::ToClient::from(msg.inner).try_into()?,
							conn.protocol_version
						)?;
						conn.tx.lock().await.send(Message::Binary(buf.into())).await?;
					} else {
						tracing::debug!(
							runner_id=?msg.runner_id,
							"received command for runner that isn't connected, ignoring"
						);
					}
				}
			}
			msg = close_sub.next() => {
				let msg = msg?;

				{
					let conns = conns.read().await;

					// Close socket
					if let Some(conn) = conns.get(&msg.runner_id) {
						tracing::info!(runner_id = ?msg.runner_id, "received close ws event, closing socket");

						let close_frame = err_to_close_frame(WsError::Eviction.build());
						conn.tx.lock().await.send(Message::Close(Some(close_frame))).await?;
					} else {
						tracing::debug!(
							runner_id=?msg.runner_id,
							"received close command for runner that isn't connected, ignoring"
						);
					}
				}
			}
		}
	}
}

#[derive(Clone)]
struct UrlData {
	protocol_version: u16,
	namespace: String,
}

fn parse_url(addr: SocketAddr, uri: hyper::Uri) -> Result<UrlData> {
	let url = url::Url::parse(&format!("ws://{addr}{uri}"))?;

	// Get protocol version from last path segment
	let last_segment = url
		.path_segments()
		.context("invalid url")?
		.last()
		.context("no path segments")?;
	ensure!(last_segment.starts_with('v'), "invalid protocol version");
	let protocol_version = last_segment[1..]
		.parse::<u16>()
		.context("invalid protocol version")?;

	// Read namespace from query parameters
	let namespace = url
		.query_pairs()
		.find_map(|(n, v)| (n == "namespace").then_some(v))
		.context("missing `namespace` query parameter")?
		.to_string();

	Ok(UrlData {
		protocol_version,
		namespace,
	})
}

fn err_to_close_frame(err: anyhow::Error) -> CloseFrame {
	let rivet_err = err
		.chain()
		.find_map(|x| x.downcast_ref::<RivetError>())
		.cloned()
		.unwrap_or_else(|| RivetError::from(&INTERNAL_ERROR));

	let code = match (rivet_err.group(), rivet_err.code()) {
		("ws", "connection_closed") => CloseCode::Normal,
		_ => CloseCode::Error,
	};

	// NOTE: reason cannot be more than 123 bytes as per the WS protocol
	let reason = util::safe_slice(
		&format!("{}.{}", rivet_err.group(), rivet_err.code()),
		0,
		123,
	)
	.into();

	CloseFrame { code, reason }
}
