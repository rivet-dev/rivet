//! Generic loader for native actor plugins (`dlopen` of a `cdylib`), per the
//! dylib-actor-plugin spec §6.1. RivetKit knows only this generic ABI, not any
//! product-specific symbols. The plugin is resolved by path, its single API
//! descriptor is verified (refuse-on-mismatch, no fallback), and it is adapted
//! into the existing [`ActorFactory`] boxed-closure entry point.
//!
//! This module is the load/ABI layer plus the host-driven event adapter. The
//! original core reply moves directly into one event task and is completed by
//! that event's callback; there is no host reply-token slab.
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
//! | `ConnectionPreflight`   | `ConnPreflight` (reply: accept)                |
//! | `WorkflowHistory/Replay`| not applicable to native plugins (reply empty) |
//!
//! The host admits events in mailbox order and allows a bounded number of
//! direct plugin callbacks in flight. Shutdown closes admission, waits for the
//! plugin barrier, and then drains every admitted completion before freeing the
//! instance.

// FFI glue: `unsafe fn`s here are unsafe in their entirety by design.
#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use futures::future::FutureExt as _;
use libloading::{Library, Symbol};
use parking_lot::Mutex;
use rivet_actor_plugin_abi as abi;
use tokio::runtime::Handle;
use tokio::sync::Semaphore;

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

/// Opaque plugin-owned handle (plugin/factory/instance). Send+Sync because the
/// plugin owns the pointed-to state and the host only passes it back opaquely.
#[derive(Clone, Copy)]
pub(crate) struct OpaqueHandle(pub(crate) *mut c_void);
unsafe impl Send for OpaqueHandle {}
unsafe impl Sync for OpaqueHandle {}

/// A `dlopen`ed, ABI-verified, initialized plugin. Kept alive for the process
/// lifetime (never unloaded — unloading a dylib with live runtime/threads is
/// unsound). One per unique dylib path.
// `factory_free`/`plugin_shutdown` are retained for explicit teardown paths not
// yet wired. Loaded libraries remain cached for the process lifetime.
#[allow(dead_code)]
pub(crate) struct LoadedPlugin {
	// Field order matters for drop: handle/table before `_lib`. We never drop
	// these in practice (cached for process lifetime), but keep `_lib` last.
	plugin: OpaqueHandle,
	api: abi::PluginApi,
	_lib: Library,
}

unsafe impl Send for LoadedPlugin {}
unsafe impl Sync for LoadedPlugin {}

#[allow(dead_code)]
impl LoadedPlugin {
	pub(crate) fn factory_new(&self) -> abi::FactoryNewFn {
		self.api.factory_new
	}
	pub(crate) fn instance_new(&self) -> abi::InstanceNewFn {
		self.api.instance_new
	}
	pub(crate) fn handle_event(&self) -> abi::HandleEventFn {
		self.api.handle_event
	}
	pub(crate) fn cancel_event(&self) -> abi::CancelEventFn {
		self.api.cancel_event
	}
	pub(crate) fn shutdown(&self) -> abi::ShutdownFn {
		self.api.shutdown
	}
	pub(crate) fn instance_free(&self) -> abi::HandleFn {
		self.api.instance_free
	}
	pub(crate) fn factory_free(&self) -> abi::HandleFn {
		self.api.factory_free
	}
	pub(crate) fn plugin_shutdown(&self) -> abi::HandleFn {
		self.api.plugin_shutdown
	}
}

/// Initial fields shared by every descriptor version. Read this prefix before
/// copying the full table so an undersized descriptor is rejected cleanly.
#[repr(C)]
#[derive(Clone, Copy)]
struct PluginApiHeader {
	abi_magic: u64,
	abi_version: u64,
	struct_size: usize,
}

