use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use rivet_actor_plugin_abi::{
	ConnInfo, Event, KeepAwakeToken, KvEntry, KvListOpts, PortableActorBackend, PortableActorCtx,
	PortableBoxFuture, ReplyToken, RequestSaveOpts as PortableRequestSaveOpts, ScheduledEvent,
};
use tokio::sync::{Mutex as AsyncMutex, oneshot};

use crate::actor::connection::ConnHandle;
use crate::actor::lifecycle_hooks::{ActorEvents, ActorStart, Reply};
use crate::actor::messages::{ActorEvent, Response, StateDelta};
use crate::actor::messages::{QueueSendResult, QueueSendStatus};
use crate::actor::state::RequestSaveOpts;
use crate::actor::task_types::ShutdownKind;
use crate::types::{ListOpts, format_actor_key};
use crate::{ActorConfig, ActorContext, ActorFactory};

pub struct NativeBackend {
	ctx: ActorContext,
	events: AsyncMutex<ActorEvents>,
	startup: Mutex<Option<oneshot::Sender<Result<()>>>>,
	slab: ReplySlab,
	keep_awake: KeepAwakeStore,
}

impl NativeBackend {
	pub fn new(start: ActorStart) -> Self {
		Self {
			ctx: start.ctx,
			events: AsyncMutex::new(start.events),
			startup: Mutex::new(start.startup_ready),
			slab: ReplySlab::new(),
			keep_awake: KeepAwakeStore::new(),
		}
	}
}

