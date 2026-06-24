use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc;

use anyhow::{Result, anyhow, bail};
use serde::de::DeserializeOwned;

use crate::{AbiResult, AbiStatus, BorrowedBuf, HostVtable, OwnedBuf};

pub type PortableBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReplyToken(pub u64);

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RequestSaveOpts {
	pub immediate: bool,
	pub max_wait_ms: Option<u32>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct KeepAwakeToken {
	pub token: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ActorIdentity {
	pub actor_id: String,
	pub name: String,
	pub key: String,
	pub region: String,
	pub input: Option<Vec<u8>>,
	pub has_state: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ConnInfo {
	pub id: String,
	pub params: Vec<u8>,
	pub state: Vec<u8>,
	pub is_hibernatable: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnListResponse {
	pub conns: Vec<ConnInfo>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnDisconnectRequest {
	pub conn_ids: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnSendRequest {
	pub conn_id: String,
	pub name: String,
	pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct KvListOpts {
	pub reverse: bool,
	pub limit: Option<u32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct KvEntry {
	pub key: Vec<u8>,
	pub value: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvKeyRequest {
	pub key: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvKeysRequest {
	pub keys: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvEntriesRequest {
	pub entries: Vec<KvEntry>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvRangeRequest {
	pub start: Vec<u8>,
	pub end: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvListPrefixRequest {
	pub prefix: Vec<u8>,
	pub opts: KvListOpts,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvListRangeRequest {
	pub start: Vec<u8>,
	pub end: Vec<u8>,
	pub opts: KvListOpts,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvGetResponse {
	pub value: Option<Vec<u8>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvValuesResponse {
	pub values: Vec<Option<Vec<u8>>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KvListResponse {
	pub entries: Vec<KvEntry>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScheduleActionRequest {
	pub delay_ms: Option<u64>,
	pub timestamp_ms: Option<i64>,
	pub action_name: String,
	pub args: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScheduleAlarmRequest {
	pub timestamp_ms: Option<i64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ScheduledEvent {
	pub event_id: String,
	pub timestamp_ms: i64,
	pub action_name: String,
	pub args: Option<Vec<u8>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScheduledEventsResponse {
	pub events: Vec<ScheduledEvent>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConnPreflightWire {
	conn: ConnInfo,
	params: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConnOpenWire {
	conn: ConnInfo,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ConnClosedWire {
	conn: ConnInfo,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SubscribeWire {
	conn: ConnInfo,
	event_name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct QueueSendWire {
	name: String,
	body: Vec<u8>,
	conn: ConnInfo,
	request: Vec<u8>,
	wait: bool,
	timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WsOpenPayload {
	pub conn: ConnInfo,
	pub request: Option<Vec<u8>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueueSendResponse {
	pub status: String,
	pub response: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum Event {
	Action {
		name: String,
		args: Vec<u8>,
		reply: ReplyToken,
	},
	Http {
		request: Vec<u8>,
		reply: ReplyToken,
	},
	Subscribe {
		conn: ConnInfo,
		event_name: String,
		reply: ReplyToken,
	},
	QueueSend {
		name: String,
		body: Vec<u8>,
		conn: ConnInfo,
		request: Vec<u8>,
		wait: bool,
		timeout_ms: Option<u64>,
		reply: ReplyToken,
	},
	WebSocketOpen {
		conn: ConnInfo,
		request: Option<Vec<u8>>,
		reply: ReplyToken,
	},
	ConnPreflight {
		conn: ConnInfo,
		params: Vec<u8>,
		reply: ReplyToken,
	},
	ConnOpen {
		conn: ConnInfo,
		reply: ReplyToken,
	},
	ConnClosed {
		conn: ConnInfo,
	},
	SerializeState {
		reply: ReplyToken,
	},
	Sleep {
		reply: ReplyToken,
	},
	Destroy {
		reply: ReplyToken,
	},
}

impl Event {
	pub fn reply_token(&self) -> Option<ReplyToken> {
		match self {
			Event::Action { reply, .. }
			| Event::Http { reply, .. }
			| Event::Subscribe { reply, .. }
			| Event::QueueSend { reply, .. }
			| Event::WebSocketOpen { reply, .. }
			| Event::ConnPreflight { reply, .. }
			| Event::ConnOpen { reply, .. }
			| Event::SerializeState { reply }
			| Event::Sleep { reply }
			| Event::Destroy { reply } => Some(*reply),
			Event::ConnClosed { .. } => None,
		}
	}
}

pub trait PortableActorBackend: Send + Sync {
	fn next_event(&self) -> PortableBoxFuture<'_, Result<Option<Event>>>;
	fn reply_ok(&self, token: ReplyToken, payload: Vec<u8>) -> Result<()>;
	fn reply_err(&self, token: ReplyToken, message: String) -> Result<()>;
	fn startup_ready(&self, result: Result<()>) -> Result<()>;
	fn broadcast(&self, name: String, payload: Vec<u8>) -> Result<()>;
	fn actor_id(&self) -> Result<String>;
	fn name(&self) -> Result<String>;
	fn key(&self) -> Result<String>;
	fn region(&self) -> Result<String>;
	fn input(&self) -> Result<Option<Vec<u8>>>;
	fn has_state(&self) -> Result<bool>;
	fn state(&self) -> Result<Vec<u8>>;
	fn set_state(&self, state: Vec<u8>) -> Result<()>;
	fn save_state(&self, state: Vec<u8>) -> PortableBoxFuture<'_, Result<()>>;
	fn request_save(&self, opts: RequestSaveOpts) -> Result<()>;
	fn request_save_and_wait(&self, opts: RequestSaveOpts) -> PortableBoxFuture<'_, Result<()>>;
	fn sleep(&self) -> Result<()>;
	fn actor_aborted(&self) -> Result<bool>;
	fn wait_for_actor_abort(&self) -> PortableBoxFuture<'_, Result<()>>;
	fn keep_awake_enter(&self) -> Result<KeepAwakeToken>;
	fn keep_awake_exit(&self, token: KeepAwakeToken) -> Result<()>;
	fn keep_awake_count(&self) -> Result<usize>;
	fn kv_get(&self, key: Vec<u8>) -> PortableBoxFuture<'_, Result<Option<Vec<u8>>>>;
	fn kv_put(&self, key: Vec<u8>, value: Vec<u8>) -> PortableBoxFuture<'_, Result<()>>;
	fn kv_delete(&self, key: Vec<u8>) -> PortableBoxFuture<'_, Result<()>>;
	fn kv_batch_get(
		&self,
		keys: Vec<Vec<u8>>,
	) -> PortableBoxFuture<'_, Result<Vec<Option<Vec<u8>>>>>;
	fn kv_batch_put(&self, entries: Vec<KvEntry>) -> PortableBoxFuture<'_, Result<()>>;
	fn kv_batch_delete(&self, keys: Vec<Vec<u8>>) -> PortableBoxFuture<'_, Result<()>>;
	fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> PortableBoxFuture<'_, Result<()>>;
	fn kv_list_prefix(
		&self,
		prefix: Vec<u8>,
		opts: KvListOpts,
	) -> PortableBoxFuture<'_, Result<Vec<KvEntry>>>;
	fn kv_list_range(
		&self,
		start: Vec<u8>,
		end: Vec<u8>,
		opts: KvListOpts,
	) -> PortableBoxFuture<'_, Result<Vec<KvEntry>>>;
	fn schedule_after_ms(
		&self,
		delay_ms: u64,
		action_name: String,
		args: Vec<u8>,
	) -> PortableBoxFuture<'_, Result<()>>;
	fn schedule_at_ms(
		&self,
		timestamp_ms: i64,
		action_name: String,
		args: Vec<u8>,
	) -> PortableBoxFuture<'_, Result<()>>;
	fn set_alarm(&self, timestamp_ms: Option<i64>) -> PortableBoxFuture<'_, Result<()>>;
	fn scheduled_events(&self) -> PortableBoxFuture<'_, Result<Vec<ScheduledEvent>>>;
	fn conn_list(&self) -> PortableBoxFuture<'_, Result<Vec<ConnInfo>>>;
	fn disconnect_conn(&self, conn_id: String) -> PortableBoxFuture<'_, Result<()>>;
	fn disconnect_conns(&self, conn_ids: Vec<String>) -> PortableBoxFuture<'_, Result<()>>;
	fn send(&self, conn_id: String, name: String, payload: Vec<u8>) -> Result<()>;
	fn ack_hibernatable_websocket_message(
		&self,
		gateway_id: Vec<u8>,
		request_id: Vec<u8>,
		server_message_index: u16,
	) -> Result<()>;
	fn sql_is_enabled(&self) -> bool;
	fn db_exec<'a>(&'a self, sql: &'a str) -> PortableBoxFuture<'a, Result<Vec<u8>>>;
	fn db_query<'a>(
		&'a self,
		sql: &'a str,
		params: Option<Vec<u8>>,
	) -> PortableBoxFuture<'a, Result<Vec<u8>>>;
	fn db_run<'a>(
		&'a self,
		sql: &'a str,
		params: Option<Vec<u8>>,
	) -> PortableBoxFuture<'a, Result<()>>;
}

/// Backend-portable actor context.
///
/// This type is deliberately separate from RivetKit's engine-side
/// `ActorContext`: a dylib actor cannot link `rivetkit-core`, so the actor-facing
/// API must be neutral bytes, ids, and enums instead of engine handles. Use it
/// when one actor source must run both in-process and as a `cdylib`. The tradeoff
/// is a small dispatch/marshalling cost and a lower-level API than the native
/// engine context. TypeScript app contexts are unaffected.
#[derive(Clone)]
pub enum Backend {
	/// In-process backend supplied by rivetkit-core.
	///
	/// `rivet-actor-plugin-abi` cannot name rivetkit-core's concrete
	/// `NativeBackend` without creating a dependency cycle, so the enum variant
	/// stores the neutral backend trait behind an `Arc`.
	Native(Arc<dyn PortableActorBackend>),
	/// FFI backend used by actors loaded from a `cdylib`.
	Dylib(DylibBackend),
}

impl Backend {
	fn as_portable(&self) -> &dyn PortableActorBackend {
		match self {
			Backend::Native(backend) => backend.as_ref(),
			Backend::Dylib(backend) => backend,
		}
	}
}

#[derive(Clone)]
pub struct PortableActorCtx {
	backend: Backend,
}

impl PortableActorCtx {
	pub fn new(backend: impl PortableActorBackend + 'static) -> Self {
		Self {
			backend: Backend::Native(Arc::new(backend)),
		}
	}

	pub fn from_backend(backend: Backend) -> Self {
		Self { backend }
	}

	pub fn new_dylib(backend: DylibBackend) -> Self {
		Self {
			backend: Backend::Dylib(backend),
		}
	}

	fn backend(&self) -> &dyn PortableActorBackend {
		self.backend.as_portable()
	}

	pub async fn next_event(&self) -> Result<Option<Event>> {
		self.backend().next_event().await
	}

	pub fn reply_ok(&self, token: ReplyToken, payload: Vec<u8>) -> Result<()> {
		self.backend().reply_ok(token, payload)
	}

	pub fn reply_err(&self, token: ReplyToken, message: impl Into<String>) -> Result<()> {
		self.backend().reply_err(token, message.into())
	}

	pub fn startup_ready(&self, result: Result<()>) -> Result<()> {
		self.backend().startup_ready(result)
	}

	pub fn broadcast(&self, name: impl Into<String>, payload: Vec<u8>) -> Result<()> {
		self.backend().broadcast(name.into(), payload)
	}

	pub fn actor_id(&self) -> Result<String> {
		self.backend().actor_id()
	}

	pub fn name(&self) -> Result<String> {
		self.backend().name()
	}

	pub fn key(&self) -> Result<String> {
		self.backend().key()
	}

	pub fn region(&self) -> Result<String> {
		self.backend().region()
	}

	pub fn input(&self) -> Result<Option<Vec<u8>>> {
		self.backend().input()
	}

	pub fn has_state(&self) -> Result<bool> {
		self.backend().has_state()
	}

	pub fn state(&self) -> Result<Vec<u8>> {
		self.backend().state()
	}

	pub fn set_state(&self, state: Vec<u8>) -> Result<()> {
		self.backend().set_state(state)
	}

	pub async fn save_state(&self, state: Vec<u8>) -> Result<()> {
		self.backend().save_state(state).await
	}

	pub fn request_save(&self, opts: RequestSaveOpts) -> Result<()> {
		self.backend().request_save(opts)
	}

	pub async fn request_save_and_wait(&self, opts: RequestSaveOpts) -> Result<()> {
		self.backend().request_save_and_wait(opts).await
	}

	pub fn sleep(&self) -> Result<()> {
		self.backend().sleep()
	}

	pub fn actor_aborted(&self) -> Result<bool> {
		self.backend().actor_aborted()
	}

	pub async fn wait_for_actor_abort(&self) -> Result<()> {
		self.backend().wait_for_actor_abort().await
	}

	pub fn keep_awake_region(&self) -> Result<PortableKeepAwakeRegion> {
		let token = self.backend().keep_awake_enter()?;
		Ok(PortableKeepAwakeRegion {
			backend: self.backend.clone(),
			token: Some(token),
		})
	}

	pub async fn keep_awake<F>(&self, future: F) -> Result<F::Output>
	where
		F: Future,
	{
		let _guard = self.keep_awake_region()?;
		Ok(future.await)
	}

	pub fn keep_awake_count(&self) -> Result<usize> {
		self.backend().keep_awake_count()
	}

	pub async fn kv_get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
		self.backend().kv_get(key).await
	}

	pub async fn kv_put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
		self.backend().kv_put(key, value).await
	}

	pub async fn kv_delete(&self, key: Vec<u8>) -> Result<()> {
		self.backend().kv_delete(key).await
	}

	pub async fn kv_batch_get(&self, keys: Vec<Vec<u8>>) -> Result<Vec<Option<Vec<u8>>>> {
		self.backend().kv_batch_get(keys).await
	}

	pub async fn kv_batch_put(&self, entries: Vec<KvEntry>) -> Result<()> {
		self.backend().kv_batch_put(entries).await
	}

	pub async fn kv_batch_delete(&self, keys: Vec<Vec<u8>>) -> Result<()> {
		self.backend().kv_batch_delete(keys).await
	}

	pub async fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> Result<()> {
		self.backend().kv_delete_range(start, end).await
	}

	pub async fn kv_list_prefix(&self, prefix: Vec<u8>, opts: KvListOpts) -> Result<Vec<KvEntry>> {
		self.backend().kv_list_prefix(prefix, opts).await
	}

	pub async fn kv_list_range(
		&self,
		start: Vec<u8>,
		end: Vec<u8>,
		opts: KvListOpts,
	) -> Result<Vec<KvEntry>> {
		self.backend().kv_list_range(start, end, opts).await
	}

	pub async fn schedule_after_ms(
		&self,
		delay_ms: u64,
		action_name: impl Into<String>,
		args: Vec<u8>,
	) -> Result<()> {
		self.backend()
			.schedule_after_ms(delay_ms, action_name.into(), args)
			.await
	}

	pub async fn after(
		&self,
		delay_ms: u64,
		action_name: impl Into<String>,
		args: Vec<u8>,
	) -> Result<()> {
		self.schedule_after_ms(delay_ms, action_name, args).await
	}

	pub async fn schedule_at_ms(
		&self,
		timestamp_ms: i64,
		action_name: impl Into<String>,
		args: Vec<u8>,
	) -> Result<()> {
		self.backend()
			.schedule_at_ms(timestamp_ms, action_name.into(), args)
			.await
	}

	pub async fn at(
		&self,
		timestamp_ms: i64,
		action_name: impl Into<String>,
		args: Vec<u8>,
	) -> Result<()> {
		self.schedule_at_ms(timestamp_ms, action_name, args).await
	}

	pub async fn set_alarm(&self, timestamp_ms: Option<i64>) -> Result<()> {
		self.backend().set_alarm(timestamp_ms).await
	}

	pub async fn scheduled_events(&self) -> Result<Vec<ScheduledEvent>> {
		self.backend().scheduled_events().await
	}

	pub async fn conn_list(&self) -> Result<Vec<ConnInfo>> {
		self.backend().conn_list().await
	}

	pub async fn disconnect_conn(&self, conn_id: impl Into<String>) -> Result<()> {
		self.backend().disconnect_conn(conn_id.into()).await
	}

	pub async fn disconnect_conns(&self, conn_ids: Vec<String>) -> Result<()> {
		self.backend().disconnect_conns(conn_ids).await
	}

	pub fn send(
		&self,
		conn_id: impl Into<String>,
		name: impl Into<String>,
		payload: Vec<u8>,
	) -> Result<()> {
		self.backend().send(conn_id.into(), name.into(), payload)
	}

	pub fn ack_hibernatable_websocket_message(
		&self,
		gateway_id: impl Into<Vec<u8>>,
		request_id: impl Into<Vec<u8>>,
		server_message_index: u16,
	) -> Result<()> {
		self.backend().ack_hibernatable_websocket_message(
			gateway_id.into(),
			request_id.into(),
			server_message_index,
		)
	}

	pub fn sql_is_enabled(&self) -> bool {
		self.backend().sql_is_enabled()
	}

	pub async fn db_exec(&self, sql: &str) -> Result<Vec<u8>> {
		self.backend().db_exec(sql).await
	}

	pub async fn db_query(&self, sql: &str, params: Option<Vec<u8>>) -> Result<Vec<u8>> {
		self.backend().db_query(sql, params).await
	}

	pub async fn db_run(&self, sql: &str, params: Option<Vec<u8>>) -> Result<()> {
		self.backend().db_run(sql, params).await
	}
}

pub struct PortableKeepAwakeRegion {
	backend: Backend,
	token: Option<KeepAwakeToken>,
}

impl Drop for PortableKeepAwakeRegion {
	fn drop(&mut self) {
		if let Some(token) = self.token.take() {
			let _ = self.backend.as_portable().keep_awake_exit(token);
		}
	}
}

struct SendResult(AbiResult);
unsafe impl Send for SendResult {}

extern "C" fn complete_to_channel(user_data: *mut c_void, result: AbiResult) {
	let _ = std::panic::catch_unwind(|| unsafe {
		let tx = Box::from_raw(user_data as *mut mpsc::Sender<SendResult>);
		let _ = tx.send(SendResult(result));
	});
}

#[derive(Clone, Copy)]
struct SendUserData(*mut c_void);
unsafe impl Send for SendUserData {}

fn abi_result_to_bytes(result: AbiResult) -> Result<Vec<u8>> {
	let status = result.status;
	let payload = unsafe { result.payload.into_vec() };
	match status {
		AbiStatus::Ok => Ok(payload),
		AbiStatus::Err => Err(anyhow!("{}", String::from_utf8_lossy(&payload))),
		AbiStatus::Panic => bail!("host operation panicked"),
		AbiStatus::Cancelled => bail!("host operation cancelled"),
		AbiStatus::ChannelClosed => bail!("host event stream closed"),
	}
}

fn encode_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ciborium::into_writer(value, &mut out)?;
	Ok(out)
}

fn decode_cbor<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
	Ok(ciborium::from_reader(std::io::Cursor::new(bytes))?)
}

fn call_async<F>(submit: F) -> PortableBoxFuture<'static, Result<AbiResult>>
where
	F: FnOnce(crate::CompletionFn, *mut c_void) + Send + 'static,
{
	Box::pin(async move {
		let (tx, rx) = mpsc::channel::<SendResult>();
		let user_data = SendUserData(Box::into_raw(Box::new(tx)) as *mut c_void);
		submit(complete_to_channel, user_data.0);
		rx.recv()
			.map(|r| r.0)
			.map_err(|_| anyhow!("host completion channel closed"))
	})
}

#[derive(Clone, Copy)]
struct SendVtable(HostVtable);
unsafe impl Send for SendVtable {}
unsafe impl Sync for SendVtable {}

impl SendVtable {
	fn next_event(&self, done: crate::CompletionFn, user_data: *mut c_void) {
		(self.0.next_event)(self.0.ctx, done, user_data);
	}
}

pub struct DylibBackend {
	host: HostVtable,
}

unsafe impl Send for DylibBackend {}
unsafe impl Sync for DylibBackend {}

impl DylibBackend {
	/// Build a dylib backend from the host vtable received by `rivet_actor_run`.
	///
	/// The backend clones the opaque host context and releases it on drop, so it
	/// may outlive the synchronous `run` call that provided the vtable.
	///
	/// # Safety
	/// `host` must point to a valid same-version [`HostVtable`].
	pub unsafe fn from_host_vtable(host: &HostVtable) -> Self {
		let host = *host;
		(host.ctx_clone)(host.ctx);
		Self { host }
	}

	fn complete_bytes<F>(&self, submit: F) -> PortableBoxFuture<'static, Result<Vec<u8>>>
	where
		F: FnOnce(HostVtable, crate::CompletionFn, *mut c_void) + Send + 'static,
	{
		let host = SendVtable(self.host);
		Box::pin(async move {
			let result = call_async(move |done, user_data| submit(host.0, done, user_data)).await?;
			abi_result_to_bytes(result)
		})
	}

	fn identity(&self) -> Result<ActorIdentity> {
		let buf = (self.host.actor_identity)(self.host.ctx);
		decode_cbor(&unsafe { buf.into_vec() })
	}
}

impl Clone for DylibBackend {
	fn clone(&self) -> Self {
		(self.host.ctx_clone)(self.host.ctx);
		Self { host: self.host }
	}
}

impl Drop for DylibBackend {
	fn drop(&mut self) {
		(self.host.ctx_release)(self.host.ctx);
	}
}

impl PortableActorBackend for DylibBackend {
	fn next_event(&self) -> PortableBoxFuture<'_, Result<Option<Event>>> {
		let host = SendVtable(self.host);
		Box::pin(async move {
			let result =
				call_async(move |done, user_data| host.next_event(done, user_data)).await?;
			if result.status == AbiStatus::ChannelClosed {
				unsafe { result.payload.free_self() };
				return Ok(None);
			}
			decode_event_frame(&abi_result_to_bytes(result)?).map(Some)
		})
	}

	fn reply_ok(&self, token: ReplyToken, payload: Vec<u8>) -> Result<()> {
		match (self.host.reply_ok)(self.host.ctx, token.0, OwnedBuf::from_vec(payload)) {
			AbiStatus::Ok => Ok(()),
			other => Err(anyhow!("reply_ok failed with {other:?}")),
		}
	}

	fn reply_err(&self, token: ReplyToken, message: String) -> Result<()> {
		match (self.host.reply_err)(
			self.host.ctx,
			token.0,
			OwnedBuf::from_vec(message.into_bytes()),
		) {
			AbiStatus::Ok => Ok(()),
			other => Err(anyhow!("reply_err failed with {other:?}")),
		}
	}

	fn startup_ready(&self, result: Result<()>) -> Result<()> {
		match result {
			Ok(()) => (self.host.startup_ready)(self.host.ctx, 1, BorrowedBuf::from_slice(&[])),
			Err(err) => {
				let msg = err.to_string();
				(self.host.startup_ready)(
					self.host.ctx,
					0,
					BorrowedBuf::from_slice(msg.as_bytes()),
				);
			}
		}
		Ok(())
	}

	fn broadcast(&self, name: String, payload: Vec<u8>) -> Result<()> {
		match (self.host.broadcast)(
			self.host.ctx,
			OwnedBuf::from_vec(name.into_bytes()),
			OwnedBuf::from_vec(payload),
		) {
			AbiStatus::Ok => Ok(()),
			other => Err(anyhow!("broadcast failed with {other:?}")),
		}
	}

	fn actor_id(&self) -> Result<String> {
		Ok(self.identity()?.actor_id)
	}

	fn name(&self) -> Result<String> {
		Ok(self.identity()?.name)
	}

	fn key(&self) -> Result<String> {
		Ok(self.identity()?.key)
	}

	fn region(&self) -> Result<String> {
		Ok(self.identity()?.region)
	}

	fn input(&self) -> Result<Option<Vec<u8>>> {
		Ok(self.identity()?.input)
	}

	fn has_state(&self) -> Result<bool> {
		Ok(self.identity()?.has_state)
	}

	fn state(&self) -> Result<Vec<u8>> {
		let buf = (self.host.state_get)(self.host.ctx);
		Ok(unsafe { buf.into_vec() })
	}

	fn set_state(&self, state: Vec<u8>) -> Result<()> {
		match (self.host.state_set)(self.host.ctx, OwnedBuf::from_vec(state)) {
			AbiStatus::Ok => Ok(()),
			other => Err(anyhow!("set_state failed with {other:?}")),
		}
	}

	fn save_state(&self, state: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		let fut = self.complete_bytes(move |host, done, user_data| {
			(host.state_save)(host.ctx, OwnedBuf::from_vec(state), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn request_save(&self, opts: RequestSaveOpts) -> Result<()> {
		let (has_max_wait, max_wait_ms) = match opts.max_wait_ms {
			Some(max_wait_ms) => (1, max_wait_ms),
			None => (0, 0),
		};
		match (self.host.request_save)(
			self.host.ctx,
			u8::from(opts.immediate),
			has_max_wait,
			max_wait_ms,
		) {
			AbiStatus::Ok => Ok(()),
			other => Err(anyhow!("request_save failed with {other:?}")),
		}
	}

	fn request_save_and_wait(&self, opts: RequestSaveOpts) -> PortableBoxFuture<'_, Result<()>> {
		let (has_max_wait, max_wait_ms) = match opts.max_wait_ms {
			Some(max_wait_ms) => (1, max_wait_ms),
			None => (0, 0),
		};
		let fut = self.complete_bytes(move |host, done, user_data| {
			(host.request_save_and_wait)(
				host.ctx,
				u8::from(opts.immediate),
				has_max_wait,
				max_wait_ms,
				done,
				user_data,
			);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn sleep(&self) -> Result<()> {
		abi_result_to_bytes((self.host.sleep)(self.host.ctx)).map(|_| ())
	}

	fn actor_aborted(&self) -> Result<bool> {
		Ok((self.host.actor_aborted)(self.host.ctx) != 0)
	}

	fn wait_for_actor_abort(&self) -> PortableBoxFuture<'_, Result<()>> {
		let fut = self.complete_bytes(move |host, done, user_data| {
			(host.wait_actor_abort)(host.ctx, done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn keep_awake_enter(&self) -> Result<KeepAwakeToken> {
		let bytes = abi_result_to_bytes((self.host.keep_awake_enter)(self.host.ctx))?;
		decode_cbor(&bytes)
	}

	fn keep_awake_exit(&self, token: KeepAwakeToken) -> Result<()> {
		match (self.host.keep_awake_exit)(self.host.ctx, token.token) {
			AbiStatus::Ok => Ok(()),
			other => Err(anyhow!("keep_awake_exit failed with {other:?}")),
		}
	}

	fn keep_awake_count(&self) -> Result<usize> {
		Ok((self.host.keep_awake_count)(self.host.ctx) as usize)
	}

	fn kv_get(&self, key: Vec<u8>) -> PortableBoxFuture<'_, Result<Option<Vec<u8>>>> {
		let request = encode_cbor(&KvKeyRequest { key });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_get)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			let response: KvGetResponse = decode_cbor(&fut.await?)?;
			Ok(response.value)
		})
	}

	fn kv_put(&self, key: Vec<u8>, value: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&KvEntriesRequest {
			entries: vec![KvEntry { key, value }],
		});
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_put)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn kv_delete(&self, key: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&KvKeysRequest { keys: vec![key] });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_delete)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn kv_batch_get(
		&self,
		keys: Vec<Vec<u8>>,
	) -> PortableBoxFuture<'_, Result<Vec<Option<Vec<u8>>>>> {
		let request = encode_cbor(&KvKeysRequest { keys });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_batch_get)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			let response: KvValuesResponse = decode_cbor(&fut.await?)?;
			Ok(response.values)
		})
	}

	fn kv_batch_put(&self, entries: Vec<KvEntry>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&KvEntriesRequest { entries });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_batch_put)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn kv_batch_delete(&self, keys: Vec<Vec<u8>>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&KvKeysRequest { keys });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_batch_delete)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn kv_delete_range(&self, start: Vec<u8>, end: Vec<u8>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&KvRangeRequest { start, end });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_delete_range)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn kv_list_prefix(
		&self,
		prefix: Vec<u8>,
		opts: KvListOpts,
	) -> PortableBoxFuture<'_, Result<Vec<KvEntry>>> {
		let request = encode_cbor(&KvListPrefixRequest { prefix, opts });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_list_prefix)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			let response: KvListResponse = decode_cbor(&fut.await?)?;
			Ok(response.entries)
		})
	}

	fn kv_list_range(
		&self,
		start: Vec<u8>,
		end: Vec<u8>,
		opts: KvListOpts,
	) -> PortableBoxFuture<'_, Result<Vec<KvEntry>>> {
		let request = encode_cbor(&KvListRangeRequest { start, end, opts });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.kv_list_range)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			let response: KvListResponse = decode_cbor(&fut.await?)?;
			Ok(response.entries)
		})
	}

	fn schedule_after_ms(
		&self,
		delay_ms: u64,
		action_name: String,
		args: Vec<u8>,
	) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&ScheduleActionRequest {
			delay_ms: Some(delay_ms),
			timestamp_ms: None,
			action_name,
			args,
		});
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.schedule_after)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn schedule_at_ms(
		&self,
		timestamp_ms: i64,
		action_name: String,
		args: Vec<u8>,
	) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&ScheduleActionRequest {
			delay_ms: None,
			timestamp_ms: Some(timestamp_ms),
			action_name,
			args,
		});
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.schedule_at)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn set_alarm(&self, timestamp_ms: Option<i64>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&ScheduleAlarmRequest { timestamp_ms });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.set_alarm)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn scheduled_events(&self) -> PortableBoxFuture<'_, Result<Vec<ScheduledEvent>>> {
		let request = encode_cbor(&());
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.scheduled_events)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			let response: ScheduledEventsResponse = decode_cbor(&fut.await?)?;
			Ok(response.events)
		})
	}

	fn conn_list(&self) -> PortableBoxFuture<'_, Result<Vec<ConnInfo>>> {
		let request = encode_cbor(&());
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.conn_list)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			let response: ConnListResponse = decode_cbor(&fut.await?)?;
			Ok(response.conns)
		})
	}

	fn disconnect_conn(&self, conn_id: String) -> PortableBoxFuture<'_, Result<()>> {
		self.disconnect_conns(vec![conn_id])
	}

	fn disconnect_conns(&self, conn_ids: Vec<String>) -> PortableBoxFuture<'_, Result<()>> {
		let request = encode_cbor(&ConnDisconnectRequest { conn_ids });
		let fut = self.complete_bytes(move |host, done, user_data| {
			let request = match request {
				Ok(request) => request,
				Err(error) => {
					done(
						user_data,
						AbiResult::err(OwnedBuf::from_vec(format!("{error:#}").into_bytes())),
					);
					return;
				}
			};
			(host.conn_disconnect)(host.ctx, OwnedBuf::from_vec(request), done, user_data);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}

	fn send(&self, conn_id: String, name: String, payload: Vec<u8>) -> Result<()> {
		let request = encode_cbor(&ConnSendRequest {
			conn_id,
			name,
			payload,
		})?;
		abi_result_to_bytes((self.host.conn_send)(
			self.host.ctx,
			OwnedBuf::from_vec(request),
		))?;
		Ok(())
	}

	fn ack_hibernatable_websocket_message(
		&self,
		gateway_id: Vec<u8>,
		request_id: Vec<u8>,
		server_message_index: u16,
	) -> Result<()> {
		abi_result_to_bytes((self.host.hibernatable_ws_ack)(
			self.host.ctx,
			OwnedBuf::from_vec(gateway_id),
			OwnedBuf::from_vec(request_id),
			server_message_index,
		))?;
		Ok(())
	}

	fn sql_is_enabled(&self) -> bool {
		(self.host.sql_is_enabled)(self.host.ctx) != 0
	}

	fn db_exec<'a>(&'a self, sql: &'a str) -> PortableBoxFuture<'a, Result<Vec<u8>>> {
		let sql = sql.as_bytes().to_vec();
		self.complete_bytes(move |host, done, user_data| {
			(host.db_exec)(host.ctx, OwnedBuf::from_vec(sql), done, user_data);
		})
	}

	fn db_query<'a>(
		&'a self,
		sql: &'a str,
		params: Option<Vec<u8>>,
	) -> PortableBoxFuture<'a, Result<Vec<u8>>> {
		let sql = sql.as_bytes().to_vec();
		let params = params.unwrap_or_default();
		self.complete_bytes(move |host, done, user_data| {
			(host.db_query)(
				host.ctx,
				OwnedBuf::from_vec(sql),
				OwnedBuf::from_vec(params),
				done,
				user_data,
			);
		})
	}

	fn db_run<'a>(
		&'a self,
		sql: &'a str,
		params: Option<Vec<u8>>,
	) -> PortableBoxFuture<'a, Result<()>> {
		let sql = sql.as_bytes().to_vec();
		let params = params.unwrap_or_default();
		let fut = self.complete_bytes(move |host, done, user_data| {
			(host.db_run)(
				host.ctx,
				OwnedBuf::from_vec(sql),
				OwnedBuf::from_vec(params),
				done,
				user_data,
			);
		});
		Box::pin(async move {
			fut.await?;
			Ok(())
		})
	}
}