fn validate_plugin_api_header(header: PluginApiHeader) -> Result<()> {
	if header.abi_magic != abi::RIVET_ACTOR_ABI_MAGIC {
		bail!(
			"not a rivet actor plugin (magic {:#x} != {:#x})",
			header.abi_magic,
			abi::RIVET_ACTOR_ABI_MAGIC
		);
	}
	if header.abi_version != abi::RIVET_ACTOR_ABI_VERSION {
		bail!(
			"actor plugin ABI v{}, host expects v{} (same-version lockstep; no fallback)",
			header.abi_version,
			abi::RIVET_ACTOR_ABI_VERSION
		);
	}
	let expected_size = std::mem::size_of::<abi::PluginApi>();
	if header.struct_size != expected_size {
		bail!(
			"actor plugin API descriptor is {} bytes, host expects {expected_size} bytes",
			header.struct_size
		);
	}
	Ok(())
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

	// Resolve only the descriptor getter. The getter performs no allocation or
	// initialization; validate its fixed header before any plugin function.
	let plugin_api: abi::PluginApiFn = sym(&lib, abi::symbols::PLUGIN_API)?;
	let api_ptr = plugin_api();
	if api_ptr.is_null() {
		bail!("rivet actor plugin returned a null API descriptor");
	}
	let header = (api_ptr as *const PluginApiHeader).read();
	validate_plugin_api_header(header)?;
	let api = api_ptr.read();

	let mut out_err = abi::OwnedBuf::empty();
	let plugin = (api.plugin_init)(&mut out_err as *mut _);
	if plugin.is_null() {
		let msg = take_out_err(out_err);
		bail!("rivet_actor_plugin_init failed: {msg}");
	}

	Ok(LoadedPlugin {
		plugin: OpaqueHandle(plugin),
		api,
		_lib: lib,
	})
}

/// Create a per-actor-type plugin factory: load the plugin, call `factory_new`
/// with the opaque config envelope + sidecar path, and adapt the result into a
/// RivetKit [`ActorFactory`].
///
pub fn build_native_plugin_factory(
	plugin_path: &Path,
	config_json: &str,
	sidecar_path: &str,
	config: ActorConfig,
) -> Result<ActorFactory> {
	build_native_plugin_factory_inner(plugin_path, config_json, sidecar_path, config, None)
}

/// Build a native-plugin actor with a generic host callback overlay. The
/// overlay and native backend share one core actor instance, mailbox, database,
/// connection set, and shutdown barrier.
pub fn build_native_plugin_factory_with_overlay(
	plugin_path: &Path,
	config_json: &str,
	sidecar_path: &str,
	config: ActorConfig,
	overlay: Arc<dyn NativePluginOverlay>,
) -> Result<ActorFactory> {
	build_native_plugin_factory_inner(
		plugin_path,
		config_json,
		sidecar_path,
		config,
		Some(overlay),
	)
}

fn build_native_plugin_factory_inner(
	plugin_path: &Path,
	config_json: &str,
	sidecar_path: &str,
	config: ActorConfig,
	overlay: Option<Arc<dyn NativePluginOverlay>>,
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
		let overlay = overlay.clone();
		Box::pin(run_native_actor(plugin, factory, start, overlay))
			as crate::runtime::RuntimeBoxFuture<Result<()>>
	};

	Ok(ActorFactory::new_with_manual_startup_ready(config, entry))
}

pub type NativePluginOverlayFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'static>>;

/// Runtime-neutral callback layer around a native plugin backend. NAPI uses
/// this trait for TypeScript hooks; other hosts may provide their own overlay.
pub trait NativePluginOverlay: Send + Sync + 'static {
	fn resolve_instance_options(
		&self,
		_ctx: ActorContext,
		_input: Option<Vec<u8>>,
		_is_new: bool,
	) -> NativePluginOverlayFuture<Option<Vec<u8>>> {
		Box::pin(async { Ok(None) })
	}

	fn host_call(
		&self,
		_ctx: ActorContext,
		name: String,
		_payload: Vec<u8>,
	) -> NativePluginOverlayFuture<Vec<u8>> {
		Box::pin(async move {
			Err(anyhow!(
				"native plugin host call `{name}` is not registered"
			))
		})
	}

	fn handle_event(
		&self,
		ctx: ActorContext,
		event: ActorEvent,
		native: NativePluginEventHandler,
	) -> NativePluginOverlayFuture<()>;
}

