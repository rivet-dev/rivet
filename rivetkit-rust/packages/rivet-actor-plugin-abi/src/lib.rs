//! Stable C-ABI contract for RivetKit native actor plugins loaded via `dlopen`.
//!
//! This crate is the **generic** contract shared, in lockstep, by the RivetKit
//! host (which `dlopen`s a plugin) and an actor plugin `cdylib`.
//! It contains **no** product-specific or business logic — only:
//!
//! 1. the ABI version + magic constants (§4.1 of the dylib-actor-plugin spec),
//! 2. the `#[repr(C)]` boundary types (buffers, results, events, the host
//!    vtable),
//! 3. the exported plugin symbol names the host resolves after `dlopen`,
//! 4. (added in a follow-up module) the generic actor wire codec.
//!
//! ## Memory & safety model (normative)
//!
//! - Only `#[repr(C)]` scalars, opaque pointers, and length-prefixed byte
//!   buffers cross the boundary. No `String`/`Vec`/`Box<dyn>`/`Future`/
//!   `serde_json::Value` is ever passed by value.
//! - **One free path:** every [`OwnedBuf`] is freed exactly once by calling its
//!   own non-optional `free` pointer. This is allocator-correct regardless of
//!   which side produced the buffer (the two binaries link independent
//!   allocators).
//! - Inputs to an **async submit** transfer ownership via [`OwnedBuf`] (the
//!   callee frees them when done). [`BorrowedBuf`] is only valid for the
//!   duration of a **synchronous** call.
//! - Every `extern "C"` boundary (both directions, incl. completion callbacks)
//!   must wrap its body in `catch_unwind`; a panic across `extern "C"` is UB.

#![allow(clippy::missing_safety_doc)]
// FFI glue: these `unsafe fn`s are unsafe in their entirety by design; an
// inner-block-per-op convention adds noise without adding safety here.
#![allow(unsafe_op_in_unsafe_fn)]

pub mod codec;
pub mod portable;

use std::ffi::c_void;

pub use portable::{
	ActorIdentity, Backend, ConnDisconnectRequest, ConnInfo, ConnListResponse, ConnSendRequest,
	DylibBackend, Event, KeepAwakeToken, KvEntriesRequest, KvEntry, KvGetResponse, KvKeyRequest,
	KvKeysRequest, KvListOpts, KvListPrefixRequest, KvListRangeRequest, KvListResponse,
	KvRangeRequest, KvValuesResponse, PortableActorBackend, PortableActorCtx, PortableBoxFuture,
	QueueSendResponse, ReplyToken, RequestSaveOpts, ScheduleActionRequest, ScheduleAlarmRequest,
	ScheduledEvent, ScheduledEventsResponse, WsOpenPayload, decode_action_payload,
	decode_event_frame, encode_action_payload, encode_conn_closed_payload,
	encode_conn_open_payload, encode_conn_preflight_payload, encode_event_frame,
	encode_queue_send_payload, encode_queue_send_response, encode_subscribe_payload,
	encode_ws_open_payload,
};

/// Bumped on ANY change to the structs, symbol signatures, the wire codec, or
/// the event-tag enum in this crate. The host refuses to load a plugin whose
/// reported version != this. No negotiation, no fallback (same-version
/// lockstep, matching the project's wire-protocol rule).
pub const RIVET_ACTOR_ABI_VERSION: u64 = 13;

/// Magic returned by [`SYM_ABI_MAGIC`] to detect "this `.so` is not a rivet
/// actor plugin at all" before any other symbol is called. ASCII "RVTABI\0\1".
pub const RIVET_ACTOR_ABI_MAGIC: u64 = 0x52_56_54_41_42_49_00_01;

