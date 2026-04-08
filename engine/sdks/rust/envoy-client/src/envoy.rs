use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::actor::ToActor;
use crate::commands::{ACK_COMMANDS_INTERVAL_MS, handle_commands, send_command_ack};
use crate::config::EnvoyConfig;
use crate::connection::{start_connection, ws_send};
use crate::context::{SharedContext, WsTxMessage};
use crate::events::{handle_ack_events, handle_send_events, resend_unacknowledged_events};
use crate::handle::EnvoyHandle;
use crate::kv::{
	KV_CLEANUP_INTERVAL_MS, KvRequestEntry, cleanup_old_kv_requests, handle_kv_request,
	handle_kv_response, process_unsent_kv_requests,
};
use crate::tunnel::{
	HibernatingWebSocketMetadata, handle_tunnel_message, resend_buffered_tunnel_messages,
	send_hibernatable_ws_message_ack,
};
use crate::utils::{BufferMap, EnvoyShutdownError};

static GLOBAL_ENVOY: OnceLock<EnvoyHandle> = OnceLock::new();

pub struct EnvoyContext {
	pub shared: Arc<SharedContext>,
	pub shutting_down: bool,
	pub actors: HashMap<String, HashMap<u32, ActorEntry>>,
	pub kv_requests: HashMap<u32, KvRequestEntry>,
	pub next_kv_request_id: u32,
	pub request_to_actor: BufferMap<String>,
	pub buffered_messages: Vec<protocol::ToRivetTunnelMessage>,
}

pub struct ActorEntry {
	pub handle: mpsc::UnboundedSender<ToActor>,
	pub name: String,
	pub event_history: Vec<protocol::EventWrapper>,
	pub last_command_idx: i64,
}

pub enum ToEnvoyMessage {
	ConnMessage {
		message: protocol::ToEnvoy,
	},
	ConnClose {
		evict: bool,
	},
	SendEvents {
		events: Vec<protocol::EventWrapper>,
	},
	KvRequest {
		actor_id: String,
		data: protocol::KvRequestData,
		response_tx: oneshot::Sender<anyhow::Result<protocol::KvResponseData>>,
	},
	BufferTunnelMsg {
		msg: protocol::ToRivetTunnelMessage,
	},
	ActorIntent {
		actor_id: String,
		generation: Option<u32>,
		intent: protocol::ActorIntent,
		error: Option<String>,
	},
	SetAlarm {
		actor_id: String,
		generation: Option<u32>,
		alarm_ts: Option<i64>,
	},
	HwsRestore {
		actor_id: String,
		meta_entries: Vec<HibernatingWebSocketMetadata>,
	},
	HwsAck {
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		envoy_message_index: u16,
	},
	GetActor {
		actor_id: String,
		generation: Option<u32>,
		response_tx: oneshot::Sender<Option<ActorInfo>>,
	},
	Shutdown,
	Stop,
}

/// Information about an actor, returned by `EnvoyHandle::get_actor`.
#[derive(Debug, Clone)]
pub struct ActorInfo {
	pub name: String,
	pub generation: u32,
}

impl EnvoyContext {
	pub fn get_actor(&self, actor_id: &str, generation: Option<u32>) -> Option<&ActorEntry> {
		let gens = self.actors.get(actor_id)?;
		if gens.is_empty() {
			return None;
		}

		if let Some(g) = generation {
			return gens.get(&g);
		}

		// Return highest generation non-closed entry
		// HashMap doesn't guarantee order, so find max key
		let mut best: Option<&ActorEntry> = None;
		let mut best_gen: u32 = 0;
		for (&g, entry) in gens {
			if !entry.handle.is_closed() && (best.is_none() || g > best_gen) {
				best = Some(entry);
				best_gen = g;
			}
		}
		best
	}

	pub fn get_actor_entry_mut(
		&mut self,
		actor_id: &str,
		generation: u32,
	) -> Option<&mut ActorEntry> {
		self.actors
			.get_mut(actor_id)
			.and_then(|gens| gens.get_mut(&generation))
	}
}

pub async fn start_envoy(config: EnvoyConfig) -> EnvoyHandle {
	let handle = start_envoy_sync(config);
	handle.started().await;
	handle
}

pub fn start_envoy_sync(config: EnvoyConfig) -> EnvoyHandle {
	if config.not_global {
		start_envoy_sync_inner(config)
	} else {
		GLOBAL_ENVOY
			.get_or_init(|| start_envoy_sync_inner(config))
			.clone()
	}
}

