use sqlite_storage::pump::keys::{
	APTR_PARTITION, BOOKMARK_PARTITION, BR_PARTITION, BRANCHES_PARTITION, CMPC_PARTITION,
	CTR_PARTITION, CompactorQueueKind, NSPTR_PARTITION, NSBRANCH_PARTITION, PAGE_SIZE,
	SHARD_SIZE, SQLITE_SUBSPACE_PREFIX, actor_pointer_cur_key, actor_pointer_history_key,
	actor_prefix, actor_range, bookmark_key, bookmark_pinned_key, branch_commit_key,
	branch_delta_chunk_key, branch_manifest_cold_drained_txid_key,
	branch_manifest_last_access_bucket_key, branch_manifest_last_access_ts_ms_key,
		branch_manifest_last_hot_pass_txid_key, branch_meta_cold_compact_key,
		branch_meta_cold_lease_key, branch_meta_compact_key, branch_meta_compactor_lease_key,
		branch_meta_head_at_fork_key, branch_meta_head_key, branch_meta_quota_key, branch_pidx_key,
		branch_prefix, branch_range, branch_shard_key, branch_shard_version_prefix, branch_vtx_key,
		branches_bk_pin_key, branches_desc_pin_key, branches_list_key, branches_refcount_key,
		commit_key, compactor_enqueue_key, compactor_global_lease_key,
		ctr_eviction_index_key, ctr_eviction_index_range, decode_ctr_eviction_index_key,
		ctr_quota_global_key, delta_chunk_key, delta_chunk_prefix, delta_prefix, meta_compact_key,
		meta_compactor_lease_key, meta_head_key, meta_quota_key,
		namespace_branches_actor_tombstone_key, namespace_branches_bk_pin_key,
		namespace_branches_desc_pin_key, namespace_branches_list_key, namespace_branches_refcount_key,
		namespace_pointer_cur_key, namespace_pointer_history_key, pidx_delta_key, pidx_delta_prefix,
		shard_key, shard_prefix, shard_version_key, shard_version_prefix, vtx_key,
	};
use sqlite_storage::pump::types::{ActorBranchId, NamespaceBranchId, NamespaceId};
use uuid::Uuid;

const TEST_ACTOR: &str = "test-actor";
const TEST_BOOKMARK: &str = "0000018bcfe56800-0000000000000007";

fn actor_branch_id() -> ActorBranchId {
	ActorBranchId::from_uuid(Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff))
}

fn namespace_branch_id() -> NamespaceBranchId {
	NamespaceBranchId::from_uuid(Uuid::from_u128(0xffee_ddcc_bbaa_9988_7766_5544_3322_1100))
}

fn namespace_id() -> NamespaceId {
	NamespaceId::from_uuid(Uuid::from_u128(0x1020_3040_5060_7080_90a0_b0c0_d0e0_f000))
}

fn uuid_bytes(uuid: Uuid) -> Vec<u8> {
	uuid.as_bytes().to_vec()
}