/// Exported plugin symbol names the host resolves with `dlsym` after `dlopen`.
/// Kept as constants so host and plugin can never disagree on spelling.
pub mod symbols {
	/// `extern "C" fn() -> u64` — cheap, no allocation, no fallible work.
	/// Host calls this FIRST and aborts the load if it != [`super::RIVET_ACTOR_ABI_MAGIC`].
	pub const ABI_MAGIC: &[u8] = b"rivet_actor_abi_magic\0";
	/// `extern "C" fn() -> u64` — must equal [`super::RIVET_ACTOR_ABI_VERSION`].
	pub const ABI_VERSION: &[u8] = b"rivet_actor_abi_version\0";
	/// `extern "C" fn(out_err: *mut OwnedBuf) -> *mut c_void` — once per load.
	pub const PLUGIN_INIT: &[u8] = b"rivet_actor_plugin_init\0";
	/// `extern "C" fn(plugin, config_json: BorrowedBuf, sidecar_path: BorrowedBuf, out_err: *mut OwnedBuf) -> *mut c_void`.
	pub const FACTORY_NEW: &[u8] = b"rivet_actor_factory_new\0";
	/// `extern "C" fn(factory, host: *const HostVtable, done: CompletionFn, user_data: *mut c_void) -> *mut c_void`.
	pub const RUN: &[u8] = b"rivet_actor_run\0";
	/// `extern "C" fn(instance: *mut c_void)` — closes the event stream.
	pub const CANCEL: &[u8] = b"rivet_actor_cancel\0";
	/// `extern "C" fn(instance: *mut c_void)` — force VM teardown on grace deadline.
	pub const GRACE_DEADLINE: &[u8] = b"rivet_actor_grace_deadline\0";
	/// `extern "C" fn(instance: *mut c_void)` — only after `run`'s completion fired.
	pub const INSTANCE_FREE: &[u8] = b"rivet_actor_instance_free\0";
	/// `extern "C" fn(factory: *mut c_void)`.
	pub const FACTORY_FREE: &[u8] = b"rivet_actor_factory_free\0";
	/// `extern "C" fn(plugin: *mut c_void)` — drains the plugin runtime.
	pub const PLUGIN_SHUTDOWN: &[u8] = b"rivet_actor_plugin_shutdown\0";
}

/// Borrowed, immutable bytes. Valid ONLY for the duration of a **synchronous**
/// call. MUST NOT be used as input to an async submit (the bytes would be read
/// after the submit returns — use-after-free). Use [`OwnedBuf`] there.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BorrowedBuf {
	pub ptr: *const u8,
	pub len: usize,
}

impl BorrowedBuf {
	/// Borrow a slice for the duration of a synchronous call. The slice MUST
	/// outlive the call.
	pub fn from_slice(s: &[u8]) -> Self {
		Self {
			ptr: s.as_ptr(),
			len: s.len(),
		}
	}

	/// # Safety
	/// `ptr`/`len` must describe a valid, initialized region that outlives the
	/// returned slice's use.
	pub unsafe fn as_slice<'a>(&self) -> &'a [u8] {
		if self.len == 0 {
			&[]
		} else {
			std::slice::from_raw_parts(self.ptr, self.len)
		}
	}
}

/// Owned bytes. Freed EXACTLY ONCE by calling its own `free` (non-optional).
/// Empty buffers use [`noop_free`], never a null `free`.
#[repr(C)]
pub struct OwnedBuf {
	pub ptr: *mut u8,
	pub len: usize,
	pub cap: usize,
	/// Frees this buffer with the allocator of the side that produced it.
	pub free: extern "C" fn(ptr: *mut u8, len: usize, cap: usize),
}

/// `free` impl for an [`OwnedBuf`] backed by a Rust `Vec<u8>` allocated on
/// *this* side. Each side uses its own `from_vec`/`free` pair, so frees are
/// always allocator-correct.
pub extern "C" fn free_rust_vec(ptr: *mut u8, len: usize, cap: usize) {
	if !ptr.is_null() && cap != 0 {
		// SAFETY: ptr/len/cap came from `Vec::into_raw_parts` on this side.
		unsafe {
			drop(Vec::from_raw_parts(ptr, len, cap));
		}
	}
}