fn start_envoy_sync_inner(config: EnvoyConfig) -> EnvoyHandle {
	let (envoy_tx, envoy_rx) = mpsc::unbounded_channel::<ToEnvoyMessage>();
	let (start_tx, start_rx) = tokio::sync::watch::channel(());

	let envoy_key = uuid::Uuid::new_v4().to_string();
	let shared = Arc::new(SharedContext {
		config,
		envoy_key,
		envoy_tx: envoy_tx.clone(),
		ws_tx: Arc::new(tokio::sync::Mutex::new(None)),
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: std::sync::atomic::AtomicBool::new(false),
	});

	let handle = EnvoyHandle {
		shared: shared.clone(),
		started_rx: start_rx,
	};

	// Start signal handler
	let handle2 = handle.clone();
	tokio::spawn(async move {
		let _ = tokio::signal::ctrl_c().await;
		handle2.shutdown(false);
	});

	start_connection(shared.clone());

	let ctx = EnvoyContext {
		shared: shared.clone(),
		shutting_down: false,
		actors: HashMap::new(),
		kv_requests: HashMap::new(),
		next_kv_request_id: 0,
		request_to_actor: BufferMap::new(),
		buffered_messages: Vec::new(),
	};

	tracing::info!("starting envoy");

	tokio::spawn(envoy_loop(ctx, envoy_rx, start_tx));

	handle
}

async fn envoy_loop(
	mut ctx: EnvoyContext,
	mut rx: mpsc::UnboundedReceiver<ToEnvoyMessage>,
	start_tx: tokio::sync::watch::Sender<()>,
) {
	let mut ack_interval =
		tokio::time::interval(std::time::Duration::from_millis(ACK_COMMANDS_INTERVAL_MS));
	let mut kv_cleanup_interval =
		tokio::time::interval(std::time::Duration::from_millis(KV_CLEANUP_INTERVAL_MS));

	let mut lost_timeout: Option<std::pin::Pin<Box<tokio::time::Sleep>>> = None;

	loop {
		tokio::select! {
			msg = rx.recv() => {
				let Some(msg) = msg else { break };

				match msg {
					ToEnvoyMessage::ConnMessage { message } => {
						lost_timeout = handle_conn_message(&mut ctx, &start_tx, lost_timeout, message).await;
					}
					ToEnvoyMessage::ConnClose { evict } => {
						lost_timeout = handle_conn_close(&ctx, lost_timeout);
						if evict { break; }
					}
					ToEnvoyMessage::SendEvents { events } => {
						handle_send_events(&mut ctx, events).await;
					}
					ToEnvoyMessage::KvRequest { actor_id, data, response_tx } => {
						handle_kv_request(&mut ctx, actor_id, data, response_tx).await;
					}
					ToEnvoyMessage::BufferTunnelMsg { msg } => {
						ctx.buffered_messages.push(msg);
					}
					ToEnvoyMessage::ActorIntent { actor_id, generation, intent, error } => {
						if let Some(entry) = ctx.get_actor(&actor_id, generation) {
							let _ = entry.handle.send(ToActor::Intent { intent, error });
						}
					}
					ToEnvoyMessage::SetAlarm { actor_id, generation, alarm_ts } => {
						if let Some(entry) = ctx.get_actor(&actor_id, generation) {
							let _ = entry.handle.send(ToActor::SetAlarm { alarm_ts });
						}
					}
					ToEnvoyMessage::HwsRestore { actor_id, meta_entries } => {
						if let Some(entry) = ctx.get_actor(&actor_id, None) {
							let _ = entry.handle.send(ToActor::HwsRestore { meta_entries });
						}
					}
					ToEnvoyMessage::HwsAck { gateway_id, request_id, envoy_message_index } => {
						send_hibernatable_ws_message_ack(&mut ctx, gateway_id, request_id, envoy_message_index);
					}
					ToEnvoyMessage::GetActor { actor_id, generation, response_tx } => {
						let info = ctx.get_actor(&actor_id, generation).map(|entry| {
							let actor_gen = generation.unwrap_or_else(|| {
								ctx.actors
									.get(&actor_id)
									.and_then(|gens| {
										gens.iter()
											.filter(|(_, e)| !e.handle.is_closed())
											.map(|(&g, _)| g)
											.max()
									})
									.unwrap_or(0)
							});
							ActorInfo {
								name: entry.name.clone(),
								generation: actor_gen,
							}
						});
						let _ = response_tx.send(info);
					}
					ToEnvoyMessage::Shutdown => {
						handle_shutdown(&mut ctx).await;
					}
					ToEnvoyMessage::Stop => {
						break;
					}
				}
			}
			_ = ack_interval.tick() => {
				send_command_ack(&ctx).await;
			}
			_ = kv_cleanup_interval.tick() => {
				cleanup_old_kv_requests(&mut ctx);
			}
			_ = async {
				match lost_timeout.as_mut() {
					Some(timeout) => timeout.as_mut().await,
					None => std::future::pending::<()>().await,
				}
			} => {
				// Lost timeout fired
				for (_id, request) in ctx.kv_requests.drain() {
					let _ = request.response_tx.send(Err(anyhow::anyhow!(EnvoyShutdownError)));
				}

				if !ctx.actors.is_empty() {
					tracing::warn!("stopping all actors due to envoy lost threshold");
					for (_actor_id, gens) in &ctx.actors {
						for (_g, entry) in gens {
							if !entry.handle.is_closed() {
								let _ = entry.handle.send(ToActor::Lost);
							}
						}
					}
					ctx.actors.clear();
				}

				lost_timeout = None;
			}
		}
	}

	// Cleanup
	{
		let guard = ctx.shared.ws_tx.lock().await;
		if let Some(tx) = guard.as_ref() {
			let _ = tx.send(WsTxMessage::Close);
		}
	}

	for (_id, request) in ctx.kv_requests.drain() {
		let _ = request
			.response_tx
			.send(Err(anyhow::anyhow!("envoy shutting down")));
	}

	ctx.actors.clear();

	tracing::info!("envoy stopped");

	ctx.shared.config.callbacks.on_shutdown();
}

