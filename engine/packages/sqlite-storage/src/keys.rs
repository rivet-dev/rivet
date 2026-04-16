//! Key builders for sqlite-storage blobs and indexes.

use anyhow::{Context, Result, ensure};

pub const SQLITE_SUBSPACE_PREFIX: u8 = 0x02;

const META_PATH: &[u8] = b"/META";
const SHARD_PATH: &[u8] = b"/SHARD/";
const DELTA_PATH: &[u8] = b"/DELTA/";
const PIDX_DELTA_PATH: &[u8] = b"/PIDX/delta/";
const STAGE_PATH: &[u8] = b"/STAGE/";

/// Build the common actor-scoped prefix: `[0x02, actor_id_bytes]`.
pub(crate) fn actor_prefix(actor_id: &str) -> Vec<u8> {
	let actor_bytes = actor_id.as_bytes();
	let mut key = Vec::with_capacity(1 + actor_bytes.len());
	key.push(SQLITE_SUBSPACE_PREFIX);
	key.extend_from_slice(actor_bytes);
	key
}

pub fn meta_key(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + META_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(META_PATH);
	key
}

pub fn shard_key(actor_id: &str, shard_id: u32) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + SHARD_PATH.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(SHARD_PATH);
	key.extend_from_slice(&shard_id.to_be_bytes());
	key
}

pub fn shard_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + SHARD_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(SHARD_PATH);
	key
}

pub fn delta_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + DELTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key
}

pub fn delta_chunk_prefix(actor_id: &str, txid: u64) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key =
		Vec::with_capacity(prefix.len() + DELTA_PATH.len() + std::mem::size_of::<u64>() + 1);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(DELTA_PATH);
	key.extend_from_slice(&txid.to_be_bytes());
	key.push(b'/');
	key
}

pub fn delta_chunk_key(actor_id: &str, txid: u64, chunk_idx: u32) -> Vec<u8> {
	let prefix = delta_chunk_prefix(actor_id, txid);
	let mut key = Vec::with_capacity(prefix.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn pidx_delta_key(actor_id: &str, pgno: u32) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key =
		Vec::with_capacity(prefix.len() + PIDX_DELTA_PATH.len() + std::mem::size_of::<u32>());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(PIDX_DELTA_PATH);
	key.extend_from_slice(&pgno.to_be_bytes());
	key
}

pub fn pidx_delta_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + PIDX_DELTA_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(PIDX_DELTA_PATH);
	key
}

pub fn stage_key(actor_id: &str, stage_id: u64, chunk_idx: u16) -> Vec<u8> {
	let chunk_prefix = stage_chunk_prefix(actor_id, stage_id);
	let mut key = Vec::with_capacity(chunk_prefix.len() + std::mem::size_of::<u16>());
	key.extend_from_slice(&chunk_prefix);
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

pub fn stage_prefix(actor_id: &str) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key = Vec::with_capacity(prefix.len() + STAGE_PATH.len());
	key.extend_from_slice(&prefix);
	key.extend_from_slice(STAGE_PATH);
	key
}

pub fn stage_chunk_prefix(actor_id: &str, stage_id: u64) -> Vec<u8> {
	let prefix = actor_prefix(actor_id);
	let mut key =
		Vec::with_capacity(prefix.len() + STAGE_PATH.len() + std::mem::size_of::<u64>() + 1);
	key.extend_from_slice(&prefix);
	key.extend_from_slice(STAGE_PATH);
	key.extend_from_slice(&stage_id.to_be_bytes());
	key.push(b'/');
	key
}

pub fn decode_delta_chunk_txid(actor_id: &str, key: &[u8]) -> Result<u64> {
	let prefix = delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"delta key did not start with expected prefix"
	);
	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() >= std::mem::size_of::<u64>() + 1,
		"delta key suffix had {} bytes, expected at least {}",
		suffix.len(),
		std::mem::size_of::<u64>() + 1
	);
	ensure!(
		suffix[std::mem::size_of::<u64>()] == b'/',
		"delta key missing txid/chunk separator"
	);

	Ok(u64::from_be_bytes(
		suffix[..std::mem::size_of::<u64>()]
			.try_into()
			.context("delta txid suffix should decode as u64")?,
	))
}

pub fn decode_delta_chunk_idx(actor_id: &str, txid: u64, key: &[u8]) -> Result<u32> {
	let prefix = delta_chunk_prefix(actor_id, txid);
	ensure!(
		key.starts_with(&prefix),
		"delta chunk key did not start with expected prefix"
	);
	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == std::mem::size_of::<u32>(),
		"delta chunk key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<u32>()
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("delta chunk suffix should decode as u32")?,
	))
}

#[cfg(test)]
mod tests {
	use super::{
		DELTA_PATH, META_PATH, SHARD_PATH, SQLITE_SUBSPACE_PREFIX, STAGE_PATH, actor_prefix,
		decode_delta_chunk_idx, decode_delta_chunk_txid, delta_chunk_key, delta_chunk_prefix,
		delta_prefix, meta_key, pidx_delta_key, pidx_delta_prefix, shard_key, shard_prefix,
		stage_chunk_prefix, stage_key, stage_prefix,
	};

	const TEST_ACTOR: &str = "test-actor";

	#[test]
	fn meta_key_includes_actor_id() {
		let key = meta_key(TEST_ACTOR);
		let expected_prefix = actor_prefix(TEST_ACTOR);
		assert!(key.starts_with(&expected_prefix));
		assert_eq!(&key[expected_prefix.len()..], META_PATH);
	}

