use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::*;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use gas::prelude::*;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming as BodyIncoming};
use hyper::{Request, Response, StatusCode};
use hyper_tungstenite::tungstenite::Utf8Bytes as WsUtf8Bytes;
use hyper_tungstenite::tungstenite::protocol::frame::CloseFrame as WsCloseFrame;
use hyper_tungstenite::tungstenite::protocol::frame::coding::CloseCode as WsCloseCode;
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message as WsMessage};
use pegboard::pubsub_subjects::{
	TunnelHttpResponseSubject, TunnelHttpRunnerSubject, TunnelHttpWebSocketSubject,
};
use rivet_guard_core::custom_serve::CustomServeTrait;
use rivet_guard_core::proxy_service::ResponseBody;
use rivet_guard_core::request_context::RequestContext;
use rivet_pools::Pools;
use rivet_tunnel_protocol::{MessageBody, TunnelMessage, versioned};
use rivet_util::Id;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio_tungstenite::accept_async;
use tracing::{error, info};
use universalpubsub::pubsub::NextOutput;

const UPS_REQ_TIMEOUT: Duration = Duration::from_secs(2);

struct RunnerConnection {
	_runner_id: Id,
	_port_name: String,
}

type Connections = Arc<RwLock<HashMap<Id, Arc<RunnerConnection>>>>;

pub struct PegboardTunnelCustomServe {
	ctx: StandaloneCtx,
	connections: Connections,
}

impl PegboardTunnelCustomServe {
	pub async fn new(ctx: StandaloneCtx) -> Result<Self> {
		let connections = Arc::new(RwLock::new(HashMap::new()));

		Ok(Self { ctx, connections })
	}
}

#[async_trait]
impl CustomServeTrait for PegboardTunnelCustomServe {
	async fn handle_request(
		&self,
		_req: hyper::Request<http_body_util::Full<hyper::body::Bytes>>,
		_request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Pegboard tunnel doesn't handle regular HTTP requests
		// Return a simple status response
		let response = Response::builder()
			.status(StatusCode::OK)
			.header("Content-Type", "text/plain")
			.body(ResponseBody::Full(Full::new(Bytes::from(
				"pegboard-tunnel WebSocket endpoint",
			))))?;

		Ok(response)
	}