/// Per-instance native fallback supplied to a host overlay. Each call pushes
/// exactly one event to the already-running plugin instance.
#[derive(Clone)]
pub struct NativePluginEventHandler {
	plugin: Arc<LoadedPlugin>,
	instance: OpaqueHandle,
	next_event_id: Arc<std::sync::atomic::AtomicU64>,
}

impl NativePluginEventHandler {
	pub async fn handle_event(&self, event: ActorEvent) -> Result<()> {
		let Some(event) = forward_actor_event(event) else {
			return Ok(());
		};
		let event_id = self
			.next_event_id
			.fetch_update(
				std::sync::atomic::Ordering::Relaxed,
				std::sync::atomic::Ordering::Relaxed,
				|current| Some(current.wrapping_add(1).max(1)),
			)
			.expect("native plugin event id update is infallible");
		dispatch_plugin_event(self.plugin.clone(), self.instance, event_id, event).await
	}
}

/// Completion callback used by terminal, event, and shutdown operations.
/// Reclaims the boxed oneshot sender and the producer-owned payload.
struct PluginDone {
	status: abi::AbiStatus,
	payload: Vec<u8>,
}

extern "C" fn plugin_done(user_data: *mut c_void, result: abi::AbiResult) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let tx = Box::from_raw(user_data as *mut tokio::sync::oneshot::Sender<PluginDone>);
		let status = result.status;
		let payload = result.payload.into_vec();
		let _ = tx.send(PluginDone { status, payload });
	}));
}

fn plugin_done_result(done: PluginDone, operation: &str) -> Result<Vec<u8>> {
	match done.status {
		abi::AbiStatus::Ok => Ok(done.payload),
		status => {
			let message = String::from_utf8_lossy(&done.payload);
			if message.is_empty() {
				Err(anyhow!(
					"native plugin {operation} completed with {status:?}"
				))
			} else {
				Err(anyhow!(
					"native plugin {operation} completed with {status:?}: {message}"
				))
			}
		}
	}
}

struct ForcedShutdown {
	plugin: Arc<LoadedPlugin>,
	instance: OpaqueHandle,
}

extern "C" fn forced_shutdown_done(user_data: *mut c_void, result: abi::AbiResult) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		result.payload.free_self();
		let shutdown = Box::from_raw(user_data as *mut ForcedShutdown);
		(shutdown.plugin.instance_free())(shutdown.instance.0);
	}));
}

/// Guarantees forced shutdown and eventual instance reclamation if the host
/// actor future is cancelled before the normal shutdown barrier completes.
struct InstanceGuard {
	plugin: Arc<LoadedPlugin>,
	instance: Option<OpaqueHandle>,
}

unsafe impl Send for InstanceGuard {}

impl InstanceGuard {
	fn handle(&self) -> OpaqueHandle {
		self.instance.expect("native plugin instance is live")
	}

	unsafe fn free(mut self) {
		let instance = self
			.instance
			.take()
			.expect("native plugin instance is live");
		(self.plugin.instance_free())(instance.0);
	}
}

impl Drop for InstanceGuard {
	fn drop(&mut self) {
		if let Some(instance) = self.instance.take() {
			let shutdown = Box::new(ForcedShutdown {
				plugin: self.plugin.clone(),
				instance,
			});
			unsafe {
				(self.plugin.shutdown())(
					instance.0,
					1,
					forced_shutdown_done,
					Box::into_raw(shutdown) as *mut c_void,
				);
			}
		}
	}
}

struct HostCtxGuard(Option<*const c_void>);
unsafe impl Send for HostCtxGuard {}

impl Drop for HostCtxGuard {
	fn drop(&mut self) {
		if let Some(ctx) = self.0.take() {
			host_ctx_release(ctx);
		}
	}
}