pub fn encode_action_payload(name: &str, args: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(4 + name.len() + args.len());
	out.extend_from_slice(&(name.len() as u32).to_le_bytes());
	out.extend_from_slice(name.as_bytes());
	out.extend_from_slice(args);
	out
}

pub fn decode_action_payload(payload: &[u8]) -> Result<(String, Vec<u8>)> {
	let name_len = payload
		.get(0..4)
		.ok_or_else(|| anyhow!("action payload missing name length"))
		.and_then(|b| {
			b.try_into()
				.map(u32::from_le_bytes)
				.map_err(|_| anyhow!("invalid action name length"))
		})? as usize;
	let rest = payload
		.get(4..)
		.ok_or_else(|| anyhow!("action payload missing body"))?;
	let name = rest
		.get(..name_len)
		.ok_or_else(|| anyhow!("action payload name out of bounds"))?;
	let args = rest
		.get(name_len..)
		.ok_or_else(|| anyhow!("action payload args out of bounds"))?;
	Ok((String::from_utf8(name.to_vec())?, args.to_vec()))
}

pub fn encode_event_frame(tag: u32, token: ReplyToken, payload: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(12 + payload.len());
	out.extend_from_slice(&tag.to_le_bytes());
	out.extend_from_slice(&token.0.to_le_bytes());
	out.extend_from_slice(payload);
	out
}