impl PortableActorBackend for NativeBackend {
	fn next_event(&self) -> PortableBoxFuture<'_, Result<Option<Event>>> {
		Box::pin(async move {
			loop {
				let event = {
					let mut events = self.events.lock().await;
					events.recv().await
				};
				let Some(event) = event else {
					self.slab.drain();
					return Ok(None);
				};

				match event {
					ActorEvent::Action {
						name, args, reply, ..
					} => {
						let token = self.slab.insert(PendingReply::Bytes(reply));
						return Ok(Some(Event::Action {
							name,
							args,
							reply: token,
						}));
					}
					ActorEvent::HttpRequest { request, reply } => {
						let token = self.slab.insert(PendingReply::Http(reply));
						return Ok(Some(Event::Http {
							request: encode_http_request(&request),
							reply: token,
						}));
					}
					ActorEvent::SubscribeRequest {
						conn,
						event_name,
						reply,
					} => {
						let token = self.slab.insert(PendingReply::Unit(reply));
						return Ok(Some(Event::Subscribe {
							conn: conn_info(&conn),
							event_name,
							reply: token,
						}));
					}
					ActorEvent::QueueSend {
						name,
						body,
						conn,
						request,
						wait,
						timeout_ms,
						reply,
					} => {
						let token = self.slab.insert(PendingReply::Queue(reply));
						return Ok(Some(Event::QueueSend {
							name,
							body,
							conn: conn_info(&conn),
							request: encode_http_request(&request),
							wait,
							timeout_ms,
							reply: token,
						}));
					}
					ActorEvent::WebSocketOpen {
						conn,
						request,
						reply,
						..
					} => {
						let token = self.slab.insert(PendingReply::Unit(reply));
						return Ok(Some(Event::WebSocketOpen {
							conn: conn_info(&conn),
							request: request.as_ref().map(encode_http_request),
							reply: token,
						}));
					}
					ActorEvent::ConnectionPreflight {
						conn,
						params,
						reply,
						..
					} => {
						let token = self.slab.insert(PendingReply::Unit(reply));
						return Ok(Some(Event::ConnPreflight {
							conn: conn_info(&conn),
							params,
							reply: token,
						}));
					}
					ActorEvent::ConnectionOpen { conn, reply, .. } => {
						let token = self.slab.insert(PendingReply::Unit(reply));
						return Ok(Some(Event::ConnOpen {
							conn: conn_info(&conn),
							reply: token,
						}));
					}
					ActorEvent::ConnectionClosed { conn } => {
						return Ok(Some(Event::ConnClosed {
							conn: conn_info(&conn),
						}));
					}
					ActorEvent::SerializeState { reply, .. } => {
						let token = self.slab.insert(PendingReply::State(reply));
						return Ok(Some(Event::SerializeState { reply: token }));
					}
					ActorEvent::RunGracefulCleanup { reason, reply } => {
						let token = self.slab.insert(PendingReply::Unit(reply));
						return Ok(Some(match reason {
							ShutdownKind::Sleep => Event::Sleep { reply: token },
							ShutdownKind::Destroy => Event::Destroy { reply: token },
						}));
					}
					ActorEvent::DisconnectConn { reply, .. } => reply.send(Ok(())),
					ActorEvent::WorkflowHistoryRequested { reply } => reply.send(Ok(None)),
					ActorEvent::WorkflowReplayRequested { reply, .. } => reply.send(Ok(None)),

					#[cfg(test)]
					ActorEvent::BeginSleep => {}
					#[cfg(test)]
					ActorEvent::FinalizeSleep { reply } => reply.send(Ok(())),
					#[cfg(test)]
					ActorEvent::Destroy { reply } => reply.send(Ok(())),
				}
			}
		})
	}

	fn reply_ok(&self, token: ReplyToken, payload: Vec<u8>) -> Result<()> {
		self.slab.fulfill_ok(token, payload)
	}

	fn reply_err(&self, token: ReplyToken, message: String) -> Result<()> {
		self.slab.fulfill_err(token, message)
	}

	fn startup_ready(&self, result: Result<()>) -> Result<()> {
		let is_ok = result.is_ok();
		if let Some(tx) = self.startup.lock().take() {
			let _ = tx.send(result);
		}
		if is_ok {
			Ok(())
		} else {
			Err(anyhow!("startup_ready signaled failure"))
		}
	}

	fn broadcast(&self, name: String, payload: Vec<u8>) -> Result<()> {
		self.ctx.broadcast(&name, &payload);
		Ok(())
	}

	fn actor_id(&self) -> Result<String> {
		Ok(self.ctx.actor_id().to_owned())
	}

	fn name(&self) -> Result<String> {
		Ok(self.ctx.name().to_owned())
	}

	fn key(&self) -> Result<String> {
		Ok(format_actor_key(self.ctx.key()))
	}

	fn region(&self) -> Result<String> {
		Ok(self.ctx.region().to_owned())
	}

	fn input(&self) -> Result<Option<Vec<u8>>> {
		Ok(self.ctx.input())
	}

	fn has_state(&self) -> Result<bool> {
		Ok(self.ctx.has_state())
	}

	fn state(&self) -> Result<Vec<u8>> {
		Ok(self.ctx.state())
	}

	fn set_state(&self, state: Vec<u8>) -> Result<()> {
		self.ctx.set_initial_state(state);
		Ok(())
	}

	fn save_state(&self, state: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx
				.save_state(vec![StateDelta::ActorState(state)])
				.await
		})
	}

	fn request_save(&self, opts: PortableRequestSaveOpts) -> Result<()> {
		self.ctx.request_save(RequestSaveOpts {
			immediate: opts.immediate,
			max_wait_ms: opts.max_wait_ms,
		});
		Ok(())
	}

	fn request_save_and_wait(
		&self,
		opts: PortableRequestSaveOpts,
	) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx
				.request_save_and_wait(RequestSaveOpts {
					immediate: opts.immediate,
					max_wait_ms: opts.max_wait_ms,
				})
				.await
		})
	}

	fn sleep(&self) -> Result<()> {
		self.ctx.sleep()
	}

	fn actor_aborted(&self) -> Result<bool> {
		Ok(self.ctx.actor_aborted())
	}

	fn wait_for_actor_abort(&self) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx.actor_abort_signal().cancelled().await;
			Ok(())
		})
	}

	fn keep_awake_enter(&self) -> Result<KeepAwakeToken> {
		Ok(self.keep_awake.insert(self.ctx.keep_awake_region()))
	}

	fn keep_awake_exit(&self, token: KeepAwakeToken) -> Result<()> {
		self.keep_awake.remove(token)
	}

	fn keep_awake_count(&self) -> Result<usize> {
		Ok(self.ctx.keep_awake_count())
	}

	fn kv_get(&self, key: Vec<u8>) -> PortableBoxFuture<'_, Result<Option<Vec<u8>>>> {
		Box::pin(async move {
			let mut values = self.ctx.kv_batch_get(&[key.as_slice()]).await?;
			Ok(values.pop().flatten())
		})
	}

	fn kv_put(&self, key: Vec<u8>, value: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx
				.kv_batch_put(&[(key.as_slice(), value.as_slice())])
				.await
		})
	}

	fn kv_delete(&self, key: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move { self.ctx.kv_batch_delete(&[key.as_slice()]).await })
	}

	fn kv_batch_get(
		&self,
		keys: Vec<Vec<u8>>,
	) -> PortableBoxFuture<'_, Result<Vec<Option<Vec<u8>>>>> {
		Box::pin(async move {
			let key_refs: Vec<&[u8]> = keys.iter().map(Vec::as_slice).collect();
			self.ctx.kv_batch_get(&key_refs).await
		})
	}

	fn kv_batch_put(&self, entries: Vec<KvEntry>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			let entry_refs: Vec<(&[u8], &[u8])> = entries
				.iter()
				.map(|entry| (entry.key.as_slice(), entry.value.as_slice()))
				.collect();
			self.ctx.kv_batch_put(&entry_refs).await
		})
	}

	fn kv_batch_delete(&self, keys: Vec<Vec<u8>>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			let key_refs: Vec<&[u8]> = keys.iter().map(Vec::as_slice).collect();
			self.ctx.kv_batch_delete(&key_refs).await
		})
	}

	fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move { self.ctx.kv_delete_range(&start, &end).await })
	}

	fn kv_list_prefix(
		&self,
		prefix: Vec<u8>,
		opts: KvListOpts,
	) -> PortableBoxFuture<'_, Result<Vec<KvEntry>>> {
		Box::pin(async move {
			Ok(self
				.ctx
				.kv_list_prefix(&prefix, list_opts(opts))
				.await?
				.into_iter()
				.map(|(key, value)| KvEntry { key, value })
				.collect())
		})
	}

	fn kv_list_range(
		&self,
		start: Vec<u8>,
		end: Vec<u8>,
		opts: KvListOpts,
	) -> PortableBoxFuture<'_, Result<Vec<KvEntry>>> {
		Box::pin(async move {
			Ok(self
				.ctx
				.kv_list_range(&start, &end, list_opts(opts))
				.await?
				.into_iter()
				.map(|(key, value)| KvEntry { key, value })
				.collect())
		})
	}

	fn schedule_after_ms(
		&self,
		delay_ms: u64,
		action_name: String,
		args: Vec<u8>,
	) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx
				.after(Duration::from_millis(delay_ms), &action_name, &args);
			Ok(())
		})
	}

	fn schedule_at_ms(
		&self,
		timestamp_ms: i64,
		action_name: String,
		args: Vec<u8>,
	) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx.at(timestamp_ms, &action_name, &args);
			Ok(())
		})
	}

	fn set_alarm(&self, timestamp_ms: Option<i64>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move { self.ctx.set_alarm(timestamp_ms) })
	}

	fn scheduled_events(&self) -> PortableBoxFuture<'_, Result<Vec<ScheduledEvent>>> {
		Box::pin(async move {
			Ok(self
				.ctx
				.scheduled_events()
				.into_iter()
				.map(|event| ScheduledEvent {
					event_id: event.event_id,
					timestamp_ms: event.timestamp,
					action_name: event.action,
					args: event.args,
				})
				.collect())
		})
	}

	fn conn_list(&self) -> PortableBoxFuture<'_, Result<Vec<ConnInfo>>> {
		Box::pin(async move { Ok(self.ctx.conns().map(|conn| conn_info(&conn)).collect()) })
	}

	fn disconnect_conn(&self, conn_id: String) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move { self.ctx.disconnect_conn(conn_id).await })
	}

	fn disconnect_conns(&self, conn_ids: Vec<String>) -> PortableBoxFuture<'_, Result<()>> {
		Box::pin(async move {
			self.ctx
				.disconnect_conns(|conn| conn_ids.iter().any(|id| id == conn.id()))
				.await
		})
	}

	fn send(&self, conn_id: String, name: String, payload: Vec<u8>) -> Result<()> {
		let conn = self
			.ctx
			.conns()
			.find(|conn| conn.id() == conn_id)
			.ok_or_else(|| anyhow!("connection `{conn_id}` not found"))?;
		conn.try_send(&name, &payload)
	}

	fn ack_hibernatable_websocket_message(
		&self,
		gateway_id: Vec<u8>,
		request_id: Vec<u8>,
		server_message_index: u16,
	) -> Result<()> {
		self.ctx
			.ack_hibernatable_websocket_message(&gateway_id, &request_id, server_message_index)
	}

	fn sql_is_enabled(&self) -> bool {
		self.ctx.sql().is_enabled()
	}

	fn db_exec<'a>(&'a self, sql: &'a str) -> PortableBoxFuture<'a, Result<Vec<u8>>> {
		Box::pin(async move { self.ctx.db_exec(sql).await })
	}

	fn db_query<'a>(
		&'a self,
		sql: &'a str,
		params: Option<Vec<u8>>,
	) -> PortableBoxFuture<'a, Result<Vec<u8>>> {
		Box::pin(async move { self.ctx.db_query(sql, params.as_deref()).await })
	}

	fn db_run<'a>(
		&'a self,
		sql: &'a str,
		params: Option<Vec<u8>>,
	) -> PortableBoxFuture<'a, Result<()>> {
		Box::pin(async move { self.ctx.db_run(sql, params.as_deref()).await })
	}
}