async fn handle_conn_message(
	ctx: &mut EnvoyContext,
	start_tx: &tokio::sync::watch::Sender<()>,
	mut lost_timeout: Option<std::pin::Pin<Box<tokio::time::Sleep>>>,
	message: protocol::ToEnvoy,
) -> Option<std::pin::Pin<Box<tokio::time::Sleep>>> {
	match message {
		protocol::ToEnvoy::ToEnvoyInit(init) => {
			{
				let mut guard = ctx.shared.protocol_metadata.lock().await;
				*guard = Some(init.metadata.clone());
			}
			tracing::info!(?init.metadata, "received init");

			lost_timeout = None;
			resend_unacknowledged_events(ctx).await;
			process_unsent_kv_requests(ctx).await;
			resend_buffered_tunnel_messages(ctx).await;

			let _ = start_tx.send(());
		}
		protocol::ToEnvoy::ToEnvoyCommands(commands) => {
			handle_commands(ctx, commands).await;
		}
		protocol::ToEnvoy::ToEnvoyAckEvents(ack) => {
			handle_ack_events(ctx, ack);
		}
		protocol::ToEnvoy::ToEnvoyKvResponse(response) => {
			handle_kv_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoyTunnelMessage(tunnel_msg) => {
			handle_tunnel_message(ctx, tunnel_msg).await;
		}
		protocol::ToEnvoy::ToEnvoyPing(_) => {
			// Should be handled by connection task
		}
	}

	lost_timeout
}

fn handle_conn_close(
	ctx: &EnvoyContext,
	lost_timeout: Option<std::pin::Pin<Box<tokio::time::Sleep>>>,
) -> Option<std::pin::Pin<Box<tokio::time::Sleep>>> {
	if lost_timeout.is_some() {
		return lost_timeout;
	}

	// Read threshold from protocol metadata, fall back to 10 seconds
	let lost_threshold = {
		let metadata = ctx.shared.protocol_metadata.try_lock().ok();
		metadata
			.and_then(|guard| guard.as_ref().map(|m| m.envoy_lost_threshold as u64))
			.unwrap_or(10_000)
	};

	tracing::debug!(ms = lost_threshold, "starting envoy lost timeout");

	Some(Box::pin(tokio::time::sleep(
		std::time::Duration::from_millis(lost_threshold),
	)))
}

async fn handle_shutdown(ctx: &mut EnvoyContext) {
	if ctx.shutting_down {
		return;
	}
	ctx.shutting_down = true;

	tracing::debug!("envoy received shutdown");

	ws_send(&ctx.shared, protocol::ToRivet::ToRivetStopping).await;

	// Check if any actors are still active
	let has_actors = ctx
		.actors
		.values()
		.any(|gens| gens.values().any(|entry| !entry.handle.is_closed()));

	if !has_actors {
		let _ = ctx.shared.envoy_tx.send(ToEnvoyMessage::Stop);
	} else {
		// Wait for all actors to finish. The process manager (Docker,
		// k8s, etc.) provides the ultimate shutdown deadline.
		let actor_handles: Vec<mpsc::UnboundedSender<ToActor>> = ctx
			.actors
			.values()
			.flat_map(|gens| gens.values())
			.filter(|entry| !entry.handle.is_closed())
			.map(|entry| entry.handle.clone())
			.collect();

		let envoy_tx = ctx.shared.envoy_tx.clone();
		tokio::spawn(async move {
			futures_util::future::join_all(actor_handles.iter().map(|h| h.closed())).await;
			tracing::debug!("all actors stopped during graceful shutdown");
			let _ = envoy_tx.send(ToEnvoyMessage::Stop);
		});
	}
}