/// `free` impl for empty/borrowed-dangling buffers; does nothing.
pub extern "C" fn noop_free(_ptr: *mut u8, _len: usize, _cap: usize) {}

impl OwnedBuf {
	/// Transfer ownership of a `Vec<u8>` across the boundary. The receiver must
	/// call [`OwnedBuf::free_self`] (or consume via [`OwnedBuf::into_vec`])
	/// exactly once.
	pub fn from_vec(mut v: Vec<u8>) -> Self {
		if v.capacity() == 0 {
			return Self::empty();
		}
		let ptr = v.as_mut_ptr();
		let len = v.len();
		let cap = v.capacity();
		std::mem::forget(v);
		Self {
			ptr,
			len,
			cap,
			free: free_rust_vec,
		}
	}

	/// An empty buffer with a no-op free.
	pub fn empty() -> Self {
		Self {
			ptr: std::ptr::NonNull::dangling().as_ptr(),
			len: 0,
			cap: 0,
			free: noop_free,
		}
	}

	/// View the bytes without taking ownership.
	///
	/// # Safety
	/// `self` must be a valid buffer that outlives the returned slice.
	pub unsafe fn as_slice(&self) -> &[u8] {
		if self.len == 0 {
			&[]
		} else {
			std::slice::from_raw_parts(self.ptr, self.len)
		}
	}

	/// Take ownership of the bytes IF this buffer was produced on this side via
	/// [`OwnedBuf::from_vec`] (i.e. `free == free_rust_vec`). Otherwise copies
	/// the bytes and frees the original with its own `free`.
	///
	/// # Safety
	/// `self` must be a valid, not-yet-freed buffer. Consumes it (do not free
	/// again).
	pub unsafe fn into_vec(self) -> Vec<u8> {
		if self.cap == 0 {
			return Vec::new();
		}
		if self.free as *const () as usize == free_rust_vec as *const () as usize {
			Vec::from_raw_parts(self.ptr, self.len, self.cap)
		} else {
			// Cross-side buffer: copy out, then free with the producer's free.
			let copy = std::slice::from_raw_parts(self.ptr, self.len).to_vec();
			(self.free)(self.ptr, self.len, self.cap);
			copy
		}
	}

	/// Free this buffer via its own allocator. Call exactly once.
	///
	/// # Safety
	/// `self` must be a valid, not-yet-freed buffer; do not use it afterward.
	pub unsafe fn free_self(self) {
		(self.free)(self.ptr, self.len, self.cap);
	}
}

/// Status of a completed cross-boundary operation.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbiStatus {
	Ok = 0,
	/// Application error; `payload` carries a structured error (see codec).
	Err = 1,
	/// A panic was caught at a boundary; the operation did not complete.
	Panic = 2,
	/// The operation was cancelled.
	Cancelled = 3,
	/// The event stream is closed (terminal for `next_event`).
	ChannelClosed = 4,
}

/// Result of an async submit: status + payload. The completion callback ALWAYS
/// takes ownership of `payload` and frees it, on every path.
#[repr(C)]
pub struct AbiResult {
	pub status: AbiStatus,
	pub payload: OwnedBuf,
}

impl AbiResult {
	pub fn ok(payload: OwnedBuf) -> Self {
		Self {
			status: AbiStatus::Ok,
			payload,
		}
	}
	pub fn err(payload: OwnedBuf) -> Self {
		Self {
			status: AbiStatus::Err,
			payload,
		}
	}
	pub fn status_only(status: AbiStatus) -> Self {
		Self {
			status,
			payload: OwnedBuf::empty(),
		}
	}
	pub fn channel_closed() -> Self {
		Self::status_only(AbiStatus::ChannelClosed)
	}
}

