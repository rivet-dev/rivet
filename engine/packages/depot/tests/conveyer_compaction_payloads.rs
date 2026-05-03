use depot::types::{
	BucketBranchId, BucketCatalogDbFact, BucketForkFact, ColdShardRef, CompactionRoot,
	DatabaseBranchId, DbHistoryPin, DbHistoryPinKind, PitrIntervalCoverage, RestorePointId,
	RetiredColdObject, RetiredColdObjectDeleteState, SQLITE_STORAGE_META_VERSION, SqliteCmpDirty,
	decode_bucket_catalog_db_fact, decode_bucket_fork_fact, decode_cold_shard_ref,
	decode_compaction_root, decode_db_history_pin, decode_pitr_interval_coverage,
	decode_retired_cold_object, decode_sqlite_cmp_dirty, encode_bucket_catalog_db_fact,
	encode_bucket_fork_fact, encode_cold_shard_ref, encode_compaction_root, encode_db_history_pin,
	encode_pitr_interval_coverage, encode_retired_cold_object, encode_sqlite_cmp_dirty,
};
use gas::prelude::Id;
use uuid::Uuid;

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
}

fn bucket_branch_id(value: u128) -> BucketBranchId {
	BucketBranchId::from_uuid(Uuid::from_u128(value))
}

fn gas_id(value: u128, label: u16) -> Id {
	Id::v1(Uuid::from_u128(value), label)
}

fn assert_embedded_version(encoded: &[u8]) {
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_STORAGE_META_VERSION
	);
}

#[test]
fn compaction_root_round_trips_with_embedded_version() {
	let root = CompactionRoot {
		schema_version: 1,
		manifest_generation: 42,
		hot_watermark_txid: 100,
		cold_watermark_txid: 80,
		cold_watermark_versionstamp: [7; 16],
	};

	let encoded = encode_compaction_root(root.clone()).expect("root should encode");
	assert_embedded_version(&encoded);

	let decoded = decode_compaction_root(&encoded).expect("root should decode");
	assert_eq!(decoded, root);
}

#[test]
fn cold_shard_ref_round_trips_with_embedded_version() {
	let reference = ColdShardRef {
		object_key: "db/branch/shard/00000001/0000000000000064-job-hash.ltx".into(),
		object_generation_id: gas_id(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff, 9),
		shard_id: 3,
		as_of_txid: 100,
		min_txid: 50,
		max_txid: 100,
		min_versionstamp: [1; 16],
		max_versionstamp: [2; 16],
		size_bytes: 96 * 1024,
		content_hash: [3; 32],
		publish_generation: 43,
	};

	let encoded = encode_cold_shard_ref(reference.clone()).expect("cold ref should encode");
	assert_embedded_version(&encoded);

	let decoded = decode_cold_shard_ref(&encoded).expect("cold ref should decode");
	assert_eq!(decoded, reference);
}

#[test]
fn retired_cold_object_round_trips_with_embedded_version() {
	let object = RetiredColdObject {
		object_key: "db/branch/shard/00000001/0000000000000064-job-hash.ltx".into(),
		object_generation_id: gas_id(0x1020_3040_5060_7080_90a0_b0c0_d0e0_f000, 11),
		content_hash: [4; 32],
		retired_manifest_generation: 44,
		retired_at_ms: 1_714_000_000_000,
		delete_after_ms: 1_714_000_060_000,
		delete_state: RetiredColdObjectDeleteState::DeleteIssued,
	};

	let encoded = encode_retired_cold_object(object.clone()).expect("retired object should encode");
	assert_embedded_version(&encoded);

	let decoded = decode_retired_cold_object(&encoded).expect("retired object should decode");
	assert_eq!(decoded, object);
}

#[test]
fn sqlite_cmp_dirty_round_trips_with_embedded_version() {
	let dirty = SqliteCmpDirty {
		observed_head_txid: 123,
		updated_at_ms: 1_714_000_000_000,
	};

	let encoded = encode_sqlite_cmp_dirty(dirty.clone()).expect("dirty marker should encode");
	assert_embedded_version(&encoded);

	let decoded = decode_sqlite_cmp_dirty(&encoded).expect("dirty marker should decode");
	assert_eq!(decoded, dirty);
}