const MAX_IN_FLIGHT_PLUGIN_EVENTS: usize = 64;
const MAX_IN_FLIGHT_HOST_CALLS: usize = 32;
const HOST_CALL_WARN_AT: usize = 25;
const MAX_HOST_CALL_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_HOST_CALL_RESPONSE_BYTES: usize = 1024 * 1024;
const HOST_CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Drive one native-plugin actor instance. RivetKit owns mailbox admission and
/// pushes each event directly into the plugin with an exactly-once completion.
async fn run_native_actor(
	plugin: Arc<LoadedPlugin>,
	factory: OpaqueHandle,
	start: crate::actor::lifecycle_hooks::ActorStart,
	overlay: Option<Arc<dyn NativePluginOverlay>>,
) -> Result<()> {
	let runtime = Handle::current();
	let state = Arc::new(HostCtxState {
		ctx: start.ctx.clone(),
		runtime,
		overlay: overlay.clone(),
		host_call_slots: Arc::new(Semaphore::new(MAX_IN_FLIGHT_HOST_CALLS)),
		startup: Mutex::new(start.startup_ready),
		keep_awake: KeepAwakeStore::new(),
	});
	let instance_options = match &overlay {
		Some(overlay) => {
			overlay
				.resolve_instance_options(start.ctx.clone(), start.input.clone(), start.is_new)
				.await
		}
		None => Ok(None),
	};
	let instance_options = match instance_options {
		Ok(options) => options,
		Err(error) => {
			if let Some(startup) = state.startup.lock().take() {
				let _ = startup.send(Err(anyhow!("{error:#}")));
			}
			return Err(error).context("resolve native plugin instance options");
		}
	};
	let instance_start = abi::InstanceStart {
		is_new: start.is_new,
		input: start.input.clone(),
		instance_options,
	};
	let instance_start = encode_cbor(&instance_start).context("encode native plugin start data")?;

	let host_ctx = state.clone().into_ctx_ptr();
	let host_ctx_guard = HostCtxGuard(Some(host_ctx));
	let vtable = SendVtable(abi::HostVtable {
		abi_version: abi::RIVET_ACTOR_ABI_VERSION,
		ctx: host_ctx,
		ctx_clone: host_ctx_clone,
		ctx_release: host_ctx_release,
		db_exec: host_db_exec,
		db_query: host_db_query,
		db_run: host_db_run,
		host_call,
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
		startup_ready: host_startup_ready,
		broadcast: host_broadcast,
		log: host_log,
	});

	let (terminal_tx, mut terminal_rx) = tokio::sync::oneshot::channel::<PluginDone>();
	let instance = {
		let terminal_user_data = Box::into_raw(Box::new(terminal_tx)) as *mut c_void;
		let mut out_err = abi::OwnedBuf::empty();
		let instance = unsafe {
			(plugin.instance_new())(
				factory.0,
				&vtable.0 as *const abi::HostVtable,
				abi::BorrowedBuf::from_slice(&instance_start),
				&mut out_err as *mut _,
				plugin_done,
				terminal_user_data,
			)
		};
		if instance.is_null() {
			let message = unsafe { take_out_err(out_err) };
			unsafe {
				drop(Box::from_raw(
					terminal_user_data as *mut tokio::sync::oneshot::Sender<PluginDone>,
				))
			};
			return Err(anyhow!("native plugin instance_new failed: {message}"));
		}
		OpaqueHandle(instance)
	};
	let guard = InstanceGuard {
		plugin: plugin.clone(),
		instance: Some(instance),
	};

	let (admission_result, early_terminal) = {
		let dispatch =
			dispatch_plugin_events(start.events, start.ctx, plugin.clone(), instance, overlay);
		tokio::pin!(dispatch);
		tokio::select! {
			result = &mut dispatch => (result, None),
			terminal = &mut terminal_rx => {
				let result = terminal
					.map_err(|_| anyhow!("native plugin terminal completion channel dropped"))
					.and_then(|done| plugin_done_result(done, "actor loop"))
					.map(|_| ());
				let result = match result {
					Ok(()) => Err(anyhow!("native plugin actor loop exited before host admission closed")),
					Err(error) => Err(error),
				};
				(result, Some(()))
			}
		}
	};

	let force = admission_result.is_err() || early_terminal.is_some();
	let shutdown_result = shutdown_plugin_instance(&plugin, guard.handle(), force).await;
	let dispatch_result = match admission_result {
		Ok(tasks) => drain_plugin_events(tasks).await,
		Err(error) => Err(error),
	};
	let terminal_result = if early_terminal.is_some() {
		Ok(())
	} else {
		terminal_rx
			.await
			.map_err(|_| anyhow!("native plugin terminal completion channel dropped"))
			.and_then(|done| plugin_done_result(done, "actor loop"))
			.map(|_| ())
	};

	if shutdown_result.is_ok() {
		unsafe { guard.free() };
	} else {
		// The ordinary shutdown barrier did not prove the instance safe to free.
		// Keep the guard armed so its forced-shutdown callback owns reclamation.
		drop(guard);
	}
	drop(host_ctx_guard);

	dispatch_result?;
	shutdown_result?;
	terminal_result
}

