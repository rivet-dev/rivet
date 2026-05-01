use depot::conveyer::keys::{
	DBPTR_PARTITION, BOOKMARK_PARTITION, BR_PARTITION, BRANCHES_PARTITION, CMPC_PARTITION,
	CTR_PARTITION, DB_PIN_PARTITION, CompactorQueueKind, NS_CHILD_PARTITION,
	NS_FORK_PIN_PARTITION, NS_PROOF_EPOCH_PARTITION, NSPTR_PARTITION, NSBRANCH_PARTITION,
	NSCAT_BY_DB_PARTITION, PAGE_SIZE, SHARD_SIZE, SQLITE_CMP_DIRTY_PARTITION,
	SQLITE_SUBSPACE_PREFIX, bookmark_key, bookmark_pinned_key,
	branch_commit_key, branch_compaction_cold_shard_key,
	branch_compaction_cold_shard_version_prefix, branch_compaction_retired_cold_object_key,
	branch_compaction_root_key, branch_compaction_stage_hot_shard_key,
	branch_compaction_stage_hot_shard_version_prefix, branch_delta_chunk_key,
	branch_manifest_cold_drained_txid_key,
	branch_manifest_last_access_bucket_key, branch_manifest_last_access_ts_ms_key,
	branch_manifest_last_hot_pass_txid_key, branch_meta_cold_compact_key,
	branch_meta_cold_lease_key, branch_meta_compact_key, branch_meta_compactor_lease_key,
	branch_meta_head_at_fork_key, branch_meta_head_key, branch_meta_quota_key, branch_pidx_key,
	branch_prefix, branch_range, branch_shard_key, branch_shard_version_prefix, branch_vtx_key,
	branches_bk_pin_key, branches_desc_pin_key, branches_list_key, branches_refcount_key,
	commit_key, compactor_enqueue_key, compactor_global_lease_key, ctr_eviction_index_key,
	ctr_eviction_index_range, ctr_quota_global_key, database_pointer_cur_key,
	database_pointer_history_key, database_prefix, database_range, db_pin_key, db_pin_prefix,
	decode_ctr_eviction_index_key, delta_chunk_key, delta_chunk_prefix, delta_prefix,
	meta_compact_key, meta_compactor_lease_key, meta_head_key, meta_quota_key, ns_child_key,
	ns_child_prefix, ns_fork_pin_key, ns_fork_pin_prefix, ns_proof_epoch_key, nscat_by_db_key,
	nscat_by_db_prefix, namespace_branches_bk_pin_key,
	namespace_branches_database_name_tombstone_key, namespace_branches_desc_pin_key,
	namespace_branches_list_key, namespace_branches_refcount_key, namespace_pointer_cur_key,
	namespace_pointer_history_key, pidx_delta_key, pidx_delta_prefix, shard_key, shard_prefix,
	shard_version_key, shard_version_prefix, sqlite_cmp_dirty_key, vtx_key,
};
use depot::conveyer::types::{DatabaseBranchId, NamespaceBranchId, NamespaceId};
use gas::prelude::Id;
use uuid::Uuid;

const TEST_DATABASE: &str = "test-database";
const TEST_BOOKMARK: &str = "0000018bcfe56800-0000000000000007";

fn database_branch_id() -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff))
}

fn namespace_branch_id() -> NamespaceBranchId {
	NamespaceBranchId::from_uuid(Uuid::from_u128(0xffee_ddcc_bbaa_9988_7766_5544_3322_1100))
}

fn namespace_id() -> NamespaceId {
	NamespaceId::from_uuid(Uuid::from_u128(0x1020_3040_5060_7080_90a0_b0c0_d0e0_f000))
}

fn compaction_job_id() -> Id {
	Id::v1(Uuid::from_u128(0x1234_5678_9abc_def0_1122_3344_5566_7788), 42)
}

fn uuid_bytes(uuid: Uuid) -> Vec<u8> {
	uuid.as_bytes().to_vec()
}

