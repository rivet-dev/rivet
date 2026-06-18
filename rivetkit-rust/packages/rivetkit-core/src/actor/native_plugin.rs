//! Generic loader for native actor plugins (`dlopen` of a `cdylib`), per the
//! dylib-actor-plugin spec §6.1. RivetKit knows only this generic ABI, not any
//! product-specific symbols. The plugin is resolved by path, its ABI magic/version is
//! verified (refuse-on-mismatch, no fallback), and it is adapted into the
//! existing [`ActorFactory`] boxed-closure entry point.
//!
//! This module is the load/ABI/symbol layer. The host vtable construction and
//! the event adapter (reply slab, grace bridge) build on top of it.
//!
//! ## Event adapter mapping
//!
//! The adapter consumes the core-level [`crate::actor::messages::ActorEvent`]
//! from [`ActorStart::events`] and maps to [`abi::AbiEventTag`] — no dependency
//! on the higher-level `rivetkit` crate's `RuntimeEvent<A>` is needed:
//!
//! | `ActorEvent`            | `AbiEventTag` / handling                       |
//! |-------------------------|------------------------------------------------|
//! | `Action`                | `Action` (reply: ok/err)                       |
//! | `HttpRequest`           | `Http` (reply)                                 |
//! | `SubscribeRequest`      | `Subscribe` (reply: allow)                     |
//! | `ConnectionOpen`        | `ConnOpen` (reply: accept)                     |
//! | `ConnectionClosed`      | `ConnClosed` (no reply)                        |
//! | `QueueSend`             | `QueueSend` (reply)                            |
//! | `WebSocketOpen`         | `WsOpen` (reply)                               |
//! | `SerializeState`        | `SerializeState` (reply: actor-state bytes)    |
//! | `RunGracefulCleanup`    | split → `Sleep`/`Destroy` by reason (reply)    |
//! | `FinalizeSleep`/`Destroy` | lifecycle reply, drives VM teardown          |
//! | `DisconnectConn`        | consumed internally (host calls disconnect)    |
//! | `ConnectionPreflight`   | consumed internally                            |
//! | `WorkflowHistory/Replay`| not applicable to native plugins (reply empty) |
//!
//! Reply-bearing events allocate a `reply_token` into a slab that OWNS the
//! `Reply<T>`; draining the slab on exit/cancel drops each `Reply`, firing
//! `Err(DroppedReply)` so callers never hang (spec §6.3).

// FFI glue: `unsafe fn`s here are unsafe in their entirety by design.
#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use futures::future::FutureExt as _;
use libloading::{Library, Symbol};
use parking_lot::Mutex;
use rivet_actor_plugin_abi as abi;
use tokio::runtime::Handle;

use crate::ActorConfig;
use crate::actor::connection::ConnHandle;
use crate::actor::context::{ActorContext, KeepAwakeRegion};
use crate::actor::factory::ActorFactory;
use crate::actor::lifecycle_hooks::ActorEvents;
use crate::actor::messages::{ActorEvent, Request, Response, StateDelta};
use crate::actor::messages::{QueueSendResult, QueueSendStatus};
use crate::actor::state::RequestSaveOpts;
use crate::actor::task_types::ShutdownKind;
use crate::types::{ListOpts, format_actor_key};

// --- Plugin export signatures (must match `abi::symbols` / spec §4.5) ---

type AbiU64Fn = unsafe extern "C" fn() -> u64;
type PluginInitFn = unsafe extern "C" fn(out_err: *mut abi::OwnedBuf) -> *mut c_void;
type FactoryNewFn = unsafe extern "C" fn(
	plugin: *mut c_void,
	config_json: abi::BorrowedBuf,
	sidecar_path: abi::BorrowedBuf,
	out_err: *mut abi::OwnedBuf,
) -> *mut c_void;
type RunFn = unsafe extern "C" fn(
	factory: *mut c_void,
	host: *const abi::HostVtable,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) -> *mut c_void;
type HandleFn = unsafe extern "C" fn(handle: *mut c_void);

/// Opaque plugin-owned handle (plugin/factory/instance). Send+Sync because the
/// plugin owns the pointed-to state and the host only passes it back opaquely.
#[derive(Clone, Copy)]
pub(crate) struct OpaqueHandle(pub(crate) *mut c_void);
unsafe impl Send for OpaqueHandle {}
unsafe impl Sync for OpaqueHandle {}

/// A `dlopen`ed, ABI-verified, initialized plugin. Kept alive for the process
/// lifetime (never unloaded — unloading a dylib with live runtime/threads is
/// unsound). One per unique dylib path.
// `grace_deadline`/`factory_free`/`plugin_shutdown` are retained for lifecycle
// paths not yet wired (host grace-deadline trigger, explicit teardown).
#[allow(dead_code)]
pub(crate) struct LoadedPlugin {
	// Field order matters for drop: handle/symbols before `_lib`. We never drop
	// these in practice (cached for process lifetime), but keep `_lib` last.
	plugin: OpaqueHandle,
	factory_new: FactoryNewFn,
	run: RunFn,
	cancel: HandleFn,
	grace_deadline: HandleFn,
	instance_free: HandleFn,
	factory_free: HandleFn,
	plugin_shutdown: HandleFn,
	_lib: Library,
}

unsafe impl Send for LoadedPlugin {}
unsafe impl Sync for LoadedPlugin {}

#[allow(dead_code)]
impl LoadedPlugin {
	pub(crate) fn factory_new(&self) -> FactoryNewFn {
		self.factory_new
	}
	pub(crate) fn run(&self) -> RunFn {
		self.run
	}
	pub(crate) fn cancel(&self) -> HandleFn {
		self.cancel
	}
	pub(crate) fn grace_deadline(&self) -> HandleFn {
		self.grace_deadline
	}
	pub(crate) fn instance_free(&self) -> HandleFn {
		self.instance_free
	}
	pub(crate) fn factory_free(&self) -> HandleFn {
		self.factory_free
	}
	pub(crate) fn plugin_shutdown(&self) -> HandleFn {
		self.plugin_shutdown
	}
}

fn cache() -> &'static Mutex<HashMap<PathBuf, Arc<LoadedPlugin>>> {
	static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<LoadedPlugin>>>> = OnceLock::new();
	CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Read + free an out-error `OwnedBuf` produced by the plugin, returning its