/// Called EXACTLY once, from any thread, when an async op completes. Reclaims
/// `user_data`, takes ownership of `result.payload` (frees it), and MUST be
/// wrapped in `catch_unwind` by the implementer.
pub type CompletionFn = extern "C" fn(user_data: *mut c_void, result: AbiResult);

/// A lifecycle event delivered to the plugin via `next_event`. Encoded into a
/// [`CompletionFn`] `AbiResult` payload as `(tag, reply_token, event_bytes)`.
/// `reply_token == 0` means the event needs no reply.
#[repr(C)]
pub struct AbiEvent {
	pub tag: u32,
	pub reply_token: u64,
	pub payload: OwnedBuf,
}

/// Generic actor lifecycle event tags, generated to match RivetKit's
/// `RuntimeEvent`. These are generic and product-agnostic. Payloads are opaque
/// bytes; reply semantics are documented per tag in the spec (§4.6).
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbiEventTag {
	Action = 0,
	Http = 1,
	Subscribe = 2,
	ConnOpen = 3,
	ConnClosed = 4,
	QueueSend = 5,
	WsOpen = 6,
	SerializeState = 7,
	Sleep = 8,
	Destroy = 9,
	ConnPreflight = 10,
}

impl AbiEventTag {
	pub fn from_u32(v: u32) -> Option<Self> {
		Some(match v {
			0 => Self::Action,
			1 => Self::Http,
			2 => Self::Subscribe,
			3 => Self::ConnOpen,
			4 => Self::ConnClosed,
			5 => Self::QueueSend,
			6 => Self::WsOpen,
			7 => Self::SerializeState,
			8 => Self::Sleep,
			9 => Self::Destroy,
			10 => Self::ConnPreflight,
			_ => return None,
		})
	}

	/// Whether the host expects a `reply_ok`/`reply_err` for this event.
	pub fn needs_reply(self) -> bool {
		!matches!(self, Self::ConnClosed)
	}
}

// --- Host vtable fn-pointer aliases (plugin -> host) ---

/// Async submit (`db_exec`): `(ctx, sql, done, user_data)`. `sql` ownership is
/// transferred (host frees it when the spawned task is done reading).
pub type DbExecFn =
	extern "C" fn(ctx: *const c_void, sql: OwnedBuf, done: CompletionFn, user_data: *mut c_void);
/// Async submit (`db_query`/`db_run`): `(ctx, sql, params, done, user_data)`.
pub type DbSqlFn = extern "C" fn(
	ctx: *const c_void,
	sql: OwnedBuf,
	params: OwnedBuf,
	done: CompletionFn,
	user_data: *mut c_void,
);
/// Sync actor-state snapshot read: `(ctx) -> state_bytes`.
pub type StateGetFn = extern "C" fn(ctx: *const c_void) -> OwnedBuf;
/// Sync actor-state mutation: `(ctx, state_bytes) -> AbiStatus`.
pub type StateSetFn = extern "C" fn(ctx: *const c_void, state: OwnedBuf) -> AbiStatus;
/// Sync actor identity snapshot read: `(ctx) -> cbor(ActorIdentity)`.
pub type ActorIdentityFn = extern "C" fn(ctx: *const c_void) -> OwnedBuf;
/// Async actor-state save: `(ctx, state_bytes, done, user_data)`.
pub type StateSaveFn =
	extern "C" fn(ctx: *const c_void, state: OwnedBuf, done: CompletionFn, user_data: *mut c_void);