#[test]
fn meta_subkeys_use_database_prefix_and_expected_suffixes() {
	let database_prefix = database_prefix(TEST_DATABASE);

	let cases = [
		(meta_head_key(TEST_DATABASE), b"/META/head".as_slice()),
		(meta_compact_key(TEST_DATABASE), b"/META/compact".as_slice()),
		(meta_quota_key(TEST_DATABASE), b"/META/quota".as_slice()),
		(
			meta_compactor_lease_key(TEST_DATABASE),
			b"/META/compactor_lease".as_slice(),
		),
	];

	for (key, suffix) in cases {
		assert!(key.starts_with(&database_prefix));
		assert_eq!(&key[database_prefix.len()..], suffix);
	}

	assert_eq!(PAGE_SIZE, 4096);
	assert_eq!(SHARD_SIZE, 64);
}

#[test]
fn pidx_keys_sort_by_big_endian_page_number() {
	let mut keys = vec![
		pidx_delta_key(TEST_DATABASE, 9000),
		pidx_delta_key(TEST_DATABASE, 2),
		pidx_delta_key(TEST_DATABASE, 17),
		pidx_delta_key(TEST_DATABASE, 256),
	];

	keys.sort();

	assert_eq!(
		keys,
		vec![
			pidx_delta_key(TEST_DATABASE, 2),
			pidx_delta_key(TEST_DATABASE, 17),
			pidx_delta_key(TEST_DATABASE, 256),
			pidx_delta_key(TEST_DATABASE, 9000),
		]
	);
	assert!(pidx_delta_key(TEST_DATABASE, 7).starts_with(&pidx_delta_prefix(TEST_DATABASE)));
	assert_eq!(pidx_delta_key(TEST_DATABASE, 7)[0], SQLITE_SUBSPACE_PREFIX);
}

#[test]
fn database_scoped_keys_do_not_collide() {
	let database_a = "database-a";
	let database_b = "database-b";

	assert_ne!(meta_head_key(database_a), meta_head_key(database_b));
	assert_ne!(meta_compact_key(database_a), meta_compact_key(database_b));
	assert_ne!(meta_quota_key(database_a), meta_quota_key(database_b));
	assert_ne!(
		meta_compactor_lease_key(database_a),
		meta_compactor_lease_key(database_b)
	);
	assert_ne!(pidx_delta_key(database_a, 7), pidx_delta_key(database_b, 7));
	assert_ne!(
		delta_chunk_key(database_a, 1, 0),
		delta_chunk_key(database_b, 1, 0)
	);
	assert_ne!(shard_key(database_a, 3), shard_key(database_b, 3));

	let (start, end) = database_range(database_a);
	assert_eq!(start, database_prefix(database_a));
	assert_eq!(end, {
		let mut key = database_prefix(database_a);
		key.push(0);
		key
	});
	assert!(shard_key(database_a, 3).starts_with(&database_prefix(database_a)));
	assert!(shard_key(database_b, 3).starts_with(&database_prefix(database_b)));
	assert_ne!(commit_key(database_a, 7), commit_key(database_b, 7));
	assert_ne!(vtx_key(database_a, [3; 16]), vtx_key(database_b, [3; 16]));
}

#[test]
fn data_prefixes_match_full_keys() {
	assert!(delta_chunk_key(TEST_DATABASE, 7, 1).starts_with(&delta_prefix(TEST_DATABASE)));
	assert!(
		delta_chunk_key(TEST_DATABASE, 0x0102_0304_0506_0708, 0x090a_0b0c)
			.starts_with(&delta_chunk_prefix(TEST_DATABASE, 0x0102_0304_0506_0708))
	);
	assert!(shard_key(TEST_DATABASE, 3).starts_with(&shard_prefix(TEST_DATABASE)));
	assert!(shard_version_key(TEST_DATABASE, 3, 7).starts_with(&shard_version_prefix(TEST_DATABASE, 3)));
	assert!(shard_version_key(TEST_DATABASE, 3, 8) > shard_version_key(TEST_DATABASE, 3, 7));
	assert!(commit_key(TEST_DATABASE, 8) > commit_key(TEST_DATABASE, 7));
}