/// UTF-8 message (lossy). Consumes the buffer.
unsafe fn take_out_err(out: abi::OwnedBuf) -> String {
	if out.len == 0 {
		return String::new();
	}
	let msg = String::from_utf8_lossy(out.as_slice()).into_owned();
	out.free_self();
	msg
}

/// Load (or fetch from cache) the plugin at `path`, verifying its ABI and
/// running `plugin_init` exactly once per path.
pub(crate) fn load_plugin(path: &Path) -> Result<Arc<LoadedPlugin>> {
	let key = path.to_path_buf();
	if let Some(existing) = cache().lock().get(&key).cloned() {
		return Ok(existing);
	}

	// SAFETY: loading an arbitrary dylib runs its initializers; we only load
	// from trusted, host-resolved paths (spec §13). Symbol signatures are
	// fixed by the shared `rivet-actor-plugin-abi` contract.
	let loaded = unsafe { load_uncached(path) }
		.with_context(|| format!("load native actor plugin at {}", path.display()))?;
	let arc = Arc::new(loaded);
	cache().lock().insert(key, arc.clone());
	Ok(arc)
}

unsafe fn sym<T>(lib: &Library, name: &[u8]) -> Result<T>
where
	T: Copy,
{
	let symbol: Symbol<T> = lib
		.get(name)
		.with_context(|| format!("resolve symbol {}", String::from_utf8_lossy(name)))?;
	Ok(*symbol)
}

unsafe fn load_uncached(path: &Path) -> Result<LoadedPlugin> {
	let lib = Library::new(path).context("dlopen")?;

	// ABI magic + version are checked FIRST, before any other call.
	let abi_magic: AbiU64Fn = sym(&lib, abi::symbols::ABI_MAGIC)?;
	let abi_version: AbiU64Fn = sym(&lib, abi::symbols::ABI_VERSION)?;
	let magic = abi_magic();
	if magic != abi::RIVET_ACTOR_ABI_MAGIC {
		bail!(
			"not a rivet actor plugin (magic {magic:#x} != {:#x})",
			abi::RIVET_ACTOR_ABI_MAGIC
		);
	}
	let version = abi_version();
	if version != abi::RIVET_ACTOR_ABI_VERSION {
		bail!(
			"actor plugin ABI v{version}, host expects v{} (same-version lockstep; no fallback)",
			abi::RIVET_ACTOR_ABI_VERSION
		);
	}

	let plugin_init: PluginInitFn = sym(&lib, abi::symbols::PLUGIN_INIT)?;
	let factory_new: FactoryNewFn = sym(&lib, abi::symbols::FACTORY_NEW)?;
	let run: RunFn = sym(&lib, abi::symbols::RUN)?;
	let cancel: HandleFn = sym(&lib, abi::symbols::CANCEL)?;
	let grace_deadline: HandleFn = sym(&lib, abi::symbols::GRACE_DEADLINE)?;
	let instance_free: HandleFn = sym(&lib, abi::symbols::INSTANCE_FREE)?;
	let factory_free: HandleFn = sym(&lib, abi::symbols::FACTORY_FREE)?;
	let plugin_shutdown: HandleFn = sym(&lib, abi::symbols::PLUGIN_SHUTDOWN)?;

	let mut out_err = abi::OwnedBuf::empty();
	let plugin = plugin_init(&mut out_err as *mut _);
	if plugin.is_null() {
		let msg = take_out_err(out_err);
		bail!("rivet_actor_plugin_init failed: {msg}");
	}

	Ok(LoadedPlugin {
		plugin: OpaqueHandle(plugin),
		factory_new,
		run,
		cancel,
		grace_deadline,
		instance_free,
		factory_free,
		plugin_shutdown,
		_lib: lib,
	})
}

/// Create a per-actor-type plugin factory: load the plugin, call `factory_new`
/// with the opaque config envelope + sidecar path, and adapt the result into a
/// RivetKit [`ActorFactory`].
///
/// NOTE: the entry closure (host vtable + event adapter) is implemented in a
/// follow-up step; this establishes the verified load + factory-construction
/// path and the factory handle the entry will `run`.
pub fn build_native_plugin_factory(
	plugin_path: &Path,
	config_json: &str,
	sidecar_path: &str,
	config: ActorConfig,
) -> Result<ActorFactory> {
	let plugin = load_plugin(plugin_path)?;

	// Build the factory handle from the opaque config envelope. The plugin
	// parses the JSON itself (config is opaque to the host).
	let mut out_err = abi::OwnedBuf::empty();
	// SAFETY: borrowed buffers are valid for the duration of this synchronous
	// call only; `factory_new` must copy anything it retains.
	let factory_ptr = unsafe {
		(plugin.factory_new())(
			plugin.plugin.0,
			abi::BorrowedBuf::from_slice(config_json.as_bytes()),
			abi::BorrowedBuf::from_slice(sidecar_path.as_bytes()),
			&mut out_err as *mut _,
		)
	};
	if factory_ptr.is_null() {
		let msg = unsafe { take_out_err(out_err) };
		return Err(anyhow!("rivet_actor_factory_new failed: {msg}"));
	}
	let factory = OpaqueHandle(factory_ptr);

	let plugin_for_entry = plugin.clone();
	let entry = move |start: crate::actor::lifecycle_hooks::ActorStart| {
		let plugin = plugin_for_entry.clone();
		Box::pin(run_native_actor(plugin, factory, start))
			as crate::runtime::RuntimeBoxFuture<Result<()>>
	};

	Ok(ActorFactory::new_with_manual_startup_ready(config, entry))
}

/// Completion callback the plugin invokes when the actor loop exits. Reclaims
/// the boxed oneshot sender, frees the result payload, and signals the host.
struct RunDone {
	status: abi::AbiStatus,
	payload: Vec<u8>,
}

extern "C" fn run_done(user_data: *mut c_void, result: abi::AbiResult) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let tx = Box::from_raw(user_data as *mut tokio::sync::oneshot::Sender<RunDone>);
		let status = result.status;
		let payload = result.payload.into_vec();
		let _ = tx.send(RunDone { status, payload });
	}));
}