#[test]
fn meta_subkeys_use_actor_prefix_and_expected_suffixes() {
	let actor_prefix = actor_prefix(TEST_ACTOR);

	let cases = [
		(meta_head_key(TEST_ACTOR), b"/META/head".as_slice()),
		(meta_compact_key(TEST_ACTOR), b"/META/compact".as_slice()),
		(meta_quota_key(TEST_ACTOR), b"/META/quota".as_slice()),
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
	assert_ne!(meta_quota_key(actor_a), meta_quota_key(actor_b));
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
	assert_ne!(commit_key(actor_a, 7), commit_key(actor_b, 7));
	assert_ne!(vtx_key(actor_a, [3; 16]), vtx_key(actor_b, [3; 16]));
}

#[test]
fn data_prefixes_match_full_keys() {
	assert!(delta_chunk_key(TEST_ACTOR, 7, 1).starts_with(&delta_prefix(TEST_ACTOR)));
	assert!(
		delta_chunk_key(TEST_ACTOR, 0x0102_0304_0506_0708, 0x090a_0b0c)
			.starts_with(&delta_chunk_prefix(TEST_ACTOR, 0x0102_0304_0506_0708))
	);
	assert!(shard_key(TEST_ACTOR, 3).starts_with(&shard_prefix(TEST_ACTOR)));
	assert!(shard_version_key(TEST_ACTOR, 3, 7).starts_with(&shard_version_prefix(TEST_ACTOR, 3)));
	assert!(shard_version_key(TEST_ACTOR, 3, 8) > shard_version_key(TEST_ACTOR, 3, 7));
	assert!(commit_key(TEST_ACTOR, 8) > commit_key(TEST_ACTOR, 7));
}

#[test]
fn pitr_partition_prefixes_use_reserved_bytes() {
	let actor_branch = actor_branch_id();
	let namespace_branch = namespace_branch_id();
	let namespace = namespace_id();

	assert_eq!(
		&actor_pointer_cur_key(namespace_branch, TEST_ACTOR)[..3],
		&[SQLITE_SUBSPACE_PREFIX, APTR_PARTITION, b'/']
	);
	assert_eq!(
		&namespace_pointer_cur_key(namespace)[..3],
		&[SQLITE_SUBSPACE_PREFIX, NSPTR_PARTITION, b'/']
	);
	assert_eq!(
		&branches_list_key(actor_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, BRANCHES_PARTITION]
	);
	assert_eq!(
		&namespace_branches_list_key(namespace_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, NSBRANCH_PARTITION]
	);
	assert_eq!(
		&branch_meta_head_key(actor_branch)[..3],
		&[SQLITE_SUBSPACE_PREFIX, BR_PARTITION, b'/']
	);
	assert_eq!(
		&ctr_quota_global_key()[..2],
		&[SQLITE_SUBSPACE_PREFIX, CTR_PARTITION]
	);
	assert_eq!(
		&bookmark_key(TEST_ACTOR, TEST_BOOKMARK)[..2],
		&[SQLITE_SUBSPACE_PREFIX, BOOKMARK_PARTITION]
	);
	assert_eq!(
		&compactor_global_lease_key(CompactorQueueKind::Cold)[..2],
		&[SQLITE_SUBSPACE_PREFIX, CMPC_PARTITION]
	);
}

#[test]
fn pointer_keys_include_current_and_history_paths() {
	let namespace_branch = namespace_branch_id();
	let namespace = namespace_id();
	let mut expected_actor = vec![SQLITE_SUBSPACE_PREFIX, APTR_PARTITION, b'/'];
	expected_actor.extend_from_slice(&uuid_bytes(namespace_branch.as_uuid()));
	expected_actor.extend_from_slice(b"/test-actor/cur");
	assert_eq!(actor_pointer_cur_key(namespace_branch, TEST_ACTOR), expected_actor);

	let mut expected_actor_history = vec![SQLITE_SUBSPACE_PREFIX, APTR_PARTITION, b'/'];
	expected_actor_history.extend_from_slice(&uuid_bytes(namespace_branch.as_uuid()));
	expected_actor_history.extend_from_slice(b"/test-actor/history/");
	expected_actor_history.extend_from_slice(&123_i64.to_be_bytes());
	expected_actor_history.extend_from_slice(&9_u32.to_be_bytes());
	assert_eq!(
		actor_pointer_history_key(namespace_branch, TEST_ACTOR, 123, 9),
		expected_actor_history
	);

	let mut expected_namespace = vec![SQLITE_SUBSPACE_PREFIX, NSPTR_PARTITION, b'/'];
	expected_namespace.extend_from_slice(&uuid_bytes(namespace.as_uuid()));
	expected_namespace.extend_from_slice(b"/cur");
	assert_eq!(namespace_pointer_cur_key(namespace), expected_namespace);

	assert!(namespace_pointer_history_key(namespace, 123, 9) > namespace_pointer_cur_key(namespace));
}

#[test]
fn branch_record_keys_include_counter_and_pin_subkeys() {
	let actor_branch = actor_branch_id();
	let namespace_branch = namespace_branch_id();

	let mut actor_base = vec![SQLITE_SUBSPACE_PREFIX, BRANCHES_PARTITION];
	actor_base.extend_from_slice(b"/list/");
	actor_base.extend_from_slice(&uuid_bytes(actor_branch.as_uuid()));
	assert_eq!(branches_list_key(actor_branch), actor_base);
	assert_eq!(branches_refcount_key(actor_branch), [actor_base.as_slice(), b"/refcount"].concat());
	assert_eq!(branches_desc_pin_key(actor_branch), [actor_base.as_slice(), b"/desc_pin"].concat());
	assert_eq!(branches_bk_pin_key(actor_branch), [actor_base.as_slice(), b"/bk_pin"].concat());

	let mut namespace_base = vec![SQLITE_SUBSPACE_PREFIX, NSBRANCH_PARTITION];
	namespace_base.extend_from_slice(b"/list/");
	namespace_base.extend_from_slice(&uuid_bytes(namespace_branch.as_uuid()));
	assert_eq!(namespace_branches_list_key(namespace_branch), namespace_base);
	assert_eq!(
		namespace_branches_refcount_key(namespace_branch),
		[namespace_base.as_slice(), b"/refcount"].concat()
	);
	assert_eq!(
		namespace_branches_desc_pin_key(namespace_branch),
		[namespace_base.as_slice(), b"/desc_pin"].concat()
	);
	assert_eq!(
		namespace_branches_bk_pin_key(namespace_branch),
		[namespace_base.as_slice(), b"/bk_pin"].concat()
	);
	assert_eq!(
		namespace_branches_actor_tombstone_key(namespace_branch, TEST_ACTOR),
		[namespace_base.as_slice(), b"/actor_tombstones/test-actor"].concat()
	);
}

#[test]
fn actor_branch_data_keys_live_under_br_partition() {
	let branch = actor_branch_id();
	let prefix = branch_prefix(branch);
	let (start, end) = branch_range(branch);

	assert_eq!(start, prefix);
	assert_eq!(end, {
		let mut key = branch_prefix(branch);
		key.push(0);
		key
	});

	for key in [
		branch_meta_head_key(branch),
		branch_meta_head_at_fork_key(branch),
		branch_meta_compact_key(branch),
		branch_meta_cold_compact_key(branch),
		branch_meta_quota_key(branch),
		branch_meta_compactor_lease_key(branch),
		branch_meta_cold_lease_key(branch),
		branch_manifest_cold_drained_txid_key(branch),
		branch_manifest_last_hot_pass_txid_key(branch),
		branch_manifest_last_access_ts_ms_key(branch),
		branch_manifest_last_access_bucket_key(branch),
		branch_commit_key(branch, 7),
		branch_vtx_key(branch, [3; 16]),
		branch_pidx_key(branch, 9),
		branch_delta_chunk_key(branch, 7, 2),
		branch_shard_key(branch, 4, 7),
	] {
		assert!(key.starts_with(&branch_prefix(branch)));
	}

	assert_eq!(
		&branch_meta_head_at_fork_key(branch)[branch_prefix(branch).len()..],
		b"/META/head_at_fork"
	);
	assert!(branch_shard_key(branch, 4, 7).starts_with(&branch_shard_version_prefix(branch, 4)));
	assert!(branch_commit_key(branch, 8) > branch_commit_key(branch, 7));
}

#[test]
fn global_bookmark_and_compactor_keys_match_expected_suffixes() {
	let branch = actor_branch_id();

	let eviction = ctr_eviction_index_key(42, branch);
	assert!(eviction.starts_with(&[SQLITE_SUBSPACE_PREFIX, CTR_PARTITION]));
	assert!(eviction.ends_with(actor_branch_id().as_uuid().as_bytes()));
	assert_eq!(decode_ctr_eviction_index_key(&eviction).unwrap(), (42, branch));
	let (eviction_start, eviction_end) = ctr_eviction_index_range();
	assert!(eviction >= eviction_start);
	assert!(eviction < eviction_end);

	assert_eq!(
		bookmark_pinned_key(TEST_ACTOR, TEST_BOOKMARK),
		[
			bookmark_key(TEST_ACTOR, TEST_BOOKMARK).as_slice(),
			b"/pinned".as_slice()
		]
		.concat()
	);

	let cold_enqueue = compactor_enqueue_key(55, TEST_ACTOR, CompactorQueueKind::Cold);
	let eviction_enqueue = compactor_enqueue_key(55, TEST_ACTOR, CompactorQueueKind::Eviction);
	assert_ne!(cold_enqueue, eviction_enqueue);
	assert_eq!(*cold_enqueue.last().expect("kind byte"), 0x00);
	assert_eq!(*eviction_enqueue.last().expect("kind byte"), 0x01);
	assert_eq!(
		*compactor_global_lease_key(CompactorQueueKind::Eviction)
			.last()
			.expect("kind byte"),
		0x01
	);
}
