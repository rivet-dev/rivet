use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Weak};

use anyhow::Error;
use napi::bindgen_prelude::{Buffer, Either, Promise};
use napi::threadsafe_function::{
	ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction,
	ThreadsafeFunctionCallMode,
};
use napi::{Env, JsFunction, JsObject};
use napi_derive::napi;
use rivetkit_core::types::ActorKeySegment;
use rivetkit_core::{
	ActorContext as CoreActorContext, ConnHandle as CoreConnHandle,
	Request as CoreRequest, StateDelta, WebSocketCallbackRegion,
};
use scc::HashMap as SccHashMap;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken as CoreCancellationToken;

use crate::actor_factory::BridgeRivetErrorContext;
use crate::connection::ConnHandle;
use crate::kv::Kv;
use crate::napi_anyhow_error;
use crate::queue::Queue;
use crate::schedule::Schedule;
use crate::sqlite_db::SqliteDb;

type AbortSignalTsfn =
	ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>;
type DisconnectPredicateTsfn =
	ThreadsafeFunction<DisconnectPredicatePayload, ErrorStrategy::CalleeHandled>;
type RunRestartHook =
	Arc<dyn Fn() -> anyhow::Result<()> + Send + Sync + 'static>;
pub(crate) type RegisteredTask =
	Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

static ACTOR_CONTEXT_SHARED: LazyLock<SccHashMap<String, Weak<ActorContextShared>>> =
	LazyLock::new(SccHashMap::new);

/// N-API wrapper around `rivetkit-core::ActorContext`.
#[derive(Clone)]
#[napi]
pub struct ActorContext {
	inner: CoreActorContext,
	shared: Arc<ActorContextShared>,
}

#[derive(Default)]
struct ActorContextShared {
	abort_token: Mutex<Option<CoreCancellationToken>>,
	run_restart: Mutex<Option<RunRestartHook>>,
	task_sender: Mutex<Option<UnboundedSender<RegisteredTask>>>,
	end_reason: Mutex<Option<EndReason>>,
	websocket_callback_region: Mutex<Option<WebSocketCallbackRegion>>,
	ready: AtomicBool,
	started: AtomicBool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EndReason {
	Sleep,
	Destroy,
}

#[napi(object)]
pub struct JsActorKeySegment {
	pub kind: String,
	pub string_value: Option<String>,
	pub number_value: Option<f64>,
}

#[napi(object)]
pub struct JsHttpRequest {
	pub method: String,
	pub uri: String,
	pub headers: Option<std::collections::HashMap<String, String>>,
	pub body: Option<Buffer>,
}

#[napi(object)]
pub struct StateDeltaConnHibernationEntry {
	pub conn_id: String,
	pub bytes: Buffer,
}

#[napi(object)]
pub struct StateDeltaPayload {
	pub state: Option<Buffer>,
	pub conn_hibernation: Vec<StateDeltaConnHibernationEntry>,
	pub conn_hibernation_removed: Vec<String>,
}

#[napi(object)]
pub struct JsInspectorSnapshot {
	pub state_revision: i64,
	pub connections_revision: i64,
	pub queue_revision: i64,
	pub active_connections: u32,
	pub queue_size: u32,
	pub connected_clients: u32,
}

#[derive(Clone)]
struct DisconnectPredicatePayload {
	conn: CoreConnHandle,
}

impl ActorContext {
	pub(crate) fn new(inner: CoreActorContext) -> Self {
		let shared = actor_context_shared(inner.actor_id());
		Self { inner, shared }
	}

	#[allow(dead_code)]
	pub(crate) fn inner(&self) -> &CoreActorContext {
		&self.inner
	}

	pub(crate) fn attach_napi_abort_token(&self, token: CoreCancellationToken) {
		self.shared.set_abort_token(token);
	}

	pub(crate) fn reset_runtime_shared_state(&self) {
		self.shared.reset_runtime_state();
	}

	pub(crate) fn attach_run_restart<F>(&self, restart: F)
	where
		F: Fn() -> anyhow::Result<()> + Send + Sync + 'static,
	{
		self.shared.set_run_restart(Arc::new(restart));
	}

	pub(crate) fn attach_task_sender(
		&self,
		sender: UnboundedSender<RegisteredTask>,
	) {
		self.shared.set_task_sender(sender);
	}