/// Tells the plugin to cancel (close its event stream) if the host drops the
/// actor future before it completes. `instance` is taken on the happy path so
/// the guard becomes a no-op once the instance is freed.
struct CancelGuard {
	plugin: Arc<LoadedPlugin>,
	instance: Option<*mut c_void>,
}
unsafe impl Send for CancelGuard {}
impl Drop for CancelGuard {
	fn drop(&mut self) {
		if let Some(instance) = self.instance.take() {
			// Host aborted the actor future (e.g. the sleep/destroy grace
			// deadline elapsed): force VM teardown, then close the event stream.
			// Both are idempotent and the instance is not yet freed.
			unsafe {
				(self.plugin.grace_deadline())(instance);
				(self.plugin.cancel())(instance);
			}
		}
	}
}

/// Drive one native-plugin actor instance: build the host vtable over the
/// actor context, spawn the event adapter, `run` the plugin, await completion.
async fn run_native_actor(
	plugin: Arc<LoadedPlugin>,
	factory: OpaqueHandle,
	start: crate::actor::lifecycle_hooks::ActorStart,
) -> Result<()> {
	let runtime = Handle::current();
	let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<ForwardedEvent>();
	let state = Arc::new(HostCtxState {
		ctx: start.ctx.clone(),
		runtime: runtime.clone(),
		slab: ReplySlab::new(),
		events: tokio::sync::Mutex::new(event_rx),
		startup: Mutex::new(start.startup_ready),
		keep_awake: KeepAwakeStore::new(),
	});

	// Event adapter: drains core ActorEvents -> forwarded stream / inline replies.
	let loop_handle = runtime.spawn(adapter_loop(start.events, event_tx, state.clone()));

	// Host vtable, wrapped `Send` so it can live across the completion await.
	// The plugin copies it synchronously in `run`. `ctx` holds ONE owned Arc ref.
	let vtable = SendVtable(abi::HostVtable {
		abi_version: abi::RIVET_ACTOR_ABI_VERSION,
		ctx: state.clone().into_ctx_ptr(),
		ctx_clone: host_ctx_clone,
		ctx_release: host_ctx_release,
		db_exec: host_db_exec,
		db_query: host_db_query,
		db_run: host_db_run,
		sql_is_enabled: host_sql_is_enabled,
		state_get: host_state_get,
		state_set: host_state_set,
		actor_identity: host_actor_identity,
		state_save: host_state_save,
		request_save: host_request_save,
		request_save_and_wait: host_request_save_and_wait,
		sleep: host_sleep,
		actor_aborted: host_actor_aborted,
		wait_actor_abort: host_wait_actor_abort,
		keep_awake_enter: host_keep_awake_enter,
		keep_awake_exit: host_keep_awake_exit,
		keep_awake_count: host_keep_awake_count,
		kv_get: host_kv_get,
		kv_put: host_kv_put,
		kv_delete: host_kv_delete,
		kv_batch_get: host_kv_batch_get,
		kv_batch_put: host_kv_batch_put,
		kv_batch_delete: host_kv_batch_delete,
		kv_delete_range: host_kv_delete_range,
		kv_list_prefix: host_kv_list_prefix,
		kv_list_range: host_kv_list_range,
		schedule_after: host_schedule_after,
		schedule_at: host_schedule_at,
		set_alarm: host_set_alarm,
		scheduled_events: host_scheduled_events,
		conn_list: host_conn_list,
		conn_disconnect: host_conn_disconnect,
		hibernatable_ws_ack: host_hibernatable_ws_ack,
		conn_send: host_conn_send,
		next_event: host_next_event,
		reply_ok: host_reply_ok,
		reply_err: host_reply_err,
		startup_ready: host_startup_ready,
		broadcast: host_broadcast,
		log: host_log,
	});

	// Completion bridge: plugin -> host on actor exit.
	let (done_tx, done_rx) = tokio::sync::oneshot::channel::<RunDone>();
	let user_data = Box::into_raw(Box::new(done_tx)) as *mut c_void;

	// Submit. The plugin copies the vtable synchronously in `run`.
	let instance = unsafe {
		(plugin.run())(
			factory.0,
			&vtable.0 as *const abi::HostVtable,
			run_done,
			user_data,
		)
	};
	let mut cancel = CancelGuard {
		plugin: plugin.clone(),
		instance: Some(instance),
	};

	// Await actor exit.
	let status = done_rx.await;

	// Cleanup: stop the adapter loop, drain outstanding replies (DroppedReply),
	// release the original ctx ref, free the instance (taken so the guard is a
	// no-op on the freed handle).
	let instance = cancel.instance.take().expect("instance present after run");
	loop_handle.abort();
	state.slab.drain();
	// SAFETY: balanced with `into_ctx_ptr`; in-flight callbacks hold their own
	// ctx clones, so the underlying ActorContext stays alive until they drop.
	unsafe {
		host_ctx_release(vtable.0.ctx);
		(plugin.instance_free())(instance);
	}

	match status {
		Ok(RunDone {
			status: abi::AbiStatus::Ok,
			..
		}) => Ok(()),
		Ok(done) => {
			let message = String::from_utf8_lossy(&done.payload);
			if message.is_empty() {
				Err(anyhow!("native actor exited with status {:?}", done.status))
			} else {
				Err(anyhow!(
					"native actor exited with status {:?}: {message}",
					done.status
				))
			}
		}
		Err(_) => Err(anyhow!("native actor completion channel dropped")),
	}
}

/// `Send` wrapper so the `#[repr(C)]` vtable (which holds a `*const c_void`)
/// can be kept alive across the completion await. The pointed-to state is
/// `Send + Sync` (`Arc<HostCtxState>`).
struct SendVtable(abi::HostVtable);
unsafe impl Send for SendVtable {}

// ---------------------------------------------------------------------------
// Host vtable — the plugin -> host capabilities (spec §4.4 / §6.2).
//
// The opaque `ctx` handle the plugin receives is `Arc::into_raw(Arc<HostCtxState>)`.
// It is refcounted: `ctx_clone`/`ctx_release` manage the Arc so the underlying
// `ActorContext` outlives any in-flight callback (spec §6.4). The async `db_*`
// fns are called ON THE PLUGIN'S THREAD, so they must spawn on the captured
// HOST runtime `Handle` (ambient spawn would hit the plugin's tokio).
// ---------------------------------------------------------------------------

/// One forwarded lifecycle event: `(tag, reply_token, event_bytes)`.
pub(crate) type ForwardedEvent = (u32, u64, Vec<u8>);