async fn shutdown_plugin_instance(
	plugin: &Arc<LoadedPlugin>,
	instance: OpaqueHandle,
	force: bool,
) -> Result<()> {
	let (done_tx, done_rx) = tokio::sync::oneshot::channel::<PluginDone>();
	unsafe {
		(plugin.shutdown())(
			instance.0,
			u8::from(force),
			plugin_done,
			Box::into_raw(Box::new(done_tx)) as *mut c_void,
		);
	}
	let done = done_rx
		.await
		.map_err(|_| anyhow!("native plugin shutdown completion channel dropped"))?;
	plugin_done_result(done, "shutdown").map(|_| ())
}

async fn dispatch_plugin_events(
	mut events: ActorEvents,
	ctx: ActorContext,
	plugin: Arc<LoadedPlugin>,
	instance: OpaqueHandle,
	overlay: Option<Arc<dyn NativePluginOverlay>>,
) -> Result<tokio::task::JoinSet<Result<()>>> {
	let mut tasks = tokio::task::JoinSet::new();
	let native = NativePluginEventHandler {
		plugin,
		instance,
		next_event_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
	};

	while let Some(event) = events.recv().await {
		while tasks.len() >= MAX_IN_FLIGHT_PLUGIN_EVENTS {
			join_plugin_event(&mut tasks).await?;
		}
		let native = native.clone();
		if let Some(overlay) = overlay.clone() {
			let ctx = ctx.clone();
			tasks.spawn(async move { overlay.handle_event(ctx, event, native).await });
		} else {
			tasks.spawn(async move { native.handle_event(event).await });
		}
		while let Some(result) = tasks.try_join_next() {
			result.context("native plugin event task panicked")??;
		}
	}

	Ok(tasks)
}

async fn drain_plugin_events(mut tasks: tokio::task::JoinSet<Result<()>>) -> Result<()> {
	while !tasks.is_empty() {
		join_plugin_event(&mut tasks).await?;
	}
	Ok(())
}

async fn join_plugin_event(tasks: &mut tokio::task::JoinSet<Result<()>>) -> Result<()> {
	tasks
		.join_next()
		.await
		.context("native plugin event task missing")?
		.context("native plugin event task panicked")?
}

