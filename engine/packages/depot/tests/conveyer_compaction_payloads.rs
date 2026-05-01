use depot::types::{
	BookmarkStr, ColdShardRef, CompactionRoot, DatabaseBranchId, DbHistoryPin, DbHistoryPinKind,
	NamespaceBranchId, RetiredColdObject, RetiredColdObjectDeleteState,
	SQLITE_STORAGE_META_VERSION, SqliteCmpDirty, decode_cold_shard_ref,
	decode_compaction_root, decode_db_history_pin, decode_retired_cold_object,
	decode_sqlite_cmp_dirty, encode_cold_shard_ref, encode_compaction_root,
	encode_db_history_pin, encode_retired_cold_object, encode_sqlite_cmp_dirty,
};
use gas::prelude::Id;
use uuid::Uuid;

fn database_branch_id(value: u128) -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(Uuid::from_u128(value))
}

fn namespace_branch_id(value: u128) -> NamespaceBranchId {
	NamespaceBranchId::from_uuid(Uuid::from_u128(value))
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
fn db_history_pin_round_trips_each_kind_with_embedded_version() {
	let cases = [
		DbHistoryPin {
			at_versionstamp: [5; 16],
			at_txid: 50,
			kind: DbHistoryPinKind::DatabaseFork,
			owner_database_branch_id: Some(database_branch_id(
				0x1111_2222_3333_4444_5555_6666_7777_8888,
			)),
			owner_namespace_branch_id: None,
			owner_bookmark: None,
			created_at_ms: 1_714_000_000_001,
		},
		DbHistoryPin {
			at_versionstamp: [6; 16],
			at_txid: 60,
			kind: DbHistoryPinKind::NamespaceFork,
			owner_database_branch_id: None,
			owner_namespace_branch_id: Some(namespace_branch_id(
				0x9999_aaaa_bbbb_cccc_dddd_eeee_ffff_0000,
			)),
			owner_bookmark: None,
			created_at_ms: 1_714_000_000_002,
		},
		DbHistoryPin {
			at_versionstamp: [7; 16],
			at_txid: 70,
			kind: DbHistoryPinKind::Bookmark,
			owner_database_branch_id: None,
			owner_namespace_branch_id: None,
			owner_bookmark: Some(
				BookmarkStr::new("0000018bcfe56800-0000000000000046")
					.expect("bookmark should be valid"),
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