	async fn handle_websocket(
		&self,
		client_ws: HyperWebsocket,
		_headers: &hyper::HeaderMap,
		path: &str,
		_request_context: &mut RequestContext,
	) -> std::result::Result<(), (HyperWebsocket, anyhow::Error)> {
		let ups = match self.ctx.pools().ups() {
			Result::Ok(u) => u,
			Err(e) => return Err((client_ws, e.into())),
		};
		let connections = self.connections.clone();

		// Extract runner_id from query parameters
		let runner_id = if let Some(query_start) = path.find('?') {
			let query_string = &path[query_start + 1..];
			let params: Vec<_> = query_string.split('&').collect();
			let mut found_runner_id = None;

			for param in params {
				if let Some(eq_pos) = param.find('=') {
					let (key, value) = param.split_at(eq_pos);
					if key == "runner_id" {
						// Remove the leading '=' from value
						let id_str = &value[1..];
						found_runner_id = id_str.parse::<Id>().ok();
						break;
					}
				}
			}

			found_runner_id.unwrap_or(Id::nil())
		} else {
			Id::nil()
		};

		let port_name = "main".to_string(); // Use "main" as default port name

		info!(
			?runner_id,
			?port_name,
			?path,
			"tunnel WebSocket connection established"
		);

		let connection_id = Id::nil();

		// Subscribe to NATS topic for this runner before accepting the client websocket so
		// that failures can be retried by the proxy.
		let topic = TunnelHttpRunnerSubject::new(runner_id, &port_name).to_string();
		info!(?topic, ?runner_id, "subscribing to NATS topic");

		let mut sub = match ups.subscribe(&topic).await {
			Result::Ok(s) => s,
			Err(e) => return Err((client_ws, e.into())),
		};

		let ws_stream = match client_ws.await {
			Result::Ok(ws) => ws,
			Err(e) => {
				// Handshake already in progress; cannot retry safely here
				error!(error=?e, "client websocket await failed");
				return std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(());
			}
		};

		// Split WebSocket stream into read and write halves
		let (ws_write, mut ws_read) = ws_stream.split();
		let ws_write = Arc::new(tokio::sync::Mutex::new(ws_write));

		// Store connection
		let connection = Arc::new(RunnerConnection {
			_runner_id: runner_id,
			_port_name: port_name.clone(),
		});

		connections
			.write()
			.await
			.insert(connection_id, connection.clone());

		// Handle bidirectional message forwarding
		let ws_write_nats_to_ws = ws_write.clone();
		let connections_clone = connections.clone();
		let ups_clone = ups.clone();

		// Task for forwarding NATS -> WebSocket
		let nats_to_ws = tokio::spawn(async move {
			info!("starting NATS to WebSocket forwarding task");
			while let ::std::result::Result::Ok(NextOutput::Message(msg)) = sub.next().await {
				// Ack message
				//match msg.reply(&[]).await {
				//	Result::Ok(_) => {}
				//	Err(err) => {
				//		tracing::warn!(?err, "failed to ack gateway request response message")
				//	}
				//};

				info!(
					payload_len = msg.payload.len(),
					"received message from NATS, forwarding to WebSocket"
				);
				// Forward raw message to WebSocket
				let ws_msg = WsMessage::Binary(msg.payload.to_vec().into());
				{
					let mut stream = ws_write_nats_to_ws.lock().await;
					if let Err(e) = stream.send(ws_msg).await {
						error!(?e, "failed to send message to WebSocket");
						break;
					}
				}
			}
			info!("NATS to WebSocket forwarding task ended");
		});

		// Task for forwarding WebSocket -> NATS
		let ws_write_ws_to_nats = ws_write.clone();
		let ws_to_nats = tokio::spawn(async move {
			info!("starting WebSocket to NATS forwarding task");
			while let Some(msg) = ws_read.next().await {
				match msg {
					::std::result::Result::Ok(WsMessage::Binary(data)) => {
						info!(
							data_len = data.len(),
							"received binary message from WebSocket"
						);
						// Parse the tunnel message to extract request_id
						match versioned::TunnelMessage::deserialize(&data) {
							::std::result::Result::Ok(tunnel_msg) => {
								// Handle different message types
								match &tunnel_msg.body {
									MessageBody::ToClientResponseStart(resp) => {
										info!(?resp.request_id, status = resp.status, "forwarding HTTP response to NATS");
										let response_topic = TunnelHttpResponseSubject::new(
											runner_id,
											&port_name,
											resp.request_id,
										)
										.to_string();

										info!(?response_topic, ?resp.request_id, "publishing HTTP response to NATS");

										if let Err(e) =
											ups_clone.publish(&response_topic, &data.to_vec()).await
										{
											let err_any: anyhow::Error = e.into();
											if is_tunnel_closed_error(&err_any) {
												info!(
													"tunnel closed while publishing HTTP response; closing client websocket"
												);
												// Close client websocket with reason
												send_tunnel_closed_close_hyper(
													&ws_write_ws_to_nats,
												)
												.await;
												break;
											} else {
												error!(?err_any, ?resp.request_id, "failed to publish HTTP response to NATS");
											}
										} else {
											info!(?resp.request_id, "successfully published HTTP response to NATS");
										}
									}
									MessageBody::ToClientWebSocketMessage(ws_msg) => {
										info!(?ws_msg.web_socket_id, "forwarding WebSocket message to NATS");
										// Forward WebSocket messages to the topic that pegboard-gateway subscribes to
										let ws_topic = TunnelHttpWebSocketSubject::new(
											runner_id,
											&port_name,
											ws_msg.web_socket_id,
										)
										.to_string();

										info!(?ws_topic, ?ws_msg.web_socket_id, "publishing WebSocket message to NATS");

										if let Err(e) =
											ups_clone.publish(&ws_topic, &data.to_vec()).await
										{
											let err_any: anyhow::Error = e.into();
											if is_tunnel_closed_error(&err_any) {
												info!(
													"tunnel closed while publishing WebSocket message; closing client websocket"
												);
												// Close client websocket with reason
												send_tunnel_closed_close_hyper(
													&ws_write_ws_to_nats,
												)
												.await;
												break;
											} else {
												error!(?err_any, ?ws_msg.web_socket_id, "failed to publish WebSocket message to NATS");
											}
										} else {
											info!(?ws_msg.web_socket_id, "successfully published WebSocket message to NATS");
										}
									}
									MessageBody::ToClientWebSocketOpen(ws_open) => {
										info!(?ws_open.web_socket_id, "forwarding WebSocket open to NATS");
										let ws_topic = TunnelHttpWebSocketSubject::new(
											runner_id,
											&port_name,
											ws_open.web_socket_id,
										)
										.to_string();

										if let Err(e) =
											ups_clone.publish(&ws_topic, &data.to_vec()).await
										{
											let err_any: anyhow::Error = e.into();
											if is_tunnel_closed_error(&err_any) {
												info!(
													"tunnel closed while publishing WebSocket open; closing client websocket"
												);
												// Close client websocket with reason
												send_tunnel_closed_close_hyper(
													&ws_write_ws_to_nats,
												)
												.await;
												break;
											} else {
												error!(?err_any, ?ws_open.web_socket_id, "failed to publish WebSocket open to NATS");
											}
										} else {
											info!(?ws_open.web_socket_id, "successfully published WebSocket open to NATS");
										}
									}
									MessageBody::ToClientWebSocketClose(ws_close) => {
										info!(?ws_close.web_socket_id, "forwarding WebSocket close to NATS");
										let ws_topic = TunnelHttpWebSocketSubject::new(
											runner_id,
											&port_name,
											ws_close.web_socket_id,
										)
										.to_string();

										if let Err(e) =
											ups_clone.publish(&ws_topic, &data.to_vec()).await
										{
											let err_any: anyhow::Error = e.into();
											if is_tunnel_closed_error(&err_any) {
												info!(
													"tunnel closed while publishing WebSocket close; closing client websocket"
												);
												// Close client websocket with reason
												send_tunnel_closed_close_hyper(
													&ws_write_ws_to_nats,
												)
												.await;
												break;
											} else {
												error!(?err_any, ?ws_close.web_socket_id, "failed to publish WebSocket close to NATS");
											}
										} else {
											info!(?ws_close.web_socket_id, "successfully published WebSocket close to NATS");
										}
									}
									_ => {
										// For other message types, we might not need to forward to NATS
										info!(
											"Received non-response message from WebSocket, skipping NATS forward"
										);
										continue;
									}
								}
							}
							::std::result::Result::Err(e) => {
								error!(?e, "failed to deserialize tunnel message from WebSocket");
							}
						}
					}
					::std::result::Result::Ok(WsMessage::Close(_)) => {
						info!(?runner_id, "WebSocket closed");
						break;
					}
					::std::result::Result::Ok(_) => {
						// Ignore other message types
					}
					Err(e) => {
						error!(?e, "WebSocket error");
						break;
					}
				}
			}
			info!("WebSocket to NATS forwarding task ended");

			// Clean up connection
			connections_clone.write().await.remove(&connection_id);
		});

		// Wait for either task to complete
		tokio::select! {
			_ = nats_to_ws => {
				info!("NATS to WebSocket task completed");
			}
			_ = ws_to_nats => {
				info!("WebSocket to NATS task completed");
			}
		}

		// Clean up
		connections.write().await.remove(&connection_id);
		info!(?runner_id, "connection closed");

		std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(())
	}
}