pub fn encode_conn_preflight_payload(conn: &ConnInfo, params: &[u8]) -> Result<Vec<u8>> {
	let wire = ConnPreflightWire {
		conn: conn.clone(),
		params: params.to_vec(),
	};
	let mut out = Vec::new();
	ciborium::into_writer(&wire, &mut out)?;
	Ok(out)
}

pub fn encode_conn_open_payload(conn: &ConnInfo) -> Result<Vec<u8>> {
	let wire = ConnOpenWire { conn: conn.clone() };
	let mut out = Vec::new();
	ciborium::into_writer(&wire, &mut out)?;
	Ok(out)
}

pub fn encode_conn_closed_payload(conn: &ConnInfo) -> Result<Vec<u8>> {
	let wire = ConnClosedWire { conn: conn.clone() };
	let mut out = Vec::new();
	ciborium::into_writer(&wire, &mut out)?;
	Ok(out)
}

pub fn encode_subscribe_payload(conn: &ConnInfo, event_name: &str) -> Result<Vec<u8>> {
	let wire = SubscribeWire {
		conn: conn.clone(),
		event_name: event_name.to_owned(),
	};
	let mut out = Vec::new();
	ciborium::into_writer(&wire, &mut out)?;
	Ok(out)
}

pub fn encode_queue_send_payload(
	name: &str,
	body: &[u8],
	conn: &ConnInfo,
	request: &[u8],
	wait: bool,
	timeout_ms: Option<u64>,
) -> Result<Vec<u8>> {
	let wire = QueueSendWire {
		name: name.to_owned(),
		body: body.to_vec(),
		conn: conn.clone(),
		request: request.to_vec(),
		wait,
		timeout_ms,
	};
	let mut out = Vec::new();
	ciborium::into_writer(&wire, &mut out)?;
	Ok(out)
}