async fn dispatch_plugin_event(
	plugin: Arc<LoadedPlugin>,
	instance: OpaqueHandle,
	event_id: u64,
	event: ForwardedEvent,
) -> Result<()> {
	let (done_tx, done_rx) = tokio::sync::oneshot::channel::<PluginDone>();
	unsafe {
		(plugin.handle_event())(
			instance.0,
			event_id,
			abi::AbiEvent {
				tag: event.tag,
				payload: abi::OwnedBuf::from_vec(event.payload),
			},
			plugin_done,
			Box::into_raw(Box::new(done_tx)) as *mut c_void,
		);
	}
	let result = done_rx
		.await
		.map_err(|_| anyhow!("native plugin event {event_id} completion channel dropped"));
	match result {
		Ok(done) => event.reply.fulfill(done),
		Err(error) => {
			event.reply.fulfill_err(format!("{error:#}"));
			Err(error)
		}
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

/// State behind the opaque `HostVtable.ctx` pointer. Shared by every vtable fn
/// and refcounted so it outlives in-flight callbacks.
pub(crate) struct HostCtxState {
	ctx: ActorContext,
	runtime: Handle,
	overlay: Option<Arc<dyn NativePluginOverlay>>,
	host_call_slots: Arc<Semaphore>,
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

extern "C" fn host_call(
	ctx: *const c_void,
	name: abi::OwnedBuf,
	payload: abi::OwnedBuf,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) {
	let _ = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
		let state = ctx_arc(ctx);
		let name = name.into_vec();
		let payload = payload.into_vec();
		let st = state.clone();
		spawn_completion(state, done, user_data, async move {
			let result = async {
				let name = String::from_utf8(name).context("native plugin host-call name utf8")?;
				if payload.len() > MAX_HOST_CALL_REQUEST_BYTES {
					bail!(
						"native plugin host-call request limit {} bytes exceeded by `{name}` ({} bytes); raise MAX_HOST_CALL_REQUEST_BYTES in the host build",
						MAX_HOST_CALL_REQUEST_BYTES,
						payload.len()
					);
				}
				let Some(overlay) = st.overlay.clone() else {
					bail!("native plugin host call `{name}` is not registered");
				};
				let in_flight = MAX_IN_FLIGHT_HOST_CALLS - st.host_call_slots.available_permits();
				if in_flight >= HOST_CALL_WARN_AT {
					tracing::warn!(
						in_flight,
						limit = MAX_IN_FLIGHT_HOST_CALLS,
						callback = %name,
						"native plugin host-call concurrency is near its limit"
					);
				}
				let _permit = st.host_call_slots.clone().try_acquire_owned().map_err(|_| {
					anyhow!(
						"native plugin host-call concurrency limit {MAX_IN_FLIGHT_HOST_CALLS} exceeded by `{name}`; raise MAX_IN_FLIGHT_HOST_CALLS in the host build"
					)
				})?;
				let abort = st.ctx.actor_abort_signal();
				let response = tokio::select! {
					biased;
					_ = abort.cancelled() => bail!("native plugin host call `{name}` cancelled during actor shutdown"),
					result = tokio::time::timeout(
						HOST_CALL_TIMEOUT,
						overlay.host_call(st.ctx.clone(), name.clone(), payload),
					) => result
						.map_err(|_| anyhow!(
							"native plugin host-call timeout of {} ms exceeded by `{name}`; raise HOST_CALL_TIMEOUT in the host build",
							HOST_CALL_TIMEOUT.as_millis()
						))??,
				};
				if response.len() > MAX_HOST_CALL_RESPONSE_BYTES {
					bail!(
						"native plugin host-call response limit {} bytes exceeded by `{name}` ({} bytes); raise MAX_HOST_CALL_RESPONSE_BYTES in the host build",
						MAX_HOST_CALL_RESPONSE_BYTES,
						response.len()
					);
				}
				Ok(response)
			}
			.await;
			match result {
				Ok(bytes) => ok_bytes(bytes),
				Err(error) => abi::AbiResult::err(encode_db_error(&error)),
			}
		});
	}));
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

struct ForwardedEvent {
	tag: u32,
	payload: Vec<u8>,
	reply: PendingReply,
}

/// Convert one core event into the generic pushed-event ABI. Events owned by
/// RivetKit itself are answered inline and never enter the plugin.
fn forward_actor_event(event: ActorEvent) -> Option<ForwardedEvent> {
	use abi::AbiEventTag as Tag;
	match event {
		ActorEvent::Action {
			name,
			args,
			conn,
			reply,
		} => match abi::encode_action_payload_with_conn(
			&name,
			&args,
			conn.as_ref().map(conn_info).as_ref(),
		) {
			Ok(payload) => Some(ForwardedEvent {
				tag: Tag::Action as u32,
				payload,
				reply: PendingReply::Bytes(reply),
			}),
			Err(error) => {
				reply.send(Err(error));
				None
			}
		},
		ActorEvent::ConnectionPreflight {
			conn,
			params,
			request,
			reply,
		} => match abi::encode_conn_preflight_payload_with_request(
			&conn_info(&conn),
			&params,
			request.as_ref().map(encode_http_request).as_deref(),
		) {
			Ok(payload) => Some(ForwardedEvent {
				tag: Tag::ConnPreflight as u32,
				payload,
				reply: PendingReply::Unit(reply),
			}),
			Err(error) => {
				reply.send(Err(error));
				None
			}
		},
		ActorEvent::ConnectionOpen {
			conn,
			request,
			reply,
		} => {
			match abi::encode_conn_open_payload_with_request(
				&conn_info(&conn),
				request.as_ref().map(encode_http_request).as_deref(),
			) {
				Ok(payload) => Some(ForwardedEvent {
					tag: Tag::ConnOpen as u32,
					payload,
					reply: PendingReply::Unit(reply),
				}),
				Err(error) => {
					reply.send(Err(error));
					None
				}
			}
		}
		ActorEvent::SubscribeRequest {
			conn,
			reply,
			event_name,
		} => match abi::encode_subscribe_payload(&conn_info(&conn), &event_name) {
			Ok(payload) => Some(ForwardedEvent {
				tag: Tag::Subscribe as u32,
				payload,
				reply: PendingReply::Unit(reply),
			}),
			Err(error) => {
				reply.send(Err(error));
				None
			}
		},
		ActorEvent::QueueSend {
			name,
			body,
			conn,
			request,
			wait,
			timeout_ms,
			reply,
		} => {
			match abi::encode_queue_send_payload(
				&name,
				&body,
				&conn_info(&conn),
				&encode_http_request(&request),
				wait,
				timeout_ms,
			) {
				Ok(payload) => Some(ForwardedEvent {
					tag: Tag::QueueSend as u32,
					payload,
					reply: PendingReply::Queue(reply),
				}),
				Err(error) => {
					reply.send(Err(error));
					None
				}
			}
		}
		ActorEvent::WebSocketOpen {
			conn,
			request,
			reply,
			..
		} => {
			let request = request.as_ref().map(encode_http_request);
			match abi::encode_ws_open_payload(&conn_info(&conn), request.as_deref()) {
				Ok(payload) => Some(ForwardedEvent {
					tag: Tag::WsOpen as u32,
					payload,
					reply: PendingReply::Unit(reply),
				}),
				Err(error) => {
					reply.send(Err(error));
					None
				}
			}
		}
		ActorEvent::RunGracefulCleanup { reason, reply } => {
			let tag = match reason {
				ShutdownKind::Sleep => Tag::Sleep,
				ShutdownKind::Destroy => Tag::Destroy,
			};
			Some(ForwardedEvent {
				tag: tag as u32,
				payload: Vec::new(),
				reply: PendingReply::Unit(reply),
			})
		}
		ActorEvent::ConnectionClosed { conn } => {
			match abi::encode_conn_closed_payload(&conn_info(&conn)) {
				Ok(payload) => Some(ForwardedEvent {
					tag: Tag::ConnClosed as u32,
					payload,
					reply: PendingReply::None,
				}),
				Err(error) => {
					tracing::error!(?error, "failed to encode connection closed event");
					None
				}
			}
		}

		ActorEvent::HttpRequest { request, reply } => Some(ForwardedEvent {
			tag: Tag::Http as u32,
			payload: encode_http_request(&request),
			reply: PendingReply::Http(reply),
		}),
		ActorEvent::SerializeState { reply, .. } => Some(ForwardedEvent {
			tag: Tag::SerializeState as u32,
			payload: Vec::new(),
			reply: PendingReply::State(reply),
		}),

		ActorEvent::DisconnectConn { reply, .. } => {
			reply.send(Ok(()));
			None
		}
		ActorEvent::WorkflowHistoryRequested { reply } => {
			reply.send(Ok(None));
			None
		}
		ActorEvent::WorkflowReplayRequested { reply, .. } => {
			reply.send(Ok(None));
			None
		}

		#[cfg(test)]
		ActorEvent::BeginSleep => None,
		#[cfg(test)]
		ActorEvent::FinalizeSleep { reply } => {
			reply.send(Ok(()));
			None
		}
		#[cfg(test)]
		ActorEvent::Destroy { reply } => {
			reply.send(Ok(()));
			None
		}
	}
}

use std::sync::atomic::{AtomicU64, Ordering};

use crate::actor::lifecycle_hooks::Reply;

enum PendingReply {
	None,
	Bytes(Reply<Vec<u8>>),
	Unit(Reply<()>),
	State(Reply<Vec<StateDelta>>),
	Http(Reply<Response>),
	Queue(Reply<QueueSendResult>),
}

impl PendingReply {
	fn fulfill(self, done: PluginDone) -> Result<()> {
		if done.status == abi::AbiStatus::Ok {
			self.fulfill_ok(done.payload);
			return Ok(());
		}
		let message = if done.payload.is_empty() && done.status == abi::AbiStatus::ChannelClosed {
			"native plugin event dropped without a response".to_owned()
		} else if done.payload.is_empty() {
			format!("native plugin event completed with {:?}", done.status)
		} else {
			String::from_utf8_lossy(&done.payload).into_owned()
		};
		let no_reply = matches!(self, PendingReply::None);
		self.fulfill_err(message.clone());
		if no_reply {
			Err(anyhow!("{message}"))
		} else {
			Ok(())
		}
	}

	fn fulfill_ok(self, payload: Vec<u8>) {
		match self {
			PendingReply::None => {}
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
			PendingReply::None => {}
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
mod native_plugin_tests {
	use super::*;

	fn current_plugin_api_header() -> PluginApiHeader {
		PluginApiHeader {
			abi_magic: abi::RIVET_ACTOR_ABI_MAGIC,
			abi_version: abi::RIVET_ACTOR_ABI_VERSION,
			struct_size: std::mem::size_of::<abi::PluginApi>(),
		}
	}

	#[test]
	fn plugin_api_header_requires_exact_magic_version_and_size() {
		validate_plugin_api_header(current_plugin_api_header()).expect("current header");

		let mut header = current_plugin_api_header();
		header.abi_magic ^= 1;
		assert!(
			validate_plugin_api_header(header)
				.unwrap_err()
				.to_string()
				.contains("not a rivet actor plugin")
		);

		let mut header = current_plugin_api_header();
		header.abi_version -= 1;
		assert!(
			validate_plugin_api_header(header)
				.unwrap_err()
				.to_string()
				.contains("same-version lockstep")
		);

		let mut header = current_plugin_api_header();
		header.struct_size -= 1;
		assert!(
			validate_plugin_api_header(header)
				.unwrap_err()
				.to_string()
				.contains("descriptor")
		);
	}

	#[test]
	fn direct_completion_delivers_payload_without_a_host_reply_slab() {
		let (tx, mut rx) = tokio::sync::oneshot::channel::<anyhow::Result<Vec<u8>>>();
		PendingReply::Bytes(Reply::from(tx))
			.fulfill(PluginDone {
				status: abi::AbiStatus::Ok,
				payload: vec![1, 2, 3],
			})
			.expect("direct completion");
		let got = rx.try_recv().expect("sent").expect("ok");
		assert_eq!(got, vec![1, 2, 3]);
	}
}