/// Sync save request: `(ctx, immediate, has_max_wait, max_wait_ms) -> AbiStatus`.
pub type RequestSaveFn = extern "C" fn(
	ctx: *const c_void,
	immediate: u8,
	has_max_wait: u8,
	max_wait_ms: u32,
) -> AbiStatus;
/// Async save request with completion: `(ctx, immediate, has_max_wait, max_wait_ms, done, user_data)`.
pub type RequestSaveAndWaitFn = extern "C" fn(
	ctx: *const c_void,
	immediate: u8,
	has_max_wait: u8,
	max_wait_ms: u32,
	done: CompletionFn,
	user_data: *mut c_void,
);
/// Sync actor sleep request: `(ctx) -> AbiResult`.
pub type SleepFn = extern "C" fn(ctx: *const c_void) -> AbiResult;
/// Sync actor-abort snapshot: `(ctx) -> 0/1`.
pub type ActorAbortedFn = extern "C" fn(ctx: *const c_void) -> u8;
/// Async actor-abort wait: `(ctx, done, user_data)`.
pub type WaitActorAbortFn =
	extern "C" fn(ctx: *const c_void, done: CompletionFn, user_data: *mut c_void);
/// Sync keep-awake enter: `(ctx) -> cbor(KeepAwakeToken)`.
pub type KeepAwakeEnterFn = extern "C" fn(ctx: *const c_void) -> AbiResult;
/// Sync keep-awake exit: `(ctx, token) -> AbiStatus`.
pub type KeepAwakeExitFn = extern "C" fn(ctx: *const c_void, token: u64) -> AbiStatus;
/// Sync keep-awake count: `(ctx) -> active_count`.
pub type KeepAwakeCountFn = extern "C" fn(ctx: *const c_void) -> u64;
/// Async KV operation: `(ctx, cbor_request, done, user_data)`.
pub type KvOpFn = extern "C" fn(
	ctx: *const c_void,
	request: OwnedBuf,
	done: CompletionFn,
	user_data: *mut c_void,
);
/// Async scheduling operation: `(ctx, cbor_request, done, user_data)`.
pub type ScheduleOpFn = extern "C" fn(
	ctx: *const c_void,
	request: OwnedBuf,
	done: CompletionFn,
	user_data: *mut c_void,
);
/// Async connection operation: `(ctx, cbor_request, done, user_data)`.
pub type ConnOpFn = extern "C" fn(
	ctx: *const c_void,
	request: OwnedBuf,
	done: CompletionFn,
	user_data: *mut c_void,
);
/// Sync hibernatable websocket ack: `(ctx, gateway_id, request_id, server_message_index)`.
pub type HibernatableAckFn = extern "C" fn(
	ctx: *const c_void,
	gateway_id: OwnedBuf,
	request_id: OwnedBuf,
	server_message_index: u16,
) -> AbiResult;
/// Sync connection send: `(ctx, cbor_request) -> AbiResult`.
pub type ConnSendFn = extern "C" fn(ctx: *const c_void, request: OwnedBuf) -> AbiResult;
/// `(ctx) -> 0|1`.
pub type SqlEnabledFn = extern "C" fn(ctx: *const c_void) -> u8;
/// Async pull of the next event: `(ctx, done, user_data)`. Completes with an
/// `AbiResult` whose payload encodes an [`AbiEvent`], or `ChannelClosed`.
pub type NextEventFn =
	extern "C" fn(ctx: *const c_void, done: CompletionFn, user_data: *mut c_void);
/// Sync event reply: `(ctx, reply_token, payload) -> AbiStatus`.
pub type ReplyFn =
	extern "C" fn(ctx: *const c_void, reply_token: u64, payload: OwnedBuf) -> AbiStatus;
/// Sync, runtime-free broadcast: `(ctx, name, payload) -> AbiStatus`.
pub type BroadcastFn =
	extern "C" fn(ctx: *const c_void, name: OwnedBuf, payload: OwnedBuf) -> AbiStatus;
/// Sync structured log: `(ctx, level, msg)`.
pub type LogFn = extern "C" fn(ctx: *const c_void, level: i32, msg: BorrowedBuf);
/// Refcount the opaque host ctx handle (clone/release).
pub type CtxRefFn = extern "C" fn(ctx: *const c_void) -> *const c_void;
pub type CtxReleaseFn = extern "C" fn(ctx: *const c_void);
/// Signal the plugin that startup is ready (manual-startup mode).
pub type StartupReadyFn = extern "C" fn(ctx: *const c_void, ok: u8, err_msg: BorrowedBuf);

