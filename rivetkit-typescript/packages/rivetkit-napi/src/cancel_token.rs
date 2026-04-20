use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

use napi::bindgen_prelude::BigInt;
use napi_derive::napi;
use scc::HashMap as SccHashMap;
use tokio_util::sync::CancellationToken;

static NEXT_CANCEL_TOKEN_ID: AtomicU64 = AtomicU64::new(1);
static CANCEL_TOKENS: LazyLock<SccHashMap<u64, CancellationToken>> =
	LazyLock::new(SccHashMap::new);
#[cfg(test)]
static CANCEL_TOKEN_TEST_LOCK: std::sync::atomic::AtomicBool =
	std::sync::atomic::AtomicBool::new(false);

pub(crate) struct CancelTokenGuard {
	pub(crate) id: u64,
}

pub(crate) fn register_token() -> (u64, CancellationToken) {
	let id = NEXT_CANCEL_TOKEN_ID.fetch_add(1, Ordering::Relaxed);
	let token = CancellationToken::new();
	let _ = CANCEL_TOKENS.insert_sync(id, token.clone());
	(id, token)
}

pub(crate) fn register_guarded_token() -> (CancelTokenGuard, CancellationToken) {
	let (id, token) = register_token();
	(CancelTokenGuard { id }, token)
}

#[cfg(test)]
pub(crate) fn active_token_count() -> usize {
	CANCEL_TOKENS.len()
}

pub(crate) fn lookup_token(id: u64) -> Option<CancellationToken> {
	CANCEL_TOKENS.read_sync(&id, |_, token| token.clone())
}

pub(crate) fn cancel(id: u64) {
	if let Some(token) = CANCEL_TOKENS.read_sync(&id, |_, token| token.clone()) {
		token.cancel();
	}
}

pub(crate) fn poll_cancelled(id: u64) -> bool {
	CANCEL_TOKENS
		.read_sync(&id, |_, token| token.is_cancelled())
		.unwrap_or(true)
}

pub(crate) fn drop_token(id: u64) {
	let _ = CANCEL_TOKENS.remove_sync(&id);
}

impl Drop for CancelTokenGuard {
	fn drop(&mut self) {
		cancel(self.id);
		drop_token(self.id);
	}
}

#[cfg(test)]
pub(crate) struct CancelTokenTestGuard;

#[cfg(test)]
pub(crate) fn lock_registry_for_test() -> CancelTokenTestGuard {
	while CANCEL_TOKEN_TEST_LOCK
		.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
		.is_err()
	{
		std::thread::yield_now();
	}

	CancelTokenTestGuard
}

#[cfg(test)]
impl Drop for CancelTokenTestGuard {
	fn drop(&mut self) {
		CANCEL_TOKEN_TEST_LOCK.store(false, Ordering::Release);
	}
}

fn parse_cancel_token_id(id: BigInt) -> Option<u64> {
	let (negative, token_id, lossless) = id.get_u64();
	if negative || !lossless {
		None
	} else {
		Some(token_id)
	}
}

#[napi]
pub fn poll_cancel_token(id: BigInt) -> bool {
	let Some(token_id) = parse_cancel_token_id(id) else {
		return true;
	};

	poll_cancelled(token_id)
}

#[napi]
pub fn register_native_cancel_token() -> BigInt {
	BigInt::from(register_token().0)
}

#[napi]
pub fn cancel_native_cancel_token(id: BigInt) {
	if let Some(token_id) = parse_cancel_token_id(id) {
		cancel(token_id);
	}
}

#[napi]
pub fn drop_native_cancel_token(id: BigInt) {
	if let Some(token_id) = parse_cancel_token_id(id) {
		drop_token(token_id);
	}
}

#[cfg(test)]
mod tests {
	use super::{
		active_token_count, cancel, drop_token, lock_registry_for_test,
		poll_cancelled, register_guarded_token, register_token,
	};

	#[test]
	fn cancel_token_registry_tracks_cancel_and_drop() {
		let _lock = lock_registry_for_test();
		let (first_id, _) = register_token();
		let (second_id, _) = register_token();

		assert_ne!(first_id, second_id);
		assert!(!poll_cancelled(first_id));

		cancel(first_id);
		assert!(poll_cancelled(first_id));

		drop_token(first_id);
		assert!(poll_cancelled(first_id));

		cancel(second_id);
		drop_token(second_id);
	}

	#[test]
	fn guarded_token_drop_cancels_and_removes_token() {
		let _lock = lock_registry_for_test();
		let baseline = active_token_count();
		let (guard, _token) = register_guarded_token();
		let guard_id = guard.id;

		assert_eq!(active_token_count(), baseline + 1);
		assert!(!poll_cancelled(guard_id));

		std::mem::drop(guard);

		assert!(poll_cancelled(guard_id));
		assert_eq!(active_token_count(), baseline);

		let (next_id, _token) = register_token();
		assert!(next_id > guard_id);
		drop_token(next_id);
	}
}