/// State behind the opaque `HostVtable.ctx` pointer. Shared by every vtable fn
/// and the adapter loop; refcounted so it outlives in-flight callbacks.
pub(crate) struct HostCtxState {
	ctx: ActorContext,
	runtime: Handle,
	/// Outstanding replies awaiting plugin answers (spec §6.3).
	slab: ReplySlab,
	/// Receiver side of the adapter loop's forwarded-event stream. The plugin
	/// pulls via `next_event`; `None` (closed) => `ChannelClosed`.
	events: tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<ForwardedEvent>>,
	/// Manual startup-ready signal (the entry uses `new_with_manual_startup_ready`).
	startup: Mutex<Option<tokio::sync::oneshot::Sender<anyhow::Result<()>>>>,
	/// User keep-awake regions held by a plugin actor.
	keep_awake: KeepAwakeStore,
}

impl HostCtxState {
	pub(crate) fn into_ctx_ptr(self: Arc<Self>) -> *const c_void {
		Arc::into_raw(self) as *const c_void
	}
}

extern "C" fn host_next_event(ctx: *const c_void, done: abi::CompletionFn, user_data: *mut c_void) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let ud = SendUserData(user_data);
		let handle = state.runtime.clone();
		handle.spawn(async move {
			let ud = ud;
			let mut rx = state.events.lock().await;
			let result = match rx.recv().await {
				Some((tag, token, payload)) => abi::AbiResult::ok(abi::OwnedBuf::from_vec(
					abi::encode_event_frame(tag, abi::ReplyToken(token), &payload),
				)),
				None => abi::AbiResult::channel_closed(),
			};
			drop(rx);
			done(ud.0, result);
		});
	}));
}

extern "C" fn host_reply_ok(
	ctx: *const c_void,
	reply_token: u64,
	payload: abi::OwnedBuf,
) -> abi::AbiStatus {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let bytes = payload.into_vec();
		if state.slab.fulfill_ok(reply_token, bytes) {
			abi::AbiStatus::Ok
		} else {
			abi::AbiStatus::Err
		}
	}))
	.unwrap_or(abi::AbiStatus::Panic)
}

extern "C" fn host_reply_err(
	ctx: *const c_void,
	reply_token: u64,
	err: abi::OwnedBuf,
) -> abi::AbiStatus {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let msg = String::from_utf8_lossy(&err.into_vec()).into_owned();
		if state.slab.fulfill_err(reply_token, msg) {
			abi::AbiStatus::Ok
		} else {
			abi::AbiStatus::Err
		}
	}))
	.unwrap_or(abi::AbiStatus::Panic)
}

extern "C" fn host_startup_ready(ctx: *const c_void, ok: u8, err_msg: abi::BorrowedBuf) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		if let Some(tx) = state.startup.lock().take() {
			let result = if ok != 0 {
				Ok(())
			} else {
				Err(anyhow!("{}", String::from_utf8_lossy(err_msg.as_slice())))
			};
			let _ = tx.send(result);
		}
	}));
}

/// Send wrapper for the plugin's `user_data` pointer so it can move into a
/// spawned task. The plugin owns the pointee; the host only round-trips it.
struct SendUserData(*mut c_void);
unsafe impl Send for SendUserData {}

/// Reconstitute an owned `Arc<HostCtxState>` from the opaque pointer WITHOUT
/// dropping the caller's reference (bumps the strong count by one). The
/// returned Arc must be dropped to release that bump.
unsafe fn ctx_arc(ptr: *const c_void) -> Arc<HostCtxState> {
	let p = ptr as *const HostCtxState;
	Arc::increment_strong_count(p);
	Arc::from_raw(p)
}

extern "C" fn host_ctx_clone(ptr: *const c_void) -> *const c_void {
	let _ = std::panic::catch_unwind(|| unsafe {
		Arc::increment_strong_count(ptr as *const HostCtxState);
	});
	ptr
}

extern "C" fn host_ctx_release(ptr: *const c_void) {
	let _ = std::panic::catch_unwind(|| unsafe {
		Arc::decrement_strong_count(ptr as *const HostCtxState);
	});
}

/// Encode a host-side error for transport. TODO(§4.7): structured
/// `{group, code, message, fatal}` CBOR; for now the UTF-8 message.
fn encode_db_error(err: &anyhow::Error) -> abi::OwnedBuf {
	abi::OwnedBuf::from_vec(format!("{err:#}").into_bytes())
}

fn ok_bytes(bytes: Vec<u8>) -> abi::AbiResult {
	abi::AbiResult::ok(abi::OwnedBuf::from_vec(bytes))
}

fn ok_cbor<T: serde::Serialize>(value: &T) -> abi::AbiResult {
	match encode_cbor(value) {
		Ok(bytes) => ok_bytes(bytes),
		Err(error) => abi::AbiResult::err(encode_db_error(&error)),
	}
}

fn encode_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ciborium::into_writer(value, &mut out)?;
	Ok(out)
}

fn decode_cbor<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
	Ok(ciborium::from_reader(std::io::Cursor::new(bytes))?)
}

fn list_opts(opts: abi::KvListOpts) -> ListOpts {
	ListOpts {
		reverse: opts.reverse,
		limit: opts.limit,
	}
}

struct CompletionGuard {
	done: abi::CompletionFn,
	ud: SendUserData,
	fired: bool,
}

impl CompletionGuard {
	fn fire(&mut self, result: abi::AbiResult) {
		if !self.fired {
			self.fired = true;
			(self.done)(self.ud.0, result);
		}
	}
}

impl Drop for CompletionGuard {
	fn drop(&mut self) {
		self.fire(abi::AbiResult::status_only(abi::AbiStatus::Cancelled));
	}
}

/// Spawn `fut` on the host runtime, then deliver its `AbiResult` to the plugin
/// completion callback exactly once. Keeps `state` alive across the call
/// (refcount), and wakes the plugin if the task is cancelled or panics.
fn spawn_completion<F>(
	state: Arc<HostCtxState>,
	done: abi::CompletionFn,
	user_data: *mut c_void,
	fut: F,
) where
	F: std::future::Future<Output = abi::AbiResult> + Send + 'static,
{
	let ud = SendUserData(user_data);
	let handle = state.runtime.clone();
	handle.spawn(async move {
		// Keep ctx alive for the whole op.
		let _state = state;
		let mut guard = CompletionGuard {
			done,
			ud,
			fired: false,
		};
		let result = match AssertUnwindSafe(fut).catch_unwind().await {
			Ok(result) => result,
			Err(_) => abi::AbiResult::status_only(abi::AbiStatus::Panic),
		};
		guard.fire(result);
	});
}