// Keep the old start function for backward compatibility in tests
pub async fn start(config: rivet_config::Config, pools: Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		gas::db::DatabaseKv::from_pools(pools.clone()).await?,
		config.clone(),
		pools.clone(),
		cache,
		"pegboard-tunnel",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	main_loop(ctx).await
}

async fn main_loop(ctx: gas::prelude::StandaloneCtx) -> Result<()> {
	let connections: Connections = Arc::new(RwLock::new(HashMap::new()));

	// Start WebSocket server
	// Use pegboard config since pegboard_tunnel doesn't exist
	let server_addr = SocketAddr::new(
		ctx.config().pegboard().host(),
		ctx.config().pegboard().port(),
	);

	info!(?server_addr, "starting pegboard-tunnel");

	let listener = TcpListener::bind(&server_addr).await?;

	// Accept connections
	loop {
		let (tcp_stream, addr) = listener.accept().await?;
		let connections = connections.clone();
		let ctx = ctx.clone();

		tokio::spawn(async move {
			if let Err(e) = handle_connection(ctx, tcp_stream, addr, connections).await {
				error!(?e, ?addr, "connection handler error");
			}
		});
	}
}

async fn handle_connection(
	ctx: gas::prelude::StandaloneCtx,
	tcp_stream: tokio::net::TcpStream,
	addr: std::net::SocketAddr,
	connections: Connections,
) -> Result<()> {
	info!(?addr, "new connection");

	// Parse WebSocket upgrade request
	let ws_stream = accept_async(tcp_stream).await?;

	// For now, we'll expect the runner to send an initial message with its ID
	// In production, this would be parsed from the URL path or headers
	let runner_id = rivet_util::Id::nil(); // Placeholder - should be extracted from connection
	let port_name = "default".to_string(); // Placeholder - should be extracted

	let connection_id = rivet_util::Id::nil();

	// Subscribe to NATS topic for this runner using raw NATS client
	let topic = TunnelHttpRunnerSubject::new(runner_id, &port_name).to_string();
	info!(?topic, ?runner_id, "subscribing to NATS topic");

	// Get UPS (UniversalPubSub) client
	let ups = ctx.pools().ups()?;
	let mut sub = ups.subscribe(&topic).await?;

	// Split WebSocket stream into read and write halves
	let (ws_write, mut ws_read) = ws_stream.split();
	let ws_write = Arc::new(Mutex::new(ws_write));

	// Store connection
	let connection = Arc::new(RunnerConnection {
		_runner_id: runner_id,
		_port_name: port_name.clone(),
	});

	connections
		.write()
		.await
		.insert(connection_id, connection.clone());

	// Handle bidirectional message forwarding
	let ws_write_clone = ws_write.clone();
	let connections_clone = connections.clone();
	let ups_clone = ups.clone();

	// Task for forwarding NATS -> WebSocket
	let nats_to_ws = tokio::spawn(async move {
		while let ::std::result::Result::Ok(NextOutput::Message(msg)) = sub.next().await {
			// Ack message
			//match msg.reply(&[]).await {
			//	Result::Ok(_) => {}
			//	Err(err) => {
			//		tracing::warn!(?err, "failed to ack gateway request response message")
			//	}
			//};

			// Forward raw message to WebSocket
			let ws_msg =
				tokio_tungstenite::tungstenite::Message::Binary(msg.payload.to_vec().into());
			{
				let mut stream = ws_write_clone.lock().await;
				if let Err(e) = stream.send(ws_msg).await {
					error!(?e, "failed to send message to WebSocket");
					break;
				}
			}
		}
	});

	// Task for forwarding WebSocket -> NATS
	let ws_write_ws_to_nats = ws_write.clone();
	let ws_to_nats = tokio::spawn(async move {
		while let Some(msg) = ws_read.next().await {
			match msg {
				::std::result::Result::Ok(tokio_tungstenite::tungstenite::Message::Binary(
					data,
				)) => {
					// Parse the tunnel message to extract request_id
					match versioned::TunnelMessage::deserialize(&data) {
						::std::result::Result::Ok(tunnel_msg) => {
							// Handle different message types
							match &tunnel_msg.body {
								MessageBody::ToClientResponseStart(resp) => {
									let response_topic = TunnelHttpResponseSubject::new(
										runner_id,
										&port_name,
										resp.request_id,
									)
									.to_string();

									if let Err(e) =
										ups_clone.publish(&response_topic, &data.to_vec()).await
									{
										let err_any: anyhow::Error = e.into();
										if is_tunnel_closed_error(&err_any) {
											info!(
												"tunnel closed while publishing HTTP response; closing client websocket"
											);
											// Close client websocket with reason
											send_tunnel_closed_close_tokio(&ws_write_ws_to_nats)
												.await;
											break;
										} else {
											error!(?err_any, ?resp.request_id, "failed to publish HTTP response to NATS");
										}
									}
								}
								MessageBody::ToClientWebSocketMessage(ws_msg) => {
									let ws_topic = TunnelHttpWebSocketSubject::new(
										runner_id,
										&port_name,
										ws_msg.web_socket_id,
									)
									.to_string();

									if let Err(e) =
										ups_clone.publish(&ws_topic, &data.to_vec()).await
									{
										let err_any: anyhow::Error = e.into();
										if is_tunnel_closed_error(&err_any) {
											info!(
												"tunnel closed while publishing WebSocket message; closing client websocket"
											);
											// Close client websocket with reason
											send_tunnel_closed_close_tokio(&ws_write_ws_to_nats)
												.await;
											break;
										} else {
											error!(?err_any, ?ws_msg.web_socket_id, "failed to publish WebSocket message to NATS");
										}
									}
								}
								MessageBody::ToClientWebSocketOpen(ws_open) => {
									let ws_topic = TunnelHttpWebSocketSubject::new(
										runner_id,
										&port_name,
										ws_open.web_socket_id,
									)
									.to_string();

									if let Err(e) =
										ups_clone.publish(&ws_topic, &data.to_vec()).await
									{
										let err_any: anyhow::Error = e.into();
										if is_tunnel_closed_error(&err_any) {
											info!(
												"tunnel closed while publishing WebSocket open; closing client websocket"
											);
											// Close client websocket with reason
											send_tunnel_closed_close_tokio(&ws_write_ws_to_nats)
												.await;
											break;
										} else {
											error!(?err_any, ?ws_open.web_socket_id, "failed to publish WebSocket open to NATS");
										}
									}
								}
								MessageBody::ToClientWebSocketClose(ws_close) => {
									let ws_topic = TunnelHttpWebSocketSubject::new(
										runner_id,
										&port_name,
										ws_close.web_socket_id,
									)
									.to_string();

									if let Err(e) =
										ups_clone.publish(&ws_topic, &data.to_vec()).await
									{
										let err_any: anyhow::Error = e.into();
										if is_tunnel_closed_error(&err_any) {
											info!(
												"tunnel closed while publishing WebSocket close; closing client websocket"
											);
											// Close client websocket with reason
											send_tunnel_closed_close_tokio(&ws_write_ws_to_nats)
												.await;
											break;
										} else {
											error!(?err_any, ?ws_close.web_socket_id, "failed to publish WebSocket close to NATS");
										}
									}
								}
								_ => {
									// For other message types, we might not need to forward to NATS
									info!(
										"Received non-response message from WebSocket, skipping NATS forward"
									);
									continue;
								}
							}
						}
						::std::result::Result::Err(e) => {
							error!(?e, "failed to deserialize tunnel message from WebSocket");
						}
					}
				}
				::std::result::Result::Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
					info!(?runner_id, "WebSocket closed");
					break;
				}
				::std::result::Result::Ok(_) => {
					// Ignore other message types
				}
				Err(e) => {
					error!(?e, "WebSocket error");
					break;
				}
			}
		}

		// Clean up connection
		connections_clone.write().await.remove(&connection_id);
	});

	// Wait for either task to complete
	tokio::select! {
		_ = nats_to_ws => {
			info!("NATS to WebSocket task completed");
		}
		_ = ws_to_nats => {
			info!("WebSocket to NATS task completed");
		}
	}

	// Clean up
	connections.write().await.remove(&connection_id);
	info!(?runner_id, "connection closed");

	Ok(())
}