pub fn encode_ws_open_payload(conn: &ConnInfo, request: Option<&[u8]>) -> Result<Vec<u8>> {
	let wire = WsOpenPayload {
		conn: conn.clone(),
		request: request.map(<[u8]>::to_vec),
	};
	let mut out = Vec::new();
	ciborium::into_writer(&wire, &mut out)?;
	Ok(out)
}

pub fn encode_queue_send_response(status: &str, response: Option<Vec<u8>>) -> Result<Vec<u8>> {
	encode_cbor(&QueueSendResponse {
		status: status.to_owned(),
		response,
	})
}

pub fn decode_event_frame(bytes: &[u8]) -> Result<Event> {
	if bytes.len() < 12 {
		bail!("event frame shorter than header");
	}
	let tag = u32::from_le_bytes(bytes[0..4].try_into()?);
	let token = ReplyToken(u64::from_le_bytes(bytes[4..12].try_into()?));
	let payload = bytes[12..].to_vec();
	let tag =
		crate::AbiEventTag::from_u32(tag).ok_or_else(|| anyhow!("unknown event tag {tag}"))?;
	Ok(match tag {
		crate::AbiEventTag::Action => {
			let (name, args) = decode_action_payload(&payload)?;
			Event::Action {
				name,
				args,
				reply: token,
			}
		}
		crate::AbiEventTag::Http => Event::Http {
			request: payload,
			reply: token,
		},
		crate::AbiEventTag::Subscribe => {
			let wire: SubscribeWire = ciborium::from_reader(std::io::Cursor::new(payload))?;
			Event::Subscribe {
				conn: wire.conn,
				event_name: wire.event_name,
				reply: token,
			}
		}
		crate::AbiEventTag::QueueSend => {
			let wire: QueueSendWire = ciborium::from_reader(std::io::Cursor::new(payload))?;
			Event::QueueSend {
				name: wire.name,
				body: wire.body,
				conn: wire.conn,
				request: wire.request,
				wait: wire.wait,
				timeout_ms: wire.timeout_ms,
				reply: token,
			}
		}
		crate::AbiEventTag::WsOpen => {
			let wire: WsOpenPayload = ciborium::from_reader(std::io::Cursor::new(payload))?;
			Event::WebSocketOpen {
				conn: wire.conn,
				request: wire.request,
				reply: token,
			}
		}
		crate::AbiEventTag::ConnOpen => {
			let wire: ConnOpenWire = ciborium::from_reader(std::io::Cursor::new(payload))?;
			Event::ConnOpen {
				conn: wire.conn,
				reply: token,
			}
		}
		crate::AbiEventTag::ConnPreflight => {
			let wire: ConnPreflightWire = ciborium::from_reader(std::io::Cursor::new(payload))?;
			Event::ConnPreflight {
				conn: wire.conn,
				params: wire.params,
				reply: token,
			}
		}
		crate::AbiEventTag::ConnClosed => {
			let wire: ConnClosedWire = ciborium::from_reader(std::io::Cursor::new(payload))?;
			Event::ConnClosed { conn: wire.conn }
		}
		crate::AbiEventTag::SerializeState => Event::SerializeState { reply: token },
		crate::AbiEventTag::Sleep => Event::Sleep { reply: token },
		crate::AbiEventTag::Destroy => Event::Destroy { reply: token },
	})
}