extern "C" fn host_db_exec(
	ctx: *const c_void,
	sql: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let sql_vec = sql.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let sql_str = std::str::from_utf8(&sql_vec).context("sql utf8")?;
				st.ctx.db_exec(sql_str).await
			}
			.await;
			match r {
				Ok(bytes) => ok_bytes(bytes),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_db_query(
	ctx: *const c_void,
	sql: abi::OwnedBuf,
	params: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let sql_vec = sql.into_vec();
		let params_vec = params.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let sql_str = std::str::from_utf8(&sql_vec).context("sql utf8")?;
				let params = (!params_vec.is_empty()).then_some(params_vec.as_slice());
				st.ctx.db_query(sql_str, params).await
			}
			.await;
			match r {
				Ok(bytes) => ok_bytes(bytes),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_db_run(
	ctx: *const c_void,
	sql: abi::OwnedBuf,
	params: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let sql_vec = sql.into_vec();
		let params_vec = params.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let sql_str = std::str::from_utf8(&sql_vec).context("sql utf8")?;
				let params = (!params_vec.is_empty()).then_some(params_vec.as_slice());
				st.ctx.db_run(sql_str, params).await
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_sql_is_enabled(ctx: *const c_void) -> u8 {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		u8::from(state.ctx.sql().is_enabled())
	}))
	.unwrap_or(0)
}

extern "C" fn host_state_get(ctx: *const c_void) -> abi::OwnedBuf {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		abi::OwnedBuf::from_vec(state.ctx.state())
	}))
	.unwrap_or_else(|_| abi::OwnedBuf::empty())
}

extern "C" fn host_state_set(ctx: *const c_void, state_bytes: abi::OwnedBuf) -> abi::AbiStatus {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		state.ctx.set_initial_state(state_bytes.into_vec());
		abi::AbiStatus::Ok
	}))
	.unwrap_or(abi::AbiStatus::Panic)
}

extern "C" fn host_actor_identity(ctx: *const c_void) -> abi::OwnedBuf {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let identity = abi::ActorIdentity {
			actor_id: state.ctx.actor_id().to_owned(),
			name: state.ctx.name().to_owned(),
			key: format_actor_key(state.ctx.key()),
			region: state.ctx.region().to_owned(),
			input: state.ctx.input(),
			has_state: state.ctx.has_state(),
		};
		match encode_cbor(&identity) {
			Ok(bytes) => abi::OwnedBuf::from_vec(bytes),
			Err(error) => encode_db_error(&error),
		}
	}))
	.unwrap_or_else(|_| abi::OwnedBuf::empty())
}

extern "C" fn host_state_save(
	ctx: *const c_void,
	state_bytes: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let bytes = state_bytes.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			match st.ctx.save_state(vec![StateDelta::ActorState(bytes)]).await {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_request_save(
	ctx: *const c_void,
	immediate: u8,
	has_max_wait: u8,
	max_wait_ms: u32,
) -> abi::AbiStatus {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		state.ctx.request_save(RequestSaveOpts {
			immediate: immediate != 0,
			max_wait_ms: (has_max_wait != 0).then_some(max_wait_ms),
		});
		abi::AbiStatus::Ok
	}))
	.unwrap_or(abi::AbiStatus::Panic)
}

extern "C" fn host_request_save_and_wait(
	ctx: *const c_void,
	immediate: u8,
	has_max_wait: u8,
	max_wait_ms: u32,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let opts = RequestSaveOpts {
			immediate: immediate != 0,
			max_wait_ms: (has_max_wait != 0).then_some(max_wait_ms),
		};
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			match st.ctx.request_save_and_wait(opts).await {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_sleep(ctx: *const c_void) -> abi::AbiResult {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		match state.ctx.sleep() {
			Ok(()) => ok_bytes(Vec::new()),
			Err(error) => abi::AbiResult::err(encode_db_error(&error)),
		}
	}))
	.unwrap_or_else(|_| abi::AbiResult::status_only(abi::AbiStatus::Panic))
}

extern "C" fn host_actor_aborted(ctx: *const c_void) -> u8 {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		u8::from(state.ctx.actor_aborted())
	}))
	.unwrap_or(1)
}

extern "C" fn host_wait_actor_abort(
	ctx: *const c_void,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let token = state.ctx.actor_abort_signal();
		spawn_completion(state, done, user_data, async move {
			token.cancelled().await;
			abi::AbiResult::ok(abi::OwnedBuf::empty())
		});
	}));
}

extern "C" fn host_keep_awake_enter(ctx: *const c_void) -> abi::AbiResult {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		ok_cbor(&state.keep_awake.insert(state.ctx.keep_awake_region()))
	}))
	.unwrap_or_else(|_| abi::AbiResult::status_only(abi::AbiStatus::Panic))
}

extern "C" fn host_keep_awake_exit(ctx: *const c_void, token: u64) -> abi::AbiStatus {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		match state.keep_awake.remove(abi::KeepAwakeToken { token }) {
			Ok(()) => abi::AbiStatus::Ok,
			Err(error) => {
				tracing::warn!(?error, "native plugin released an unknown keep-awake token");
				abi::AbiStatus::Err
			}
		}
	}))
	.unwrap_or(abi::AbiStatus::Panic)
}

extern "C" fn host_keep_awake_count(ctx: *const c_void) -> u64 {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		state.ctx.keep_awake_count() as u64
	}))
	.unwrap_or(0)
}