#[test]
fn pitr_partition_prefixes_use_reserved_bytes() {
	let database_branch = database_branch_id();
	let namespace_branch = namespace_branch_id();
	let namespace = namespace_id();

	assert_eq!(
		&database_pointer_cur_key(namespace_branch, TEST_DATABASE)[..3],
		&[SQLITE_SUBSPACE_PREFIX, DBPTR_PARTITION, b'/']
	);
	assert_eq!(
		&namespace_pointer_cur_key(namespace)[..3],
		&[SQLITE_SUBSPACE_PREFIX, NSPTR_PARTITION, b'/']
	);
	assert_eq!(
		&branches_list_key(database_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, BRANCHES_PARTITION]
	);
	assert_eq!(
		&namespace_branches_list_key(namespace_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, NSBRANCH_PARTITION]
	);
	assert_eq!(
		&branch_meta_head_key(database_branch)[..3],
		&[SQLITE_SUBSPACE_PREFIX, BR_PARTITION, b'/']
	);
	assert_eq!(
		&ctr_quota_global_key()[..2],
		&[SQLITE_SUBSPACE_PREFIX, CTR_PARTITION]
	);
	assert_eq!(
		&bookmark_key(TEST_DATABASE, TEST_BOOKMARK)[..2],
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
	let mut expected_database = vec![SQLITE_SUBSPACE_PREFIX, DBPTR_PARTITION, b'/'];
	expected_database.extend_from_slice(&uuid_bytes(namespace_branch.as_uuid()));
	expected_database.extend_from_slice(b"/test-database/cur");
	assert_eq!(database_pointer_cur_key(namespace_branch, TEST_DATABASE), expected_database);

	let mut expected_database_history = vec![SQLITE_SUBSPACE_PREFIX, DBPTR_PARTITION, b'/'];
	expected_database_history.extend_from_slice(&uuid_bytes(namespace_branch.as_uuid()));
	expected_database_history.extend_from_slice(b"/test-database/history/");
	expected_database_history.extend_from_slice(&123_i64.to_be_bytes());
	expected_database_history.extend_from_slice(&9_u32.to_be_bytes());
	assert_eq!(
		database_pointer_history_key(namespace_branch, TEST_DATABASE, 123, 9),
		expected_database_history
	);

	let mut expected_namespace = vec![SQLITE_SUBSPACE_PREFIX, NSPTR_PARTITION, b'/'];
	expected_namespace.extend_from_slice(&uuid_bytes(namespace.as_uuid()));
	expected_namespace.extend_from_slice(b"/cur");
	assert_eq!(namespace_pointer_cur_key(namespace), expected_namespace);

	assert!(namespace_pointer_history_key(namespace, 123, 9) > namespace_pointer_cur_key(namespace));
}

#[test]
fn branch_record_keys_include_counter_and_pin_subkeys() {
	let database_branch = database_branch_id();
	let namespace_branch = namespace_branch_id();

	let mut database_base = vec![SQLITE_SUBSPACE_PREFIX, BRANCHES_PARTITION];
	database_base.extend_from_slice(b"/list/");
	database_base.extend_from_slice(&uuid_bytes(database_branch.as_uuid()));
	assert_eq!(branches_list_key(database_branch), database_base);
	assert_eq!(branches_refcount_key(database_branch), [database_base.as_slice(), b"/refcount"].concat());
	assert_eq!(branches_desc_pin_key(database_branch), [database_base.as_slice(), b"/desc_pin"].concat());
	assert_eq!(branches_bk_pin_key(database_branch), [database_base.as_slice(), b"/bk_pin"].concat());

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
		namespace_branches_database_name_tombstone_key(namespace_branch, TEST_DATABASE),
		[namespace_base.as_slice(), b"/database_tombstones/test-database"].concat()
	);
}