	pub(crate) fn set_end_reason(&self, reason: EndReason) {
		self.shared.set_end_reason(reason);
	}

	#[cfg_attr(not(test), allow(dead_code))]
	pub(crate) fn take_end_reason(&self) -> Option<EndReason> {
		self.shared.take_end_reason()
	}

	pub(crate) fn has_end_reason(&self) -> bool {
		self.shared.has_end_reason()
	}

	pub(crate) fn set_state_initial(&self, state: Vec<u8>) -> anyhow::Result<()> {
		self.inner.set_state(state)
	}

	pub(crate) async fn mark_has_initialized_and_flush(&self) -> anyhow::Result<()> {
		self.inner.set_has_initialized(true);
		self
			.inner
			.save_state(vec![StateDelta::ActorState(self.inner.state())])
			.await
	}

	pub(crate) fn restore_hibernatable_conn(
		&self,
		conn: CoreConnHandle,
		bytes: Vec<u8>,
	) -> anyhow::Result<()> {
		conn.set_state(bytes);
		Ok(())
	}

	pub(crate) fn set_conn_state_initial(
		&self,
		conn: &CoreConnHandle,
		bytes: Vec<u8>,
	) -> anyhow::Result<()> {
		conn.set_state(bytes);
		Ok(())
	}

	pub(crate) async fn init_alarms(&self) -> anyhow::Result<()> {
		self.inner.init_alarms();
		Ok(())
	}

	pub(crate) fn mark_ready_internal(&self) {
		self.shared.mark_ready();
	}

	pub(crate) fn mark_started_internal(&self) -> anyhow::Result<()> {
		self.shared.mark_started()
	}

	pub(crate) async fn drain_overdue_scheduled_events(&self) -> anyhow::Result<()> {
		self.inner.drain_overdue_scheduled_events().await
	}

	pub(crate) fn has_conn_changes(&self) -> bool {
		self.inner.conns().any(|conn| conn.is_hibernatable())
	}
}

#[napi]
impl ActorContext {
	#[napi(constructor)]
	pub fn constructor(actor_id: String, name: String, region: String) -> Self {
		Self::new(CoreActorContext::new(actor_id, name, Vec::new(), region))
	}

	#[napi]
	pub fn state(&self) -> Buffer {
		Buffer::from(self.inner.state())
	}

	#[napi]
	pub fn vars(&self) -> Buffer {
		Buffer::from(self.inner.vars())
	}

