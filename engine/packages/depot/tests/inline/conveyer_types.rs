use super::{
	BranchState, BucketBranchId, DBHead, DatabaseBranchId, DatabaseBranchRecord, MetaCompact,
	SQLITE_STORAGE_META_VERSION, decode_database_branch_record, decode_db_head,
	decode_meta_compact, encode_database_branch_record, encode_db_head, encode_meta_compact,
};

#[derive(serde::Serialize)]
struct LegacyDatabaseBranchRecordPayload {
	branch_id: DatabaseBranchId,
	bucket_branch: BucketBranchId,
	parent: Option<DatabaseBranchId>,
	parent_versionstamp: Option<[u8; 16]>,
	root_versionstamp: [u8; 16],
	fork_depth: u8,
	created_at_ms: i64,
	created_from_restore_point: Option<super::RestorePointRef>,
	state: BranchState,
}

#[test]
fn db_head_round_trips_with_embedded_version() {
	let head = DBHead {
		head_txid: 42,
		db_size_pages: 128,
		post_apply_checksum: 9,
		branch_id: super::DatabaseBranchId::nil(),
		#[cfg(debug_assertions)]
		generation: 7,
	};

	let encoded = encode_db_head(head.clone()).expect("db head should encode");
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_STORAGE_META_VERSION
	);

	let decoded = decode_db_head(&encoded).expect("db head should decode");
	assert_eq!(decoded, head);
}

#[test]
fn database_branch_record_round_trips_current_version() {
	let record = DatabaseBranchRecord {
		branch_id: DatabaseBranchId::nil(),
		bucket_branch: BucketBranchId::nil(),
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [3; 16],
		fork_depth: 2,
		created_at_ms: 1_000,
		created_from_restore_point: None,
		state: BranchState::Live,
		lifecycle_generation: 9,
	};

	let encoded = encode_database_branch_record(record.clone())
		.expect("database branch record should encode");
	assert_eq!(u16::from_le_bytes([encoded[0], encoded[1]]), 1);

	let decoded =
		decode_database_branch_record(&encoded).expect("database branch record should decode");
	assert_eq!(decoded, record);
}

#[test]
fn database_branch_record_rejects_legacy_v1_without_lifecycle_generation() {
	let legacy_payload = LegacyDatabaseBranchRecordPayload {
		branch_id: DatabaseBranchId::nil(),
		bucket_branch: BucketBranchId::nil(),
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: [3; 16],
		fork_depth: 2,
		created_at_ms: 1_000,
		created_from_restore_point: None,
		state: BranchState::Live,
	};
	let mut encoded = 1_u16.to_le_bytes().to_vec();
	encoded.extend(serde_bare::to_vec(&legacy_payload).expect("legacy bare payload should encode"));

	let err = decode_database_branch_record(&encoded)
		.expect_err("legacy branch record should be rejected");
	assert!(
		err.chain().any(|cause| cause
			.to_string()
			.contains("decode sqlite database branch record")),
		"unexpected error chain: {err:#}"
	);
}

#[test]
fn meta_compact_round_trips_with_embedded_version() {
	let compact = MetaCompact {
		materialized_txid: 24,
	};

	let encoded = encode_meta_compact(compact.clone()).expect("compact meta should encode");
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_STORAGE_META_VERSION
	);

	let decoded = decode_meta_compact(&encoded).expect("compact meta should decode");
	assert_eq!(decoded, compact);
}
