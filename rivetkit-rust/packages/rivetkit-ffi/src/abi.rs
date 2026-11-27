//! The exported C ABI. This is the only module allowed to use `unsafe`, and it
//! contains exactly one `unsafe` block: [`input`]. `#[unsafe(no_mangle)]` is also
//! an unsafe attribute under edition 2024, which is why every export lives here.
//!
//! Every export converts its arguments and calls `rivetkit-host-adapter`'s wire.
//! The protocol (event kinds, ops, headers, leases) is documented there.
//!
//! Rules the host must follow:
//! - Every `(ptr, len)` pair points to `len` readable bytes for the duration of the
//!   call, or `len` is 0. Rust copies what it needs before returning.
//! - Callback buffers with `lease == 0` are valid only during the callback. Leased
//!   buffers stay valid until `rivet_lease_release(lease)`.
//! - Callbacks must only enqueue work and return; they run on runtime threads.
//!   They must not unwind (Java: catch every `Throwable`).
//! - Handles are opaque `u64` ids. 0 is never a valid handle.
//! - A non-zero status means the call did nothing and its token (if any) will not
//!   complete; `rivet_last_error(token)` then completes it with the error.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use anyhow::{Result, anyhow};
use rivetkit_host_adapter::wire::messages::{InitOptions, decode};

use crate::runtime;

pub const STATUS_OK: i32 = 0;
pub const STATUS_ERROR: i32 = 1;

/// `on_event(kind, actor_type, actor, id, header, header_len, body, body_len,
/// lease)`: delivers an event (see `rivet_event_kind`). Must return quickly and
/// never unwind.
pub type EventCallback =
	Option<extern "C" fn(u32, u64, u64, u64, *const u8, usize, *const u8, usize, u64)>;

/// `on_complete(token, status, header, header_len, body, body_len, lease)`:
/// completes a call. Status 0: header and body are the op's result. Otherwise the
/// header is a CBOR `{group, code, message, metadata?}`. May run on the calling
/// thread before the call returns. Must never unwind.
pub type CompleteCallback =
	Option<extern "C" fn(u64, i32, *const u8, usize, *const u8, usize, u64)>;

thread_local! {
	static LAST_ERROR: RefCell<Option<anyhow::Error>> = const { RefCell::new(None) };
}

/// Borrows host bytes as a slice. The single `unsafe` block in this crate.
fn input<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8]> {
	if len == 0 {
		return Ok(&[]);
	}
	if ptr.is_null() {
		return Err(anyhow!("null pointer with non-zero length {len}"));
	}
	if len > isize::MAX as usize {
		return Err(anyhow!("length {len} exceeds isize::MAX"));
	}
	// SAFETY: `ptr` is non-null and `len` fits in isize (checked above). The ABI
	// contract (module docs) requires the host to pass `len` initialized, readable
	// bytes that stay valid and unmodified for the whole call. Every caller copies
	// or decodes the slice before returning, so the borrow never outlives the call.
	Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// Runs `f`, turning errors and panics into `STATUS_ERROR` plus a stored last
/// error. Every export goes through here (`exports_are_guarded` checks).
fn status(f: impl FnOnce() -> Result<()>) -> i32 {
	let result = match catch_unwind(AssertUnwindSafe(f)) {
		Ok(result) => result,
		Err(panic) => {
			let message = panic
				.downcast_ref::<&str>()
				.map(|s| (*s).to_owned())
				.or_else(|| panic.downcast_ref::<String>().cloned())
				.unwrap_or_else(|| "unknown panic".to_owned());
			Err(anyhow!("panic in rivetkit-ffi: {message}"))
		}
	};
	match result {
		Ok(()) => STATUS_OK,
		Err(error) => {
			LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(error));
			STATUS_ERROR
		}
	}
}

/// The ABI version this library implements. The host checks it before
/// `rivet_init` and refuses to run on a mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn rivet_abi_version() -> u32 {
	rivetkit_host_adapter::wire::ABI_VERSION
}

