#![allow(dead_code)]

use std::sync::Arc;

use anyhow::{Context, Result};
use gas::prelude::Id;
use rivet_pools::NodeId;
use sqlite_storage::{
	compactor::{SQLITE_COLD_COMPACT_SUBJECT, SqliteColdCompactPayload, cold::ColdCompactorConfig},
	keys::{
		branch_commit_key, branch_manifest_cold_drained_txid_key,
		branch_manifest_last_hot_pass_txid_key, branch_meta_compact_key, branch_shard_key,
		branch_vtx_key, branches_bk_pin_key, branches_list_key, branches_refcount_key,
	},
	pump::{Db, branch},
	types::{
		DatabaseBranchId, DatabaseBranchRecord, BranchState, CommitRow, DirtyPage, MetaCompact,
		NamespaceBranchId, NamespaceId, encode_database_branch_record, encode_commit_row,
		encode_meta_compact,
	},
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

pub const TEST_DATABASE: &str = "database-a";

pub fn namespace() -> Id {
	Id::v1(uuid::Uuid::from_u128(0x1234), 1)
}

pub fn database_branch_id() -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(0x1234_5678_9abc_def0_0123_4567_89ab_cdef))
}

pub fn branch_object_prefix() -> String {
	format!("db/{}", database_branch_id().as_uuid().simple())
}

pub fn bookmark() -> sqlite_storage::types::BookmarkStr {
	sqlite_storage::types::BookmarkStr::new("0000018bcfe56800-0000000000000007")
		.expect("bookmark should be valid")
}

pub fn cold_payload() -> SqliteColdCompactPayload {
	SqliteColdCompactPayload::DeletePinnedBookmark {
		database_id: TEST_DATABASE.to_string(),
		database_branch_id: database_branch_id(),
		bookmark: bookmark(),
		versionstamp: [7; 16],
		pin_object_key: None,
	}
}

pub fn cold_config() -> ColdCompactorConfig {
	ColdCompactorConfig {
		lease_ttl_ms: 200,
		lease_renew_interval_ms: 40,
		lease_margin_ms: 80,
		cold_compact_delta_threshold: 1024,
		phase_a_read_timeout_ms: 5_000,
		max_concurrent_workers: 4,
		ups_subject: SQLITE_COLD_COMPACT_SUBJECT.to_string(),
	}
}

pub fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"sqlite-storage-fault-test".to_string(),
	)))
}

pub fn make_db(db: Arc<universaldb::Database>, database_id: &str) -> Db {
	Db::new(db, test_ups(), namespace(), database_id.to_string(), NodeId::new())
}

pub fn page(pgno: u32, fill: u8) -> DirtyPage {
	DirtyPage {
		pgno,
		bytes: vec![fill; sqlite_storage::keys::PAGE_SIZE as usize],
	}
}

pub async fn test_db(prefix: &str) -> Result<universaldb::Database> {
	let path = Builder::new().prefix(prefix).tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

pub async fn read_value(
	db: &universaldb::Database,
	key: Vec<u8>,
) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

pub async fn read_u64_be(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<u64>> {
	read_value(db, key).await?.map(|value| {
		Ok(u64::from_be_bytes(
			value
				.as_slice()
				.try_into()
				.context("test value should be u64 BE")?,
		))
	}).transpose()
}

pub async fn seed_cold_branch(db: &universaldb::Database) -> Result<()> {
	let branch_id = database_branch_id();

	db.run(move |tx| async move {
		tx.informal().set(
			&branches_list_key(branch_id),
			&encode_database_branch_record(DatabaseBranchRecord {
				branch_id,
				namespace_branch: NamespaceBranchId::nil(),
				parent: None,
				parent_versionstamp: None,
				root_versionstamp: [1; 16],
				fork_depth: 0,
				created_at_ms: 1_000,
				created_from_bookmark: None,
				state: BranchState::Live,
			})?,
		);
		tx.informal()
			.set(&branches_refcount_key(branch_id), &1_i64.to_le_bytes());
		tx.informal()
			.set(&branches_bk_pin_key(branch_id), &[7; 16]);
		tx.informal().set(
			&branch_meta_compact_key(branch_id),
			&encode_meta_compact(MetaCompact {
				materialized_txid: 7,
			})?,
		);
		tx.informal()
			.set(&branch_manifest_cold_drained_txid_key(branch_id), &3u64.to_be_bytes());
		tx.informal()
			.set(&branch_manifest_last_hot_pass_txid_key(branch_id), &7u64.to_be_bytes());
		tx.informal()
			.set(&branch_shard_key(branch_id, 2, 5), b"shard-five");
		tx.informal()
			.set(&sqlite_storage::keys::branch_delta_chunk_key(branch_id, 6, 0), b"delta-six");
		tx.informal().set(
			&branch_commit_key(branch_id, 6),
			&encode_commit_row(CommitRow {
				wall_clock_ms: 2_000,
				versionstamp: [6; 16],
				db_size_pages: 64,
				post_apply_checksum: 99,
			})?,
		);
		tx.informal()
			.set(&branch_vtx_key(branch_id, [6; 16]), &6u64.to_be_bytes());
		Ok(())
	})
	.await
}

pub async fn database_branch_id_for(
	db: &universaldb::Database,
	database_id: &str,
) -> Result<DatabaseBranchId> {
	db.run({
		let database_id = database_id.to_string();
		move |tx| {
			let database_id = database_id.clone();
			async move {
				branch::resolve_database_branch(
					&tx,
					NamespaceId::from_gas_id(namespace()),
					&database_id,
					Serializable,
				)
				.await?
				.context("database branch should exist")
			}
		}
	})
	.await
}