/// Determines if the tunnel is closed by if the UPS service is no longer responding.
fn is_tunnel_closed_error(err: &anyhow::Error) -> bool {
	if let Some(err) = err
		.chain()
		.find_map(|x| x.downcast_ref::<rivet_error::RivetError>())
		&& err.group() == "ups"
		&& (err.code() == "no_responders" || err.code() == "request_timeout")
	{
		true
	} else {
		false
	}
}

// Helper: Build and send a standard tunnel-closed Close frame (hyper-tungstenite)
fn tunnel_closed_close_msg_hyper() -> WsMessage {
	WsMessage::Close(Some(WsCloseFrame {
		code: WsCloseCode::Error,
		reason: WsUtf8Bytes::from_static("Tunnel closed"),
	}))
}

// Helper: Build and send a standard tunnel-closed Close frame (tokio-tungstenite)
fn tunnel_closed_close_msg_tokio() -> tokio_tungstenite::tungstenite::Message {
	tokio_tungstenite::tungstenite::Message::Close(Some(
		tokio_tungstenite::tungstenite::protocol::frame::CloseFrame {
			code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Error,
			reason: tokio_tungstenite::tungstenite::Utf8Bytes::from_static("Tunnel closed"),
		},
	))
}

// Helper: Send the tunnel-closed Close frame on a hyper-tungstenite sink
async fn send_tunnel_closed_close_hyper<S>(ws_write: &tokio::sync::Mutex<S>)
where
	S: futures::Sink<WsMessage> + Unpin,
{
	let mut stream = ws_write.lock().await;
	let _ = stream.send(tunnel_closed_close_msg_hyper()).await;
}

// Helper: Send the tunnel-closed Close frame on a tokio-tungstenite sink
async fn send_tunnel_closed_close_tokio<S>(ws_write: &tokio::sync::Mutex<S>)
where
	S: futures::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
{
	let mut stream = ws_write.lock().await;
	let _ = stream.send(tunnel_closed_close_msg_tokio()).await;
}