struct KeepAwakeStore {
	next: AtomicU64,
	regions: Mutex<HashMap<u64, crate::actor::context::KeepAwakeRegion>>,
}

impl KeepAwakeStore {
	fn new() -> Self {
		Self {
			next: AtomicU64::new(1),
			regions: Mutex::new(HashMap::new()),
		}
	}

	fn insert(&self, region: crate::actor::context::KeepAwakeRegion) -> KeepAwakeToken {
		let token = self.next.fetch_add(1, Ordering::Relaxed);
		self.regions.lock().insert(token, region);
		KeepAwakeToken { token }
	}

	fn remove(&self, token: KeepAwakeToken) -> Result<()> {
		self.regions
			.lock()
			.remove(&token.token)
			.ok_or_else(|| anyhow!("keep-awake token {} is unknown", token.token))?;
		Ok(())
	}
}

fn conn_info(conn: &ConnHandle) -> ConnInfo {
	ConnInfo {
		id: conn.id().to_owned(),
		params: conn.params(),
		state: conn.state(),
		is_hibernatable: conn.is_hibernatable(),
	}
}

fn list_opts(opts: KvListOpts) -> ListOpts {
	ListOpts {
		reverse: opts.reverse,
		limit: opts.limit,
	}
}

pub fn build_portable_native_actor_factory<F, Fut>(config: ActorConfig, actor: F) -> ActorFactory
where
	F: Fn(PortableActorCtx) -> Fut + Send + Sync + Clone + 'static,
	Fut: Future<Output = Result<()>> + Send + 'static,
{
	ActorFactory::new_with_manual_startup_ready(config, move |start| {
		let actor = actor.clone();
		Box::pin(async move {
			let ctx = PortableActorCtx::new(NativeBackend::new(start));
			actor(ctx).await
		}) as crate::runtime::RuntimeBoxFuture<Result<()>>
	})
}