	#[test]
	fn shard_and_delta_keys_use_big_endian_numeric_suffixes() {
		let shard = shard_key(TEST_ACTOR, 0x0102_0304);
		let delta = delta_chunk_key(TEST_ACTOR, 0x0102_0304_0506_0708, 0x090a_0b0c);
		let ap = actor_prefix(TEST_ACTOR);

		assert!(shard.starts_with(&ap));
		let after_actor = &shard[ap.len()..];
		assert!(after_actor.starts_with(SHARD_PATH));
		assert_eq!(&after_actor[SHARD_PATH.len()..], &[1, 2, 3, 4]);

		assert!(delta.starts_with(&ap));
		let after_actor = &delta[ap.len()..];
		assert!(after_actor.starts_with(DELTA_PATH));
		assert_eq!(
			&after_actor[DELTA_PATH.len()..],
			&[1, 2, 3, 4, 5, 6, 7, 8, b'/', 9, 10, 11, 12]
		);
	}

	#[test]
	fn pidx_keys_sort_by_page_number() {
		let pgno_2 = pidx_delta_key(TEST_ACTOR, 2);
		let pgno_17 = pidx_delta_key(TEST_ACTOR, 17);
		let pgno_9000 = pidx_delta_key(TEST_ACTOR, 9000);

		assert_eq!(pgno_2[0], SQLITE_SUBSPACE_PREFIX);
		assert!(pgno_2 < pgno_17);
		assert!(pgno_17 < pgno_9000);
	}

	#[test]
	fn delta_and_stage_prefixes_match_full_keys() {
		assert!(delta_chunk_key(TEST_ACTOR, 7, 1).starts_with(&delta_prefix(TEST_ACTOR)));
		assert!(shard_key(TEST_ACTOR, 3).starts_with(&shard_prefix(TEST_ACTOR)));
		assert!(stage_key(TEST_ACTOR, 9, 1).starts_with(&stage_prefix(TEST_ACTOR)));
	}

	#[test]
	fn delta_chunk_prefix_matches_full_key() {
		let prefix = delta_chunk_prefix(TEST_ACTOR, 0x0102_0304_0506_0708);
		let key = delta_chunk_key(TEST_ACTOR, 0x0102_0304_0506_0708, 0x090a_0b0c);

		assert!(key.starts_with(&prefix));
		assert_eq!(key.len() - prefix.len(), std::mem::size_of::<u32>());
	}

	#[test]
	fn stage_chunk_prefix_matches_full_stage_key() {
		let prefix = stage_chunk_prefix(TEST_ACTOR, 0x0102_0304_0506_0708);
		let key = stage_key(TEST_ACTOR, 0x0102_0304_0506_0708, 0x090a);

		assert!(key.starts_with(&prefix));
		assert_eq!(key.len() - prefix.len(), std::mem::size_of::<u16>());
	}

	#[test]
	fn pidx_prefix_matches_key_prefix() {
		let prefix = pidx_delta_prefix(TEST_ACTOR);
		let key = pidx_delta_key(TEST_ACTOR, 12);

		assert_eq!(prefix[0], SQLITE_SUBSPACE_PREFIX);
		assert!(key.starts_with(&prefix));
		assert_eq!(key.len() - prefix.len(), std::mem::size_of::<u32>());
	}

	#[test]
	fn stage_keys_include_actor_stage_and_chunk_components() {
		let key = stage_key(TEST_ACTOR, 0x0102_0304_0506_0708, 0x090a);
		let ap = actor_prefix(TEST_ACTOR);

		assert!(key.starts_with(&ap));
		let after_actor = &key[ap.len()..];
		assert!(after_actor.starts_with(STAGE_PATH));
		let after_stage_path = &after_actor[STAGE_PATH.len()..];
		assert_eq!(&after_stage_path[..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
		assert_eq!(after_stage_path[8], b'/');
		assert_eq!(&after_stage_path[9..], &[9, 10]);
	}

	#[test]
	fn big_endian_ordering_matches_numeric_order() {
		let mut shard_keys = vec![
			shard_key(TEST_ACTOR, 99),
			shard_key(TEST_ACTOR, 7),
			shard_key(TEST_ACTOR, 42),
		];
		let mut delta_keys = vec![
			delta_chunk_key(TEST_ACTOR, 99, 0),
			delta_chunk_key(TEST_ACTOR, 7, 0),
			delta_chunk_key(TEST_ACTOR, 42, 0),
		];

		shard_keys.sort();
		delta_keys.sort();

		assert_eq!(
			shard_keys,
			vec![
				shard_key(TEST_ACTOR, 7),
				shard_key(TEST_ACTOR, 42),
				shard_key(TEST_ACTOR, 99)
			]
		);
		assert_eq!(
			delta_keys,
			vec![
				delta_chunk_key(TEST_ACTOR, 7, 0),
				delta_chunk_key(TEST_ACTOR, 42, 0),
				delta_chunk_key(TEST_ACTOR, 99, 0)
			]
		);
	}

	#[test]
	fn different_actors_produce_different_keys() {
		assert_ne!(meta_key("actor-a"), meta_key("actor-b"));
		assert_ne!(
			delta_chunk_key("actor-a", 1, 0),
			delta_chunk_key("actor-b", 1, 0)
		);
		assert_ne!(shard_key("actor-a", 0), shard_key("actor-b", 0));
	}

	#[test]
	fn delta_chunk_decoders_round_trip() {
		let key = delta_chunk_key(TEST_ACTOR, 77, 9);

		assert_eq!(decode_delta_chunk_txid(TEST_ACTOR, &key).unwrap(), 77);
		assert_eq!(decode_delta_chunk_idx(TEST_ACTOR, 77, &key).unwrap(), 9);
	}
}