#[test]
fn database_branch_data_keys_live_under_br_partition() {
	let branch = database_branch_id();
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
		branch_compaction_root_key(branch),
		branch_compaction_cold_shard_key(branch, 4, 7),
		branch_compaction_retired_cold_object_key(branch, [1; 32]),
		branch_compaction_stage_hot_shard_key(branch, compaction_job_id(), 4, 7, 2),
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
	assert_eq!(
		&branch_compaction_root_key(branch)[branch_prefix(branch).len()..],
		b"/CMP/root"
	);
	assert!(branch_shard_key(branch, 4, 7).starts_with(&branch_shard_version_prefix(branch, 4)));
	assert!(
		branch_compaction_cold_shard_key(branch, 4, 7)
			.starts_with(&branch_compaction_cold_shard_version_prefix(branch, 4))
	);
	assert!(
		branch_compaction_stage_hot_shard_key(branch, compaction_job_id(), 4, 7, 2)
			.starts_with(&branch_compaction_stage_hot_shard_version_prefix(
				branch,
				compaction_job_id(),
				4
			))
	);
	assert!(branch_commit_key(branch, 8) > branch_commit_key(branch, 7));
}

#[test]
fn global_bookmark_and_compactor_keys_match_expected_suffixes() {
	let branch = database_branch_id();

	let eviction = ctr_eviction_index_key(42, branch);
	assert!(eviction.starts_with(&[SQLITE_SUBSPACE_PREFIX, CTR_PARTITION]));
	assert!(eviction.ends_with(database_branch_id().as_uuid().as_bytes()));
	assert_eq!(decode_ctr_eviction_index_key(&eviction).unwrap(), (42, branch));
	let (eviction_start, eviction_end) = ctr_eviction_index_range();
	assert!(eviction >= eviction_start);
	assert!(eviction < eviction_end);

	assert_eq!(
		bookmark_pinned_key(TEST_DATABASE, TEST_BOOKMARK),
		[
			bookmark_key(TEST_DATABASE, TEST_BOOKMARK).as_slice(),
			b"/pinned".as_slice()
		]
		.concat()
	);

	let cold_enqueue = compactor_enqueue_key(55, TEST_DATABASE, CompactorQueueKind::Cold);
	let eviction_enqueue = compactor_enqueue_key(55, TEST_DATABASE, CompactorQueueKind::Eviction);
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

#[test]
fn workflow_compaction_key_partitions_are_reserved() {
	let branch = database_branch_id();
	let namespace_branch = namespace_branch_id();

	assert_eq!(
		&db_pin_key(branch, b"bookmark/test")[..2],
		&[SQLITE_SUBSPACE_PREFIX, DB_PIN_PARTITION]
	);
	assert_eq!(
		&ns_fork_pin_key(namespace_branch, [1; 16], namespace_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, NS_FORK_PIN_PARTITION]
	);
	assert_eq!(
		&ns_child_key(namespace_branch, [1; 16], namespace_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, NS_CHILD_PARTITION]
	);
	assert_eq!(
		&nscat_by_db_key(branch, namespace_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, NSCAT_BY_DB_PARTITION]
	);
	assert_eq!(
		&ns_proof_epoch_key(namespace_branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, NS_PROOF_EPOCH_PARTITION]
	);
	assert_eq!(
		&sqlite_cmp_dirty_key(branch)[..2],
		&[SQLITE_SUBSPACE_PREFIX, SQLITE_CMP_DIRTY_PARTITION]
	);
}

#[test]
fn workflow_compaction_branch_keys_sort_by_big_endian_components() {
	let branch = database_branch_id();
	let job = compaction_job_id();

	let mut cold_shards = vec![
		branch_compaction_cold_shard_key(branch, 2, 10),
		branch_compaction_cold_shard_key(branch, 1, 50),
		branch_compaction_cold_shard_key(branch, 1, 7),
		branch_compaction_cold_shard_key(branch, 2, 1),
	];
	cold_shards.sort();
	assert_eq!(
		cold_shards,
		vec![
			branch_compaction_cold_shard_key(branch, 1, 7),
			branch_compaction_cold_shard_key(branch, 1, 50),
			branch_compaction_cold_shard_key(branch, 2, 1),
			branch_compaction_cold_shard_key(branch, 2, 10),
		]
	);

	let mut staged_hot_shards = vec![
		branch_compaction_stage_hot_shard_key(branch, job, 2, 10, 1),
		branch_compaction_stage_hot_shard_key(branch, job, 1, 50, 1),
		branch_compaction_stage_hot_shard_key(branch, job, 1, 7, 2),
		branch_compaction_stage_hot_shard_key(branch, job, 1, 7, 1),
	];
	staged_hot_shards.sort();
	assert_eq!(
		staged_hot_shards,
		vec![
			branch_compaction_stage_hot_shard_key(branch, job, 1, 7, 1),
			branch_compaction_stage_hot_shard_key(branch, job, 1, 7, 2),
			branch_compaction_stage_hot_shard_key(branch, job, 1, 50, 1),
			branch_compaction_stage_hot_shard_key(branch, job, 2, 10, 1),
		]
	);
}

#[test]
fn workflow_compaction_global_keys_sort_by_big_endian_components() {
	let branch = database_branch_id();
	let source_namespace = namespace_branch_id();
	let target_a = NamespaceBranchId::from_uuid(Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0001));
	let target_b = NamespaceBranchId::from_uuid(Uuid::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0002));

	assert!(db_pin_key(branch, b"bookmark/a").starts_with(&db_pin_prefix(branch)));
	assert!(nscat_by_db_key(branch, source_namespace).starts_with(&nscat_by_db_prefix(branch)));
	assert!(ns_fork_pin_key(source_namespace, [1; 16], target_a).starts_with(&ns_fork_pin_prefix(source_namespace)));
	assert!(ns_child_key(source_namespace, [1; 16], target_a).starts_with(&ns_child_prefix(source_namespace)));

	let mut fork_pins = vec![
		ns_fork_pin_key(source_namespace, [2; 16], target_a),
		ns_fork_pin_key(source_namespace, [1; 16], target_b),
		ns_fork_pin_key(source_namespace, [1; 16], target_a),
	];
	fork_pins.sort();
	assert_eq!(
		fork_pins,
		vec![
			ns_fork_pin_key(source_namespace, [1; 16], target_a),
			ns_fork_pin_key(source_namespace, [1; 16], target_b),
			ns_fork_pin_key(source_namespace, [2; 16], target_a),
		]
	);

	let mut child_edges = vec![
		ns_child_key(source_namespace, [2; 16], target_a),
		ns_child_key(source_namespace, [1; 16], target_b),
		ns_child_key(source_namespace, [1; 16], target_a),
	];
	child_edges.sort();
	assert_eq!(
		child_edges,
		vec![
			ns_child_key(source_namespace, [1; 16], target_a),
			ns_child_key(source_namespace, [1; 16], target_b),
			ns_child_key(source_namespace, [2; 16], target_a),
		]
	);

	let mut retired_objects = vec![
		branch_compaction_retired_cold_object_key(branch, [0x80; 32]),
		branch_compaction_retired_cold_object_key(branch, [0x01; 32]),
		branch_compaction_retired_cold_object_key(branch, [0xff; 32]),
	];
	retired_objects.sort();
	assert_eq!(
		retired_objects,
		vec![
			branch_compaction_retired_cold_object_key(branch, [0x01; 32]),
			branch_compaction_retired_cold_object_key(branch, [0x80; 32]),
			branch_compaction_retired_cold_object_key(branch, [0xff; 32]),
		]
	);
}