enum PendingReply {
	Bytes(Reply<Vec<u8>>),
	Unit(Reply<()>),
	State(Reply<Vec<StateDelta>>),
	Http(Reply<Response>),
	Queue(Reply<QueueSendResult>),
}

impl PendingReply {
	fn fulfill_ok(self, payload: Vec<u8>) -> Result<()> {
		match self {
			PendingReply::Bytes(reply) => reply.send(Ok(payload)),
			PendingReply::Unit(reply) => reply.send(Ok(())),
			PendingReply::State(reply) => {
				let deltas = if payload.is_empty() {
					Vec::new()
				} else {
					vec![StateDelta::ActorState(payload)]
				};
				reply.send(Ok(deltas));
			}
			PendingReply::Http(reply) => reply.send(decode_http_response(&payload)),
			PendingReply::Queue(reply) => reply.send(decode_queue_send_response(&payload)),
		}
		Ok(())
	}

	fn fulfill_err(self, message: String) {
		match self {
			PendingReply::Bytes(reply) => reply.send(Err(anyhow!("{message}"))),
			PendingReply::Unit(reply) => reply.send(Err(anyhow!("{message}"))),
			PendingReply::State(reply) => reply.send(Err(anyhow!("{message}"))),
			PendingReply::Http(reply) => reply.send(Err(anyhow!("{message}"))),
			PendingReply::Queue(reply) => reply.send(Err(anyhow!("{message}"))),
		}
	}
}

