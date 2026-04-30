use std::{
	collections::BTreeMap,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use sqlite_storage::{
	cold_tier::{ColdTier, ColdTierObjectMetadata, FilesystemColdTier},
	compactor::{
		SQLITE_COLD_COMPACT_SUBJECT, SqliteColdCompactPayload,
		cold::{
			ColdCompactorConfig, decode_cold_compact_state, decode_pending_marker,
			encode_cold_compact_state, worker,
		},
	},
	keys::{
		branch_commit_key, branch_manifest_cold_drained_txid_key,
		branch_manifest_last_hot_pass_txid_key, branch_meta_cold_compact_key,
		branch_meta_compact_key, branch_shard_key, branch_vtx_key, branches_bk_pin_key,
		branches_list_key, branches_refcount_key,
	},
	types::{
		ActorBranchId, ActorBranchRecord, BookmarkStr, BranchState, ColdManifestIndex,
		CommitRow, MetaCompact, decode_cold_manifest_index, encode_actor_branch_record,
		encode_commit_row, encode_meta_compact,
	},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;
use universaldb::utils::IsolationLevel::Snapshot;

fn actor_branch_id() -> ActorBranchId {
	ActorBranchId::from_uuid(uuid::Uuid::from_u128(0x1234_5678_9abc_def0_0123_4567_89ab_cdef))
}

fn bookmark() -> BookmarkStr {
	BookmarkStr::new("0000018bcfe56800-0000000000000007").expect("bookmark should be valid")
}

fn payload() -> SqliteColdCompactPayload {
	SqliteColdCompactPayload::DeletePinnedBookmark {
		actor_id: "actor-a".to_string(),
		actor_branch_id: actor_branch_id(),
		bookmark: bookmark(),
		versionstamp: [7; 16],
		pin_object_key: None,
	}
}

fn branch_object_prefix() -> String {
	format!("db/{}", actor_branch_id().as_uuid().simple())
}

fn cold_config() -> ColdCompactorConfig {
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

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("sqlite-storage-cold-compactor-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

async fn seed_branch(db: &universaldb::Database) -> Result<()> {
	let branch_id = actor_branch_id();

	db.run(move |tx| async move {
		tx.informal().set(
			&branches_list_key(branch_id),
			&encode_actor_branch_record(ActorBranchRecord {
				branch_id,
				namespace_branch: sqlite_storage::types::NamespaceBranchId::nil(),
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

async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
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

async fn read_u64_be(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<u64>> {
	let Some(value) = read_value(db, key).await? else {
		return Ok(None);
	};
	let bytes: [u8; std::mem::size_of::<u64>()] =
		value.as_slice().try_into().expect("test value should be u64");

	Ok(Some(u64::from_be_bytes(bytes)))
}

async fn single_pending_marker(
	tier: &dyn ColdTier,
) -> Result<(String, sqlite_storage::compactor::cold::ColdPendingMarker)> {
	let prefix = format!("{}/pending/", branch_object_prefix());
	let markers = tier.list_prefix(&prefix).await?;
	assert_eq!(markers.len(), 1, "expected one pending marker");
	let key = markers[0].key.clone();
	let bytes = tier
		.get_object(&key)
		.await?
		.expect("pending marker should exist");

	Ok((key, decode_pending_marker(&bytes)?))
}

#[derive(Clone)]
struct ObservingColdTier {
	inner: FilesystemColdTier,
	db: Arc<universaldb::Database>,
	saw_committed_handoff: Arc<AtomicBool>,
}

#[async_trait]
impl ColdTier for ObservingColdTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		if key.ends_with(".marker") {
			let state_exists =
				read_value(&self.db, branch_meta_cold_compact_key(actor_branch_id()))
					.await?
					.is_some();
			self.saw_committed_handoff
				.store(state_exists, Ordering::SeqCst);
		}

		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

#[derive(Clone)]
struct PhaseBFdbProbeTier {
	inner: FilesystemColdTier,
	db: Arc<universaldb::Database>,
	probed: Arc<AtomicBool>,
}

#[async_trait]
impl ColdTier for PhaseBFdbProbeTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		if key.contains("/image/") && !self.probed.swap(true, Ordering::SeqCst) {
			let db = Arc::clone(&self.db);
			db.run(|tx| async move {
				tx.informal().set(
					&branch_manifest_last_hot_pass_txid_key(actor_branch_id()),
					&8u64.to_be_bytes(),
				);
				Ok(())
			})
			.await?;
		}

		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

#[derive(Clone)]
struct AdvancingColdDrainedTier {
	inner: FilesystemColdTier,
	db: Arc<universaldb::Database>,
	advanced: Arc<AtomicBool>,
}

#[async_trait]
impl ColdTier for AdvancingColdDrainedTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		if key.ends_with("/cold_manifest/index.bare")
			&& !self.advanced.swap(true, Ordering::SeqCst)
		{
			let db = Arc::clone(&self.db);
			db.run(|tx| async move {
				tx.informal().set(
					&branch_meta_cold_compact_key(actor_branch_id()),
					&encode_cold_compact_state(sqlite_storage::compactor::cold::ColdCompactState {
						cold_drained_txid: 4,
						in_flight_uuid: Some(uuid::Uuid::nil()),
					})?,
				);
				Ok(())
			})
			.await?;
		}

		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

#[derive(Clone)]
struct CrashAfterPhaseBTier {
	inner: FilesystemColdTier,
	crashed: Arc<AtomicBool>,
	puts: Arc<Mutex<BTreeMap<String, usize>>>,
}

impl CrashAfterPhaseBTier {
	fn put_count(&self, key: &str) -> usize {
		self.puts.lock().get(key).copied().unwrap_or_default()
	}
}

#[async_trait]
impl ColdTier for CrashAfterPhaseBTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		{
			let mut puts = self.puts.lock();
			*puts.entry(key.to_string()).or_default() += 1;
		}

		self.inner.put_object(key, bytes).await?;

		if key.contains("/pointer_snapshot/") && !self.crashed.swap(true, Ordering::SeqCst) {
			anyhow::bail!("injected phase B crash after uploads");
		}

		Ok(())
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

#[tokio::test]
async fn phase_a_commits_handoff_before_pending_marker_put() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-cold-compactor-a").tempdir()?;
	let saw_committed_handoff = Arc::new(AtomicBool::new(false));
	let tier = Arc::new(ObservingColdTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		db: Arc::clone(&db),
		saw_committed_handoff: Arc::clone(&saw_committed_handoff),
	});

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		cold_config(),
		CancellationToken::new(),
		tier,
	)
	.await?;

	assert!(
		saw_committed_handoff.load(Ordering::SeqCst),
		"pending marker PUT should observe the committed FDB handoff"
	);

	Ok(())
}

#[tokio::test]
async fn phase_b_uploads_allow_independent_fdb_work() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-cold-compactor-b").tempdir()?;
	let probed = Arc::new(AtomicBool::new(false));
	let tier = Arc::new(PhaseBFdbProbeTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		db: Arc::clone(&db),
		probed: Arc::clone(&probed),
	});

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		cold_config(),
		CancellationToken::new(),
		tier,
	)
	.await?;

	assert!(
		probed.load(Ordering::SeqCst),
		"phase B image upload should run the FDB probe"
	);
	assert_eq!(
		read_u64_be(&db, branch_manifest_last_hot_pass_txid_key(actor_branch_id())).await?,
		Some(8)
	);
	assert_eq!(
		read_u64_be(&db, branch_manifest_cold_drained_txid_key(actor_branch_id())).await?,
		Some(7)
	);

	Ok(())
}

#[tokio::test]
async fn phase_c_aborts_when_cold_drained_txid_changes_after_phase_a() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-cold-compactor-c").tempdir()?;
	let tier = Arc::new(AdvancingColdDrainedTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		db: Arc::clone(&db),
		advanced: Arc::new(AtomicBool::new(false)),
	});

	let err = worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		cold_config(),
		CancellationToken::new(),
		tier,
	)
	.await
	.expect_err("phase C should reject a changed cold drain cursor");

	assert!(
		format!("{err:?}").contains("cold_drained_txid fence changed"),
		"unexpected error: {err:?}"
	);
	assert_eq!(
		read_u64_be(&db, branch_manifest_cold_drained_txid_key(actor_branch_id())).await?,
		Some(3)
	);

	Ok(())
}

#[tokio::test]
async fn retry_after_phase_b_crash_reuses_inflight_uuid_and_overwrites_layers() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-cold-compactor-retry").tempdir()?;
	let tier = Arc::new(CrashAfterPhaseBTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		crashed: Arc::new(AtomicBool::new(false)),
		puts: Arc::new(Mutex::new(BTreeMap::new())),
	});

	let err = worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		cold_config(),
		CancellationToken::new(),
		tier.clone(),
	)
	.await
	.expect_err("first pass should crash after phase B uploads");
	assert!(
		format!("{err:?}").contains("injected phase B crash after uploads"),
		"unexpected error: {err:?}"
	);

	let (marker_key, marker) = single_pending_marker(tier.as_ref()).await?;
	let image_key = format!(
		"{}/image/00000000/00000002-0000000000000005.ltx",
		branch_object_prefix()
	);
	let delta_key = format!(
		"{}/delta/0000000000000006-0000000000000006.ltx",
		branch_object_prefix()
	);
	let snapshot_key = format!(
		"{}/pointer_snapshot/{}.bare",
		branch_object_prefix(),
		marker.pass_uuid.simple()
	);
	assert_eq!(
		tier.get_object(&image_key).await?,
		Some(b"shard-five".to_vec())
	);
	assert!(
		tier.get_object(&snapshot_key).await?.is_some(),
		"phase B crash happens after the snapshot upload is durable"
	);

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		cold_config(),
		CancellationToken::new(),
		tier.clone(),
	)
	.await?;

	let (retry_marker_key, retry_marker) = single_pending_marker(tier.as_ref()).await?;
	assert_eq!(retry_marker_key, marker_key);
	assert_eq!(retry_marker.pass_uuid, marker.pass_uuid);
	assert!(
		tier.put_count(&image_key) >= 2,
		"retry should overwrite the deterministic image layer key"
	);
	assert!(
		tier.put_count(&delta_key) >= 2,
		"retry should overwrite the deterministic delta layer key"
	);

	let index_key = format!("{}/cold_manifest/index.bare", branch_object_prefix());
	let index: ColdManifestIndex = decode_cold_manifest_index(
		&tier
			.get_object(&index_key)
			.await?
			.expect("manifest index should exist after retry"),
	)?;
	assert_eq!(
		index.chunks.len(),
		1,
		"retry should replace the previous chunk ref for the same pass"
	);
	assert_eq!(
		read_u64_be(&db, branch_manifest_cold_drained_txid_key(actor_branch_id())).await?,
		Some(7)
	);
	let state = read_value(&db, branch_meta_cold_compact_key(actor_branch_id()))
		.await?
		.expect("cold compact state should exist");
	assert_eq!(decode_cold_compact_state(&state)?.in_flight_uuid, None);

	Ok(())
}
