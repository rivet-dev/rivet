use sqlite_storage::pump::keys::{
	PAGE_SIZE, SHARD_SIZE, SQLITE_SUBSPACE_PREFIX, actor_prefix, actor_range, delta_chunk_key,
	delta_chunk_prefix, delta_prefix, meta_compact_key, meta_compactor_lease_key, meta_head_key,
	meta_storage_used_live_key, meta_storage_used_pitr_key, pidx_delta_key, pidx_delta_prefix,
	shard_key, shard_prefix,
};

const TEST_ACTOR: &str = "test-actor";

#[test]
fn meta_subkeys_use_actor_prefix_and_expected_suffixes() {
	let actor_prefix = actor_prefix(TEST_ACTOR);

	let cases = [
		(meta_head_key(TEST_ACTOR), b"/META/head".as_slice()),
		(meta_compact_key(TEST_ACTOR), b"/META/compact".as_slice()),
		(
			meta_storage_used_live_key(TEST_ACTOR),
			b"/META/storage_used_live".as_slice(),
		),
		(
			meta_storage_used_pitr_key(TEST_ACTOR),
			b"/META/storage_used_pitr".as_slice(),
		),
		(
			meta_compactor_lease_key(TEST_ACTOR),
			b"/META/compactor_lease".as_slice(),
		),
	];

	for (key, suffix) in cases {
		assert!(key.starts_with(&actor_prefix));
		assert_eq!(&key[actor_prefix.len()..], suffix);
	}

	assert_eq!(PAGE_SIZE, 4096);
	assert_eq!(SHARD_SIZE, 64);
}

#[test]
fn pidx_keys_sort_by_big_endian_page_number() {
	let mut keys = vec![
		pidx_delta_key(TEST_ACTOR, 9000),
		pidx_delta_key(TEST_ACTOR, 2),
		pidx_delta_key(TEST_ACTOR, 17),
		pidx_delta_key(TEST_ACTOR, 256),
	];

	keys.sort();

	assert_eq!(
		keys,
		vec![
			pidx_delta_key(TEST_ACTOR, 2),
			pidx_delta_key(TEST_ACTOR, 17),
			pidx_delta_key(TEST_ACTOR, 256),
			pidx_delta_key(TEST_ACTOR, 9000),
		]
	);
	assert!(pidx_delta_key(TEST_ACTOR, 7).starts_with(&pidx_delta_prefix(TEST_ACTOR)));
	assert_eq!(pidx_delta_key(TEST_ACTOR, 7)[0], SQLITE_SUBSPACE_PREFIX);
}

#[test]
fn actor_scoped_keys_do_not_collide() {
	let actor_a = "actor-a";
	let actor_b = "actor-b";

	assert_ne!(meta_head_key(actor_a), meta_head_key(actor_b));
	assert_ne!(meta_compact_key(actor_a), meta_compact_key(actor_b));
	assert_ne!(
		meta_storage_used_live_key(actor_a),
		meta_storage_used_live_key(actor_b)
	);
	assert_ne!(
		meta_storage_used_pitr_key(actor_a),
		meta_storage_used_pitr_key(actor_b)
	);
	assert_ne!(
		meta_compactor_lease_key(actor_a),
		meta_compactor_lease_key(actor_b)
	);
	assert_ne!(pidx_delta_key(actor_a, 7), pidx_delta_key(actor_b, 7));
	assert_ne!(
		delta_chunk_key(actor_a, 1, 0),
		delta_chunk_key(actor_b, 1, 0)
	);
	assert_ne!(shard_key(actor_a, 3), shard_key(actor_b, 3));

	let (start, end) = actor_range(actor_a);
	assert_eq!(start, actor_prefix(actor_a));
	assert_eq!(end, {
		let mut key = actor_prefix(actor_a);
		key.push(0);
		key
	});
	assert!(shard_key(actor_a, 3).starts_with(&actor_prefix(actor_a)));
	assert!(shard_key(actor_b, 3).starts_with(&actor_prefix(actor_b)));
}

#[test]
fn data_prefixes_match_full_keys() {
	assert!(delta_chunk_key(TEST_ACTOR, 7, 1).starts_with(&delta_prefix(TEST_ACTOR)));
	assert!(
		delta_chunk_key(TEST_ACTOR, 0x0102_0304_0506_0708, 0x090a_0b0c)
			.starts_with(&delta_chunk_prefix(TEST_ACTOR, 0x0102_0304_0506_0708))
	);
	assert!(shard_key(TEST_ACTOR, 3).starts_with(&shard_prefix(TEST_ACTOR)));
}