extern "C" fn host_kv_get(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvKeyRequest = decode_cbor(&request_bytes)?;
				let mut values = st.ctx.kv_batch_get(&[request.key.as_slice()]).await?;
				Ok::<_, anyhow::Error>(abi::KvGetResponse {
					value: values.pop().flatten(),
				})
			}
			.await;
			match r {
				Ok(response) => ok_cbor(&response),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_kv_put(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvEntriesRequest = decode_cbor(&request_bytes)?;
				let refs: Vec<(&[u8], &[u8])> = request
					.entries
					.iter()
					.map(|entry| (entry.key.as_slice(), entry.value.as_slice()))
					.collect();
				st.ctx.kv_batch_put(&refs).await
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_kv_delete(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvKeysRequest = decode_cbor(&request_bytes)?;
				let refs: Vec<&[u8]> = request.keys.iter().map(Vec::as_slice).collect();
				st.ctx.kv_batch_delete(&refs).await
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_kv_batch_get(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvKeysRequest = decode_cbor(&request_bytes)?;
				let refs: Vec<&[u8]> = request.keys.iter().map(Vec::as_slice).collect();
				Ok::<_, anyhow::Error>(abi::KvValuesResponse {
					values: st.ctx.kv_batch_get(&refs).await?,
				})
			}
			.await;
			match r {
				Ok(response) => ok_cbor(&response),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_kv_batch_put(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	host_kv_put(ctx, request, done, user_data);
}

extern "C" fn host_kv_batch_delete(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	host_kv_delete(ctx, request, done, user_data);
}

extern "C" fn host_kv_delete_range(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvRangeRequest = decode_cbor(&request_bytes)?;
				st.ctx.kv_delete_range(&request.start, &request.end).await
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_kv_list_prefix(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvListPrefixRequest = decode_cbor(&request_bytes)?;
				let entries = st
					.ctx
					.kv_list_prefix(&request.prefix, list_opts(request.opts))
					.await?
					.into_iter()
					.map(|(key, value)| abi::KvEntry { key, value })
					.collect();
				Ok::<_, anyhow::Error>(abi::KvListResponse { entries })
			}
			.await;
			match r {
				Ok(response) => ok_cbor(&response),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_kv_list_range(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::KvListRangeRequest = decode_cbor(&request_bytes)?;
				let entries = st
					.ctx
					.kv_list_range(&request.start, &request.end, list_opts(request.opts))
					.await?
					.into_iter()
					.map(|(key, value)| abi::KvEntry { key, value })
					.collect();
				Ok::<_, anyhow::Error>(abi::KvListResponse { entries })
			}
			.await;
			match r {
				Ok(response) => ok_cbor(&response),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_schedule_after(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::ScheduleActionRequest = decode_cbor(&request_bytes)?;
				let delay_ms = request
					.delay_ms
					.ok_or_else(|| anyhow!("schedule_after missing delay_ms"))?;
				st.ctx.after(
					Duration::from_millis(delay_ms),
					&request.action_name,
					&request.args,
				);
				Ok::<_, anyhow::Error>(())
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_schedule_at(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::ScheduleActionRequest = decode_cbor(&request_bytes)?;
				let timestamp_ms = request
					.timestamp_ms
					.ok_or_else(|| anyhow!("schedule_at missing timestamp_ms"))?;
				st.ctx.at(timestamp_ms, &request.action_name, &request.args);
				Ok::<_, anyhow::Error>(())
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_set_alarm(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::ScheduleAlarmRequest = decode_cbor(&request_bytes)?;
				st.ctx.set_alarm(request.timestamp_ms)
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_scheduled_events(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		request.free_self();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let events = st
				.ctx
				.scheduled_events()
				.into_iter()
				.map(|event| abi::ScheduledEvent {
					event_id: event.event_id,
					timestamp_ms: event.timestamp,
					action_name: event.action,
					args: event.args,
				})
				.collect();
			ok_cbor(&abi::ScheduledEventsResponse { events })
		});
	}));
}

extern "C" fn host_conn_list(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		request.free_self();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let conns = st.ctx.conns().map(|conn| conn_info(&conn)).collect();
			ok_cbor(&abi::ConnListResponse { conns })
		});
	}));
}

extern "C" fn host_conn_disconnect(
	ctx: *const c_void,
	request: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request_bytes = request.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let r = async {
				let request: abi::ConnDisconnectRequest = decode_cbor(&request_bytes)?;
				st.ctx
					.disconnect_conns(|conn| request.conn_ids.iter().any(|id| id == conn.id()))
					.await
			}
			.await;
			match r {
				Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
				Err(e) => abi::AbiResult::err(encode_db_error(&e)),
			}
		});
	}));
}

extern "C" fn host_hibernatable_ws_ack(
	ctx: *const c_void,
	gateway_id: abi::OwnedBuf,
	request_id: abi::OwnedBuf,
	server_message_index: u16,
) -> abi::AbiResult {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let gateway_id = gateway_id.into_vec();
		let request_id = request_id.into_vec();
		match state.ctx.ack_hibernatable_websocket_message(
			&gateway_id,
			&request_id,
			server_message_index,
		) {
			Ok(()) => ok_bytes(Vec::new()),
			Err(error) => abi::AbiResult::err(encode_db_error(&error)),
		}
	}))
	.unwrap_or_else(|_| abi::AbiResult::status_only(abi::AbiStatus::Panic))
}

extern "C" fn host_conn_send(ctx: *const c_void, request: abi::OwnedBuf) -> abi::AbiResult {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let request: abi::ConnSendRequest = match decode_cbor(&request.into_vec()) {
			Ok(request) => request,
			Err(error) => return abi::AbiResult::err(encode_db_error(&error)),
		};
		let Some(conn) = state.ctx.conns().find(|conn| conn.id() == request.conn_id) else {
			return abi::AbiResult::err(encode_db_error(&anyhow!(
				"connection `{}` not found",
				request.conn_id
			)));
		};
		match conn.try_send(&request.name, &request.payload) {
			Ok(()) => ok_bytes(Vec::new()),
			Err(error) => abi::AbiResult::err(encode_db_error(&error)),
		}
	}))
	.unwrap_or_else(|_| abi::AbiResult::status_only(abi::AbiStatus::Panic))
}

extern "C" fn host_broadcast(
	ctx: *const c_void,
	name: abi::OwnedBuf,
	payload: abi::OwnedBuf,
) -> abi::AbiStatus {
	std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let name_vec = name.into_vec();
		let payload_vec = payload.into_vec();
		match std::str::from_utf8(&name_vec) {
			Ok(name_str) => {
				state.ctx.broadcast(name_str, &payload_vec);
				abi::AbiStatus::Ok
			}
			Err(_) => abi::AbiStatus::Err,
		}
	}))
	.unwrap_or(abi::AbiStatus::Panic)
}

extern "C" fn host_log(ctx: *const c_void, level: i32, msg: abi::BorrowedBuf) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let _state = ctx_arc(ctx);
		let bytes = msg.as_slice();
		let text = String::from_utf8_lossy(bytes);
		match level {
			0 => tracing::trace!(target: "native_plugin", "{text}"),
			1 => tracing::debug!(target: "native_plugin", "{text}"),
			2 => tracing::info!(target: "native_plugin", "{text}"),
			3 => tracing::warn!(target: "native_plugin", "{text}"),
			_ => tracing::error!(target: "native_plugin", "{text}"),
		}
	}));
}