fn decode_queue_send_response(bytes: &[u8]) -> Result<QueueSendResult> {
	let wire: rivet_actor_plugin_abi::QueueSendResponse =
		ciborium::from_reader(std::io::Cursor::new(bytes))?;
	let status = match wire.status.as_str() {
		"completed" => QueueSendStatus::Completed,
		"timedOut" => QueueSendStatus::TimedOut,
		other => return Err(anyhow!("unknown queue send status `{other}`")),
	};
	Ok(QueueSendResult {
		status,
		response: wire.response,
	})
}

struct ReplySlab {
	next: AtomicU64,
	map: Mutex<HashMap<u64, PendingReply>>,
}

impl ReplySlab {
	fn new() -> Self {
		Self {
			next: AtomicU64::new(1),
			map: Mutex::new(HashMap::new()),
		}
	}

	fn insert(&self, reply: PendingReply) -> ReplyToken {
		let token = self.next.fetch_add(1, Ordering::Relaxed);
		self.map.lock().insert(token, reply);
		ReplyToken(token)
	}

	fn fulfill_ok(&self, token: ReplyToken, payload: Vec<u8>) -> Result<()> {
		let reply = self
			.map
			.lock()
			.remove(&token.0)
			.ok_or_else(|| anyhow!("reply token {} is unknown or already answered", token.0))?;
		reply.fulfill_ok(payload)
	}

	fn fulfill_err(&self, token: ReplyToken, message: String) -> Result<()> {
		let reply = self
			.map
			.lock()
			.remove(&token.0)
			.ok_or_else(|| anyhow!("reply token {} is unknown or already answered", token.0))?;
		reply.fulfill_err(message);
		Ok(())
	}

	fn drain(&self) {
		self.map.lock().clear();
	}
}

fn encode_http_request(req: &crate::actor::messages::Request) -> Vec<u8> {
	let (method, uri, headers, body) = req.to_parts();
	let wire = HttpReqWire {
		method,
		uri,
		headers,
		body,
	};
	let mut out = Vec::new();
	let _ = ciborium::into_writer(&wire, &mut out);
	out
}

fn decode_http_response(bytes: &[u8]) -> Result<Response> {
	let wire: HttpRespWire =
		ciborium::from_reader(std::io::Cursor::new(bytes)).map_err(|e| anyhow!("{e}"))?;
	Response::from_parts(wire.status, wire.headers, wire.body)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HttpReqWire {
	method: String,
	uri: String,
	headers: HashMap<String, String>,
	#[serde(with = "serde_bytes")]
	body: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HttpRespWire {
	status: u16,
	headers: HashMap<String, String>,
	#[serde(with = "serde_bytes")]
	body: Vec<u8>,
}