/// Starts the runtime and installs the host callbacks. `options` is a CBOR
/// `{logFilter?}` (may be empty). Safe to call again with the same callbacks.
#[unsafe(no_mangle)]
pub extern "C" fn rivet_init(
	on_event: EventCallback,
	on_complete: CompleteCallback,
	options: *const u8,
	options_len: usize,
) -> i32 {
	status(|| {
		let on_event = on_event.ok_or_else(|| anyhow!("on_event is null"))?;
		let on_complete = on_complete.ok_or_else(|| anyhow!("on_complete is null"))?;
		let options: InitOptions = decode(input(options, options_len)?, "init options")?;
		runtime::init(on_event, on_complete, options)
	})
}

/// Completes `token` with this thread's last error (and clears it). Returns
/// non-zero when there is no error or the runtime is not running.
#[unsafe(no_mangle)]
pub extern "C" fn rivet_last_error(token: u64) -> i32 {
	status(|| {
		let error = LAST_ERROR
			.with(|slot| slot.borrow_mut().take())
			.ok_or_else(|| anyhow!("no error"))?;
		runtime::get()?.wire.fail(token, error);
		Ok(())
	})
}

/// Runs an op (see `rivet_op`) against `target`. The result arrives through
/// `on_complete(token)`, possibly before this returns.
#[unsafe(no_mangle)]
pub extern "C" fn rivet_call(
	target: u64,
	op: u32,
	header: *const u8,
	header_len: usize,
	body: *const u8,
	body_len: usize,
	token: u64,
) -> i32 {
	status(|| {
		let header = input(header, header_len)?;
		let body = input(body, body_len)?;
		runtime::get()?.wire.call(target, op, header, body, token);
		Ok(())
	})
}

/// Answers an event delivered through `on_event`.
#[unsafe(no_mangle)]
pub extern "C" fn rivet_reply(
	event: u64,
	status_code: i32,
	header: *const u8,
	header_len: usize,
	body: *const u8,
	body_len: usize,
) -> i32 {
	status(|| {
		runtime::get()?.wire.reply(
			event,
			status_code,
			input(header, header_len)?,
			input(body, body_len)?,
		)
	})
}

/// Frees a leased buffer. For stream chunks, also returns the stream's credit.
#[unsafe(no_mangle)]
pub extern "C" fn rivet_lease_release(lease: u64) -> i32 {
	status(|| runtime::get()?.wire.lease_release(lease))
}

/// Releases a handle the host owns: registry (stops it), client, connection,
/// socket, stream (cancels it), region (ends it), transaction (rolls back).
#[unsafe(no_mangle)]
pub extern "C" fn rivet_release(handle: u64) -> i32 {
	status(|| runtime::get()?.wire.release(handle))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn input_accepts_empty_and_rejects_null() {
		assert_eq!(input(std::ptr::null(), 0).unwrap(), &[] as &[u8]);
		assert!(input(std::ptr::null(), 3).is_err());
		assert!(input(std::ptr::dangling::<u8>(), usize::MAX).is_err());
	}

	#[test]
	fn input_reads_bytes() {
		let data = [1u8, 2, 3];
		assert_eq!(input(data.as_ptr(), data.len()).unwrap(), &[1, 2, 3]);
	}

	#[test]
	fn status_catches_panics() {
		assert_eq!(status(|| panic!("boom")), STATUS_ERROR);
		let error = LAST_ERROR.with(|slot| slot.borrow_mut().take()).unwrap();
		assert!(error.to_string().contains("boom"));
	}

	/// Every export must run its body through `status` so no panic unwinds into
	/// the host. Checked on the source so a new export cannot skip it.
	#[test]
	fn exports_are_guarded() {
		let source = include_str!("abi.rs");
		let mut exports = 0;
		for chunk in source.split("\n#[unsafe(no_mangle)]\n").skip(1) {
			exports += 1;
			let body = chunk.split_once('{').map(|(_, body)| body).unwrap_or("");
			let first = body.trim_start();
			assert!(
				first.starts_with("status(")
					|| first.starts_with("rivetkit_host_adapter::wire::ABI_VERSION"),
				"export without a status guard: {}",
				chunk
					.lines()
					.find(|line| line.contains("extern \"C\" fn"))
					.unwrap_or("?")
			);
		}
		assert_eq!(exports, 7);
	}
}
