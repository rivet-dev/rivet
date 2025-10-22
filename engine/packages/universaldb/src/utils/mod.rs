use crate::tuple::{PackError, PackResult};

mod cherry_pick;
pub mod codes;
mod ext;
mod formal_key;
pub mod keys;
mod subspace;

pub use cherry_pick::*;
pub use ext::*;
pub use formal_key::*;
pub use subspace::Subspace;

pub const CHUNK_SIZE: usize = 10_000; // 10 KB, not KiB, see https://apple.github.io/foundationdb/blob.html

#[derive(Debug, Clone, Copy)]
pub enum IsolationLevel {
	Serializable,
	Snapshot,
}

/// Indicates the transaction might have committed
#[derive(Debug, Clone, Copy)]
pub struct MaybeCommitted(pub bool);

pub fn calculate_tx_retry_backoff(attempt: usize) -> u64 {
	// TODO: Update this to mirror fdb 1:1:
	// https://github.com/apple/foundationdb/blob/21407341d9b49e1d343514a7a5f395bd5f232079/fdbclient/NativeAPI.actor.cpp#L3162

	let base_backoff_ms = 2_u64.pow((attempt as u32).min(10)) * 10;

	let jitter_ms = rand::random::<u64>() % 100;

	base_backoff_ms + jitter_ms
}

/// When using `add_conflict_range` to add a conflict for a single key, you cannot set both the start and end
/// keys to the same key. Instead, the end key must be the start key + a 0 byte.
/// See Python bindings: https://github.com/apple/foundationdb/blob/ec714791df4a6e4dafb5a926130d5789ce0c497a/bindings/python/fdb/impl.py#L633-L635
pub fn end_of_key_range(key: &[u8]) -> Vec<u8> {
	let mut end_key = Vec::with_capacity(key.len() + 1);
	end_key.extend_from_slice(key);
	end_key.push(0);
	end_key
}

// Copied from foundationdb crate
#[inline]
pub fn parse_bytes(input: &[u8], num: usize) -> PackResult<(&[u8], &[u8])> {
	if input.len() < num {
		Err(PackError::MissingBytes)
	} else {
		Ok((&input[num..], &input[..num]))
	}
}

// Copied from foundationdb crate
#[inline]
pub fn parse_byte(input: &[u8]) -> PackResult<(&[u8], u8)> {
	if input.is_empty() {
		Err(PackError::MissingBytes)
	} else {
		Ok((&input[1..], input[0]))
	}
}

// Copied from foundationdb crate
pub fn parse_code(input: &[u8], expected: u8) -> PackResult<&[u8]> {
	let (input, found) = parse_byte(input)?;
	if found == expected {
		Ok(input)
	} else {
		Err(PackError::BadCode {
			found,
			expected: Some(expected),
		})
	}
}