#[test]
fn pitr_interval_coverage_round_trips_with_embedded_version() {
	let coverage = PitrIntervalCoverage {
		txid: 42,
		versionstamp: [8; 16],
		wall_clock_ms: 1_700_000_123_456,
		expires_at_ms: 1_700_604_923_456,
	};

	let encoded = encode_pitr_interval_coverage(coverage.clone())
		.expect("PITR interval coverage should encode");
	assert_embedded_version(&encoded);

	let decoded =
		decode_pitr_interval_coverage(&encoded).expect("PITR interval coverage should decode");
	assert_eq!(decoded, coverage);
}

#[test]
fn db_history_pin_round_trips_each_kind_with_embedded_version() {
	let cases = [
		DbHistoryPin {
			at_versionstamp: [5; 16],
			at_txid: 50,
			kind: DbHistoryPinKind::DatabaseFork,
			owner_database_branch_id: Some(database_branch_id(
				0x1111_2222_3333_4444_5555_6666_7777_8888,
			)),
			owner_bucket_branch_id: None,
			owner_restore_point: None,
			created_at_ms: 1_714_000_000_001,
		},
		DbHistoryPin {
			at_versionstamp: [6; 16],
			at_txid: 60,
			kind: DbHistoryPinKind::BucketFork,
			owner_database_branch_id: None,
			owner_bucket_branch_id: Some(bucket_branch_id(
				0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0000,
			)),
			owner_restore_point: None,
			created_at_ms: 1_714_000_000_002,
		},
		DbHistoryPin {
			at_versionstamp: [7; 16],
			at_txid: 70,
			kind: DbHistoryPinKind::RestorePoint,
			owner_database_branch_id: None,
			owner_bucket_branch_id: None,
			owner_restore_point: Some(
				RestorePointId::new("0000018bcfe56800-0000000000000046")
					.expect("restore_point should be valid"),
			),
			created_at_ms: 1_714_000_000_003,
		},
	];

	for pin in cases {
		let encoded = encode_db_history_pin(pin.clone()).expect("history pin should encode");
		assert_embedded_version(&encoded);

		let decoded = decode_db_history_pin(&encoded).expect("history pin should decode");
		assert_eq!(decoded, pin);
	}
}

#[test]
fn bucket_proof_facts_round_trip_with_embedded_version() {
	let source_bucket_branch_id = bucket_branch_id(0x1111_2222_3333_4444_5555_6666_7777_8888);
	let target_bucket_branch_id = bucket_branch_id(0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0000);
	let fork_fact = BucketForkFact {
		source_bucket_branch_id,
		target_bucket_branch_id,
		fork_versionstamp: [8; 16],
		parent_cap_versionstamp: [8; 16],
	};
	let catalog_fact = BucketCatalogDbFact {
		database_branch_id: database_branch_id(0x1234_5678_90ab_cdef_1111_2222_3333_4444),
		bucket_branch_id: source_bucket_branch_id,
		catalog_versionstamp: [7; 16],
		tombstone_versionstamp: Some([9; 16]),
	};

	let encoded_fork =
		encode_bucket_fork_fact(fork_fact.clone()).expect("bucket fork fact should encode");
	assert_embedded_version(&encoded_fork);
	assert_eq!(
		decode_bucket_fork_fact(&encoded_fork).expect("bucket fork fact should decode"),
		fork_fact
	);

	let encoded_catalog = encode_bucket_catalog_db_fact(catalog_fact.clone())
		.expect("bucket catalog fact should encode");
	assert_embedded_version(&encoded_catalog);
	assert_eq!(
		decode_bucket_catalog_db_fact(&encoded_catalog).expect("bucket catalog fact should decode"),
		catalog_fact
	);
}