/// HTTP request forwarded to the plugin (`Http` tag payload), CBOR-encoded.
#[derive(serde::Serialize, serde::Deserialize)]
struct HttpReqWire {
	method: String,
	uri: String,
	headers: HashMap<String, String>,
	#[serde(with = "serde_bytes")]
	body: Vec<u8>,
}

/// HTTP response the plugin returns in its reply, CBOR-encoded.
#[derive(serde::Serialize, serde::Deserialize)]
struct HttpRespWire {
	status: u16,
	headers: HashMap<String, String>,
	#[serde(with = "serde_bytes")]
	body: Vec<u8>,
}

fn encode_http_request(req: &Request) -> Vec<u8> {
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
	let wire: HttpRespWire = ciborium::from_reader(std::io::Cursor::new(bytes))
		.context("decode plugin http response")?;
	Response::from_parts(wire.status, wire.headers, wire.body)
}

fn conn_info(conn: &ConnHandle) -> abi::ConnInfo {
	abi::ConnInfo {
		id: conn.id().to_owned(),
		params: conn.params(),
		state: conn.state(),
		is_hibernatable: conn.is_hibernatable(),
	}
}

/// The event adapter (spec §6.3): drains the core `ActorEvents`, forwards
/// reply-bearing actor events to the plugin (storing their `Reply` in the slab),
/// and answers reject/skip/internal events inline. On stream end (actor exit)
/// it drains the slab so no awaiting caller hangs.
async fn adapter_loop(
	mut events: ActorEvents,
	tx: tokio::sync::mpsc::UnboundedSender<ForwardedEvent>,
	state: Arc<HostCtxState>,
) {
	use abi::AbiEventTag as Tag;
	while let Some(ev) = events.recv().await {
		match ev {
			// --- forwarded (reply stored in slab) ---
			ActorEvent::Action {
				name, args, reply, ..
			} => {
				let token = state.slab.insert(PendingReply::Bytes(reply));
				if tx
					.send((
						Tag::Action as u32,
						token,
						abi::encode_action_payload(&name, &args),
					))
					.is_err()
				{
					break;
				}
			}
			ActorEvent::ConnectionPreflight {
				conn,
				params,
				reply,
				..
			} => {
				let token = state.slab.insert(PendingReply::Unit(reply));
				let payload = match abi::encode_conn_preflight_payload(&conn_info(&conn), &params) {
					Ok(payload) => payload,
					Err(error) => {
						tracing::error!(?error, "failed to encode connection preflight event");
						state.slab.fulfill_err(token, format!("{error:#}"));
						continue;
					}
				};
				if tx
					.send((Tag::ConnPreflight as u32, token, payload))
					.is_err()
				{
					break;
				}
			}
			ActorEvent::ConnectionOpen { conn, reply, .. } => {
				let token = state.slab.insert(PendingReply::Unit(reply));
				let payload = match abi::encode_conn_open_payload(&conn_info(&conn)) {
					Ok(payload) => payload,
					Err(error) => {
						tracing::error!(?error, "failed to encode connection open event");
						state.slab.fulfill_err(token, format!("{error:#}"));
						continue;
					}
				};
				if tx.send((Tag::ConnOpen as u32, token, payload)).is_err() {
					break;
				}
			}
			ActorEvent::SubscribeRequest {
				conn,
				reply,
				event_name,
			} => {
				let token = state.slab.insert(PendingReply::Unit(reply));
				let payload = match abi::encode_subscribe_payload(&conn_info(&conn), &event_name) {
					Ok(payload) => payload,
					Err(error) => {
						tracing::error!(?error, "failed to encode subscribe event");
						state.slab.fulfill_err(token, format!("{error:#}"));
						continue;
					}
				};
				if tx.send((Tag::Subscribe as u32, token, payload)).is_err() {
					break;
				}
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
				let token = state.slab.insert(PendingReply::Queue(reply));
				let payload = match abi::encode_queue_send_payload(
					&name,
					&body,
					&conn_info(&conn),
					&encode_http_request(&request),
					wait,
					timeout_ms,
				) {
					Ok(payload) => payload,
					Err(error) => {
						tracing::error!(?error, "failed to encode queue send event");
						state.slab.fulfill_err(token, format!("{error:#}"));
						continue;
					}
				};
				if tx.send((Tag::QueueSend as u32, token, payload)).is_err() {
					break;
				}
			}
			ActorEvent::WebSocketOpen {
				conn,
				request,
				reply,
				..
			} => {
				let token = state.slab.insert(PendingReply::Unit(reply));
				let request = request.as_ref().map(encode_http_request);
				let payload =
					match abi::encode_ws_open_payload(&conn_info(&conn), request.as_deref()) {
						Ok(payload) => payload,
						Err(error) => {
							tracing::error!(?error, "failed to encode websocket open event");
							state.slab.fulfill_err(token, format!("{error:#}"));
							continue;
						}
					};
				if tx.send((Tag::WsOpen as u32, token, payload)).is_err() {
					break;
				}
			}
			ActorEvent::RunGracefulCleanup { reason, reply } => {
				let tag = match reason {
					ShutdownKind::Sleep => Tag::Sleep,
					ShutdownKind::Destroy => Tag::Destroy,
				};
				let token = state.slab.insert(PendingReply::Unit(reply));
				if tx.send((tag as u32, token, Vec::new())).is_err() {
					break;
				}
			}
			ActorEvent::ConnectionClosed { conn } => {
				// No reply; reply_token 0.
				let payload = match abi::encode_conn_closed_payload(&conn_info(&conn)) {
					Ok(payload) => payload,
					Err(error) => {
						tracing::error!(?error, "failed to encode connection closed event");
						continue;
					}
				};
				if tx.send((Tag::ConnClosed as u32, 0, payload)).is_err() {
					break;
				}
			}

			ActorEvent::HttpRequest { request, reply } => {
				let token = state.slab.insert(PendingReply::Http(reply));
				if tx
					.send((Tag::Http as u32, token, encode_http_request(&request)))
					.is_err()
				{
					break;
				}
			}
			ActorEvent::SerializeState { reply, .. } => {
				let token = state.slab.insert(PendingReply::State(reply));
				if tx
					.send((Tag::SerializeState as u32, token, Vec::new()))
					.is_err()
				{
					break;
				}
			}

			// --- answered inline (not forwarded) ---
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
	// Actor exiting: fail any outstanding replies (DroppedReply) so no caller hangs.
	state.slab.drain();
}

// ---------------------------------------------------------------------------
// Reply slab (spec §6.3) — the safety-critical reply lifecycle.
//
// Forwarded reply-bearing events hand their `Reply<T>` to this slab keyed by a
// `reply_token`. The plugin answers via `reply_ok`/`reply_err(token, ..)`. On
// actor exit/cancel the slab is DRAINED, dropping every outstanding `Reply<T>`
// — whose `Drop` sends `Err(DroppedReply)` so callers never hang.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

use crate::actor::lifecycle_hooks::Reply;

/// A reply handle awaiting the plugin's answer. Only the variants the adapter
/// forwards are represented (Action → bytes; lifecycle/conn/subscribe → unit);
/// other events are answered inline by the adapter and never reach the slab.
pub(crate) enum PendingReply {
	Bytes(Reply<Vec<u8>>),
	Unit(Reply<()>),
	State(Reply<Vec<StateDelta>>),
	Http(Reply<Response>),
	Queue(Reply<QueueSendResult>),
}

impl PendingReply {
	fn fulfill_ok(self, payload: Vec<u8>) {
		match self {
			PendingReply::Bytes(r) => r.send(Ok(payload)),
			PendingReply::Unit(r) => r.send(Ok(())),
			PendingReply::State(r) => {
				let deltas = if payload.is_empty() {
					Vec::new()
				} else {
					vec![StateDelta::ActorState(payload)]
				};
				r.send(Ok(deltas));
			}
			PendingReply::Http(r) => r.send(decode_http_response(&payload)),
			PendingReply::Queue(r) => r.send(decode_queue_send_response(&payload)),
		}
	}

	fn fulfill_err(self, msg: String) {
		match self {
			PendingReply::Bytes(r) => r.send(Err(anyhow!("{msg}"))),
			PendingReply::Unit(r) => r.send(Err(anyhow!("{msg}"))),
			PendingReply::State(r) => r.send(Err(anyhow!("{msg}"))),
			PendingReply::Http(r) => r.send(Err(anyhow!("{msg}"))),
			PendingReply::Queue(r) => r.send(Err(anyhow!("{msg}"))),
		}
	}
}

fn decode_queue_send_response(bytes: &[u8]) -> Result<QueueSendResult> {
	let wire: abi::QueueSendResponse = ciborium::from_reader(std::io::Cursor::new(bytes))
		.context("decode plugin queue send response")?;
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

/// Token-keyed store of outstanding replies. Draining drops every `Reply<T>`,
/// firing `Err(DroppedReply)` to unblock any awaiting caller.
pub(crate) struct ReplySlab {
	next: AtomicU64,
	map: Mutex<HashMap<u64, PendingReply>>,
}

impl ReplySlab {
	pub(crate) fn new() -> Self {
		Self {
			next: AtomicU64::new(1),
			map: Mutex::new(HashMap::new()),
		}
	}

	/// Store a reply, returning its non-zero token.
	pub(crate) fn insert(&self, reply: PendingReply) -> u64 {
		let token = self.next.fetch_add(1, Ordering::Relaxed);
		self.map.lock().insert(token, reply);
		token
	}

	/// Fulfill a pending reply with the plugin's payload. Returns false if the
	/// token is unknown, already taken, or drained.
	pub(crate) fn fulfill_ok(&self, token: u64, payload: Vec<u8>) -> bool {
		if let Some(reply) = self.map.lock().remove(&token) {
			reply.fulfill_ok(payload);
			true
		} else {
			false
		}
	}

	pub(crate) fn fulfill_err(&self, token: u64, msg: String) -> bool {
		if let Some(reply) = self.map.lock().remove(&token) {
			reply.fulfill_err(msg);
			true
		} else {
			false
		}
	}

	/// Drop every outstanding reply (on actor exit/cancel). Each `Reply::drop`
	/// sends `Err(DroppedReply)`.
	pub(crate) fn drain(&self) {
		self.map.lock().clear();
	}
}

pub(crate) struct KeepAwakeStore {
	next: AtomicU64,
	regions: Mutex<HashMap<u64, KeepAwakeRegion>>,
}

impl KeepAwakeStore {
	fn new() -> Self {
		Self {
			next: AtomicU64::new(1),
			regions: Mutex::new(HashMap::new()),
		}
	}

	fn insert(&self, region: KeepAwakeRegion) -> abi::KeepAwakeToken {
		let token = self.next.fetch_add(1, Ordering::Relaxed);
		self.regions.lock().insert(token, region);
		abi::KeepAwakeToken { token }
	}

	fn remove(&self, token: abi::KeepAwakeToken) -> Result<()> {
		self.regions
			.lock()
			.remove(&token.token)
			.ok_or_else(|| anyhow!("keep-awake token {} is unknown", token.token))?;
		Ok(())
	}
}

#[cfg(test)]
mod slab_tests {
	use super::*;

	#[test]
	fn drain_drops_replies_with_error() {
		let slab = ReplySlab::new();
		let (tx, mut rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
		let token = slab.insert(PendingReply::Unit(Reply::from(tx)));
		assert_eq!(token, 1);
		slab.drain();
		// The dropped Reply must have sent an error (DroppedReply), not hang.
		match rx.try_recv() {
			Ok(result) => assert!(result.is_err(), "drained reply should be Err"),
			other => panic!("expected a sent error, got {other:?}"),
		}
	}

	#[test]
	fn fulfill_ok_delivers_payload() {
		let slab = ReplySlab::new();
		let (tx, mut rx) = tokio::sync::oneshot::channel::<anyhow::Result<Vec<u8>>>();
		let token = slab.insert(PendingReply::Bytes(Reply::from(tx)));
		assert!(slab.fulfill_ok(token, vec![1, 2, 3]));
		let got = rx.try_recv().expect("sent").expect("ok");
		assert_eq!(got, vec![1, 2, 3]);
		// Token is consumed; a second fulfill is rejected.
		assert!(!slab.fulfill_ok(token, vec![9]));
	}
}