/// The capabilities the host exposes to a running plugin actor. Built by the
/// host over a live `ActorContext`; the `ctx` handle is OWNED + refcounted
/// (the plugin may `ctx_clone` it into detached `'static` tasks, and the host
/// keeps the underlying `ActorContext` alive until the last `ctx_release`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HostVtable {
	pub abi_version: u64,

	/// Opaque, refcounted handle to the host `ActorContext`.
	pub ctx: *const c_void,
	pub ctx_clone: CtxRefFn,
	pub ctx_release: CtxReleaseFn,

	pub db_exec: DbExecFn,
	pub db_query: DbSqlFn,
	pub db_run: DbSqlFn,
	pub sql_is_enabled: SqlEnabledFn,

	pub state_get: StateGetFn,
	pub state_set: StateSetFn,
	pub actor_identity: ActorIdentityFn,
	pub state_save: StateSaveFn,
	pub request_save: RequestSaveFn,
	pub request_save_and_wait: RequestSaveAndWaitFn,
	pub sleep: SleepFn,
	pub actor_aborted: ActorAbortedFn,
	pub wait_actor_abort: WaitActorAbortFn,
	pub keep_awake_enter: KeepAwakeEnterFn,
	pub keep_awake_exit: KeepAwakeExitFn,
	pub keep_awake_count: KeepAwakeCountFn,

	pub kv_get: KvOpFn,
	pub kv_put: KvOpFn,
	pub kv_delete: KvOpFn,
	pub kv_batch_get: KvOpFn,
	pub kv_batch_put: KvOpFn,
	pub kv_batch_delete: KvOpFn,
	pub kv_delete_range: KvOpFn,
	pub kv_list_prefix: KvOpFn,
	pub kv_list_range: KvOpFn,

	pub schedule_after: ScheduleOpFn,
	pub schedule_at: ScheduleOpFn,
	pub set_alarm: ScheduleOpFn,
	pub scheduled_events: ScheduleOpFn,

	pub conn_list: ConnOpFn,
	pub conn_disconnect: ConnOpFn,
	pub hibernatable_ws_ack: HibernatableAckFn,
	pub conn_send: ConnSendFn,

	pub next_event: NextEventFn,
	pub reply_ok: ReplyFn,
	pub reply_err: ReplyFn,
	pub startup_ready: StartupReadyFn,

	pub broadcast: BroadcastFn,
	pub log: LogFn,
}

unsafe impl Send for HostVtable {}
unsafe impl Sync for HostVtable {}

/// Host-side actor knobs that the plugin's factory reports back so the host can
/// apply them (these live host-side today in `build_core_factory`). Carried in
/// the config envelope / descriptor rather than across the C ABI by value.
#[derive(Clone, Copy, Debug)]
pub struct ActorConfigKnobs {
	pub has_database: bool,
	pub sleep_grace_period_ms: u64,
	pub action_timeout_ms: u64,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn owned_buf_roundtrips_rust_vec() {
		let buf = OwnedBuf::from_vec(vec![1u8, 2, 3, 4]);
		let v = unsafe { buf.into_vec() };
		assert_eq!(v, vec![1, 2, 3, 4]);
	}

	#[test]
	fn owned_buf_empty_is_safe_to_free() {
		let buf = OwnedBuf::empty();
		unsafe { buf.free_self() };
	}

	#[test]
	fn event_tag_roundtrip_and_reply_semantics() {
		for v in 0u32..=10 {
			let tag = AbiEventTag::from_u32(v).expect("known tag");
			assert_eq!(tag as u32, v);
		}
		assert!(AbiEventTag::from_u32(11).is_none());
		assert!(!AbiEventTag::ConnClosed.needs_reply());
		assert!(AbiEventTag::Action.needs_reply());
	}
}