	#[napi]
	pub fn set_state(&self, state: Buffer) -> napi::Result<()> {
		self.inner
			.set_state(state.to_vec())
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn set_in_on_state_change_callback(&self, in_callback: bool) {
		self.inner.set_in_on_state_change_callback(in_callback);
	}

	#[napi]
	pub fn set_vars(&self, vars: Buffer) {
		self.inner.set_vars(vars.to_vec());
	}

	#[napi]
	pub fn kv(&self) -> Kv {
		Kv::new(self.inner.kv().clone())
	}

	#[napi]
	pub fn sql(&self) -> SqliteDb {
		SqliteDb::new(self.inner.clone())
	}

	#[napi]
	pub fn schedule(&self) -> Schedule {
		Schedule::new(self.inner.schedule().clone())
	}

	#[napi]
	pub fn queue(&self) -> Queue {
		Queue::new(self.inner.queue().clone())
	}

	#[napi]
	pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> napi::Result<()> {
		self
			.inner
			.set_alarm(timestamp_ms)
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn request_save(&self, immediate: bool) {
		self.inner.request_save(immediate);
	}

	#[napi]
	pub fn request_save_within(&self, ms: u32) {
		self.inner.request_save_within(ms);
	}

	#[napi]
	pub fn decode_inspector_request(
		&self,
		bytes: Buffer,
		advertised_version: u32,
	) -> napi::Result<Buffer> {
		let advertised_version = u16::try_from(advertised_version)
			.map_err(|_| napi::Error::from_reason("inspector version exceeds u16"))?;
		rivetkit_core::inspector::decode_request_payload(bytes.as_ref(), advertised_version)
			.map(Buffer::from)
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn encode_inspector_response(
		&self,
		bytes: Buffer,
		target_version: u32,
	) -> napi::Result<Buffer> {
		let target_version = u16::try_from(target_version)
			.map_err(|_| napi::Error::from_reason("inspector version exceeds u16"))?;
		rivetkit_core::inspector::encode_response_payload(bytes.as_ref(), target_version)
			.map(Buffer::from)
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn inspector_snapshot(&self) -> JsInspectorSnapshot {
		let snapshot = self.inner.inspector_snapshot();

		JsInspectorSnapshot {
			state_revision: u64_to_i64(snapshot.state_revision),
			connections_revision: u64_to_i64(snapshot.connections_revision),
			queue_revision: u64_to_i64(snapshot.queue_revision),
			active_connections: snapshot.active_connections,
			queue_size: snapshot.queue_size,
			connected_clients: usize_to_u32(snapshot.connected_clients),
		}
	}

	#[napi(js_name = "verifyInspectorAuth")]
	pub async fn verify_inspector_auth_js(
		&self,
		bearer_token: Option<String>,
	) -> napi::Result<()> {
		rivetkit_core::inspector::InspectorAuth::new()
			.verify(&self.inner, bearer_token.as_deref())
			.await
			.map_err(|error| {
				napi_anyhow_error(error.context(BridgeRivetErrorContext {
					public_: Some(true),
					status_code: Some(401),
				}))
			})
	}

	#[napi]
	pub fn queue_hibernation_removal(&self, conn_id: String) {
		self.inner.queue_hibernation_removal(conn_id);
	}

	#[napi]
	pub fn has_pending_hibernation_changes(&self) -> bool {
		self.inner.has_pending_hibernation_changes()
	}

	#[napi]
	pub fn take_pending_hibernation_changes(&self) -> Vec<String> {
		self.inner.take_pending_hibernation_changes()
	}

	#[napi]
	pub async fn save_state(
		&self,
		payload: Either<bool, StateDeltaPayload>,
	) -> napi::Result<()> {
		match payload {
			Either::A(immediate) => {
				// Preserve the old surface for callers that have not migrated yet.
				self.inner.request_save(immediate);
				Ok(())
			}
			Either::B(payload) => self
				.inner
				.save_state(state_deltas_from_payload(payload))
				.await
				.map_err(napi_anyhow_error),
		}
	}

	#[napi]
	pub fn actor_id(&self) -> String {
		self.inner.actor_id().to_owned()
	}

	#[napi]
	pub fn name(&self) -> String {
		self.inner.name().to_owned()
	}

	#[napi]
	pub fn key(&self) -> Vec<JsActorKeySegment> {
		self
			.inner
			.key()
			.iter()
			.map(|segment| match segment {
				ActorKeySegment::String(value) => JsActorKeySegment {
					kind: "string".to_owned(),
					string_value: Some(value.clone()),
					number_value: None,
				},
				ActorKeySegment::Number(value) => JsActorKeySegment {
					kind: "number".to_owned(),
					string_value: None,
					number_value: Some(*value),
				},
			})
			.collect()
	}

	#[napi]
	pub fn region(&self) -> String {
		self.inner.region().to_owned()
	}

	#[napi]
	pub fn sleep(&self) {
		self.inner.sleep();
	}

	#[napi]
	pub fn destroy(&self) {
		self.inner.destroy();
	}

	#[napi]
	pub fn destroy_requested(&self) -> bool {
		self.inner.is_destroy_requested()
	}

	#[napi]
	pub async fn wait_for_destroy_completion(&self) {
		self.inner.wait_for_destroy_completion_public().await;
	}

	#[napi]
	pub fn set_prevent_sleep(&self, prevent_sleep: bool) {
		self.inner.set_prevent_sleep(prevent_sleep);
	}

	#[napi]
	pub fn prevent_sleep(&self) -> bool {
		self.inner.prevent_sleep()
	}

	#[napi]
	pub fn aborted(&self) -> bool {
		self.shared.abort_token().is_cancelled()
	}

	#[napi]
	pub fn run_handler_active(&self) -> bool {
		self.shared.run_restart_configured()
	}

	#[napi]
	pub fn restart_run_handler(&self) -> napi::Result<()> {
		self
			.shared
			.run_restart()
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn mark_ready(&self) -> napi::Result<()> {
		self.shared.mark_ready();
		Ok(())
	}

	#[napi]
	pub fn mark_started(&self) -> napi::Result<()> {
		self.shared.mark_started().map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn is_ready(&self) -> bool {
		self.shared.is_ready()
	}

	#[napi]
	pub fn is_started(&self) -> bool {
		self.shared.is_started()
	}

	#[napi]
	pub fn begin_websocket_callback(&self) {
		self
			.shared
			.begin_websocket_callback(self.inner.websocket_callback_region());
	}

	#[napi]
	pub fn end_websocket_callback(&self) {
		self.shared.end_websocket_callback();
	}

	#[napi(ts_return_type = "AbortSignal")]
	pub fn abort_signal(&self, env: Env) -> napi::Result<JsObject> {
		let (signal, abort) = create_abort_signal(env)?;
		let token = self.shared.abort_token();

		napi::bindgen_prelude::spawn(async move {
			token.cancelled().await;
			let status = abort.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
			if status != napi::Status::Ok {
				tracing::warn!(?status, "failed to deliver abort signal");
			}
		});

		Ok(signal)
	}

	#[napi]
	pub fn conns(&self) -> Vec<ConnHandle> {
		self
			.inner
			.conns()
			.map(ConnHandle::new)
			.collect()
	}

	#[napi]
	pub async fn connect_conn(
		&self,
		params: Buffer,
		request: Option<JsHttpRequest>,
	) -> napi::Result<ConnHandle> {
		let request = request
			.map(js_http_request_to_core_request)
			.transpose()?;
		let conn = self
			.inner
			.connect_conn_with_request(
				params.to_vec(),
				request,
				async { Ok::<Vec<u8>, Error>(Vec::new()) },
			)
			.await
			.map_err(napi_anyhow_error)?;
		Ok(ConnHandle::new(conn))
	}

	#[napi]
	pub async fn disconnect_conn(&self, id: String) -> napi::Result<()> {
		self
			.inner
			.disconnect_conn(id)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi(ts_return_type = "Promise<void>")]
	pub fn disconnect_conns(
		&self,
		env: Env,
		predicate: JsFunction,
	) -> napi::Result<napi::JsObject> {
		let predicate = create_disconnect_predicate(&env, predicate)?;
		let ctx = self.inner.clone();

		env.execute_tokio_future(
			async move {
				let mut ids = BTreeSet::new();
				let conns = ctx.conns().collect::<Vec<_>>();

				for conn in conns {
					if call_disconnect_predicate(&predicate, conn.clone()).await? {
						ids.insert(conn.id().to_owned());
					}
				}

				ctx
					.disconnect_conns(move |conn| ids.contains(conn.id()))
					.await
					.map_err(napi_anyhow_error)?;
				Ok(())
			},
			|env, ()| env.get_undefined(),
		)
	}

	#[napi]
	pub fn broadcast(&self, name: String, args: Buffer) {
		self.inner.broadcast(&name, args.as_ref());
	}

	#[napi]
	pub async fn wait_until(
		&self,
		promise: Promise<serde_json::Value>,
	) -> napi::Result<()> {
		self.inner.wait_until(async move {
			if let Err(error) = promise.await {
				tracing::warn!(?error, "actor wait_until promise rejected");
			}
		});
		Ok(())
	}

	#[napi]
	pub fn register_task(
		&self,
		promise: Promise<serde_json::Value>,
	) -> napi::Result<()> {
		self
			.shared
			.register_task(Box::pin(async move {
				if let Err(error) = promise.await {
					tracing::warn!(?error, "actor keep_awake promise rejected");
				}
			}))
			.map_err(napi_anyhow_error)
	}
}

impl ActorContextShared {
	fn abort_token(&self) -> CoreCancellationToken {
		let mut guard = self
			.abort_token
			.lock()
			.expect("actor context abort token mutex poisoned");
		guard
			.get_or_insert_with(CoreCancellationToken::new)
			.clone()
	}

	fn set_abort_token(&self, token: CoreCancellationToken) {
		*self
			.abort_token
			.lock()
			.expect("actor context abort token mutex poisoned") = Some(token);
	}

	fn set_run_restart(&self, restart: RunRestartHook) {
		*self
			.run_restart
			.lock()
			.expect("actor context run restart mutex poisoned") = Some(restart);
	}

	fn set_task_sender(&self, sender: UnboundedSender<RegisteredTask>) {
		*self
			.task_sender
			.lock()
			.expect("actor context task sender mutex poisoned") = Some(sender);
	}

	fn register_task(&self, task: RegisteredTask) -> anyhow::Result<()> {
		let sender = self
			.task_sender
			.lock()
			.expect("actor context task sender mutex poisoned")
			.clone()
			.ok_or_else(|| anyhow::anyhow!("actor task registration is not configured"))?;
		sender
			.send(task)
			.map_err(|_| anyhow::anyhow!("actor task registration is closed"))
	}

	fn run_restart(&self) -> anyhow::Result<()> {
		let restart = self
			.run_restart
			.lock()
			.expect("actor context run restart mutex poisoned")
			.clone()
			.ok_or_else(|| anyhow::anyhow!("run handler restart is not configured"))?;
		restart()
	}

	fn run_restart_configured(&self) -> bool {
		self
			.run_restart
			.lock()
			.expect("actor context run restart mutex poisoned")
			.is_some()
	}

	fn set_end_reason(&self, reason: EndReason) {
		*self
			.end_reason
			.lock()
			.expect("actor context end reason mutex poisoned") = Some(reason);
	}

	fn begin_websocket_callback(&self, region: WebSocketCallbackRegion) {
		*self
			.websocket_callback_region
			.lock()
			.expect("actor context websocket callback mutex poisoned") = Some(region);
	}

	fn end_websocket_callback(&self) {
		self
			.websocket_callback_region
			.lock()
			.expect("actor context websocket callback mutex poisoned")
			.take();
	}

	#[cfg_attr(not(test), allow(dead_code))]
	fn take_end_reason(&self) -> Option<EndReason> {
		self
			.end_reason
			.lock()
			.expect("actor context end reason mutex poisoned")
			.take()
	}

	fn has_end_reason(&self) -> bool {
		self
			.end_reason
			.lock()
			.expect("actor context end reason mutex poisoned")
			.is_some()
	}

	fn mark_ready(&self) {
		let _ = self
			.ready
			.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
	}

	fn mark_started(&self) -> anyhow::Result<()> {
		if !self.is_ready() {
			anyhow::bail!("actor context cannot be started before it is ready");
		}

		let _ = self
			.started
			.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
		Ok(())
	}

	fn is_ready(&self) -> bool {
		self.ready.load(Ordering::SeqCst)
	}

	fn is_started(&self) -> bool {
		self.started.load(Ordering::SeqCst)
	}

	fn reset_runtime_state(&self) {
		*self
			.abort_token
			.lock()
			.expect("actor context abort token mutex poisoned") = None;
		*self
			.run_restart
			.lock()
			.expect("actor context run restart mutex poisoned") = None;
		*self
			.task_sender
			.lock()
			.expect("actor context task sender mutex poisoned") = None;
		*self
			.end_reason
			.lock()
			.expect("actor context end reason mutex poisoned") = None;
		*self
			.websocket_callback_region
			.lock()
			.expect("actor context websocket callback mutex poisoned") = None;
		self.ready.store(false, Ordering::SeqCst);
		self.started.store(false, Ordering::SeqCst);
	}
}

fn actor_context_shared(actor_id: &str) -> Arc<ActorContextShared> {
	ACTOR_CONTEXT_SHARED.retain_sync(|_, shared| shared.strong_count() > 0);

	match ACTOR_CONTEXT_SHARED.entry_sync(actor_id.to_owned()) {
		scc::hash_map::Entry::Occupied(mut entry) => {
			if let Some(shared) = entry.get().upgrade() {
				return shared;
			}

			let shared = Arc::new(ActorContextShared::default());
			*entry.get_mut() = Arc::downgrade(&shared);
			shared
		}
		scc::hash_map::Entry::Vacant(entry) => {
			let shared = Arc::new(ActorContextShared::default());
			entry.insert_entry(Arc::downgrade(&shared));
			shared
		}
	}
}

fn u64_to_i64(value: u64) -> i64 {
	value.min(i64::MAX as u64) as i64
}

fn usize_to_u32(value: usize) -> u32 {
	value.min(u32::MAX as usize) as u32
}

pub(crate) fn state_deltas_from_payload(
	payload: StateDeltaPayload,
) -> Vec<StateDelta> {
	let mut deltas = Vec::new();

	if let Some(state) = payload.state {
		deltas.push(StateDelta::ActorState(state.to_vec()));
	}

	deltas.extend(payload.conn_hibernation.into_iter().map(|entry| {
		StateDelta::ConnHibernation {
			conn: entry.conn_id,
			bytes: entry.bytes.to_vec(),
		}
	}));

	deltas.extend(
		payload
			.conn_hibernation_removed
			.into_iter()
			.map(StateDelta::ConnHibernationRemoved),
	);

	deltas
}

fn create_disconnect_predicate(
	env: &Env,
	callback: JsFunction,
) -> napi::Result<DisconnectPredicateTsfn> {
	let wrap_predicate: JsFunction =
		env.run_script("(callback => async payload => Boolean(await callback(payload)))")?;
	let wrapped = JsFunction::try_from(wrap_predicate.call(None, &[callback])?)?;

	wrapped.create_threadsafe_function(
		0,
		|ctx: ThreadSafeCallContext<DisconnectPredicatePayload>| {
			build_disconnect_predicate_payload(&ctx.env, ctx.value)
		},
	)
}

async fn call_disconnect_predicate(
	callback: &DisconnectPredicateTsfn,
	conn: CoreConnHandle,
) -> napi::Result<bool> {
	let promise = callback
		.call_async::<Promise<bool>>(Ok(DisconnectPredicatePayload { conn }))
		.await
		.map_err(|error| {
			napi::Error::from_reason(format!(
				"disconnect predicate failed: {error}"
			))
		})?;

	promise.await.map_err(|error| {
		napi::Error::from_reason(format!(
			"disconnect predicate failed: {error}"
		))
	})
}

fn build_disconnect_predicate_payload(
	env: &Env,
	payload: DisconnectPredicatePayload,
) -> napi::Result<Vec<napi::JsUnknown>> {
	let mut object = env.create_object()?;
	object.set("conn", ConnHandle::new(payload.conn))?;
	Ok(vec![object.into_unknown()])
}

fn create_abort_signal(env: Env) -> napi::Result<(JsObject, AbortSignalTsfn)> {
	let bridge: JsObject = env.run_script(
		"(() => { \
			const controller = new AbortController(); \
			return { signal: controller.signal, abort: () => controller.abort() }; \
		})()",
	)?;
	let signal = bridge.get_named_property::<JsObject>("signal")?;
	let abort = bridge.get_named_property::<JsFunction>("abort")?;
	let mut abort = abort.create_threadsafe_function(
		0,
		|_ctx: ThreadSafeCallContext<()>| Ok(Vec::<napi::JsUnknown>::new()),
	)?;
	abort.unref(&env)?;

	Ok((signal, abort))
}

fn js_http_request_to_core_request(request: JsHttpRequest) -> napi::Result<CoreRequest> {
	CoreRequest::from_parts(
		&request.method,
		&request.uri,
		request.headers.unwrap_or_default(),
		request.body.map(|body| body.to_vec()).unwrap_or_default(),
	)
	.map_err(napi_anyhow_error)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mark_started_requires_ready() {
		let shared = ActorContextShared::default();

		assert!(shared.mark_started().is_err());
		assert!(!shared.is_started());

		shared.mark_ready();
		assert!(shared.mark_started().is_ok());
		assert!(shared.is_started());

		shared.mark_ready();
		assert!(shared.mark_started().is_ok());
	}

	#[test]
	fn reset_runtime_state_clears_end_reason_and_lifecycle_flags() {
		let shared = ActorContextShared::default();

		shared.mark_ready();
		shared.mark_started().expect("started should succeed once ready");
		shared.set_end_reason(EndReason::Sleep);
		assert!(shared.has_end_reason());
		assert!(shared.is_ready());
		assert!(shared.is_started());

		shared.reset_runtime_state();

		assert!(!shared.has_end_reason());
		assert!(!shared.is_ready());
		assert!(!shared.is_started());
	}
}
