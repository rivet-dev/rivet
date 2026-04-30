use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use async_trait::async_trait;
use rivet_pools::NodeId;
use sqlite_storage::{
	cold_tier::{ColdTier, ColdTierObjectMetadata, FilesystemColdTier},
	compactor::{
		SQLITE_COLD_COMPACT_PAYLOAD_VERSION, SQLITE_COLD_COMPACT_SUBJECT,
		SqliteColdCompactPayload, SqliteColdCompactSubject,
		cold::{
			ColdCompactorConfig, ColdCompactorLease, ColdRenewOutcome, ColdTakeOutcome,
			decode_cold_compact_state, decode_cold_lease, decode_pending_marker,
			encode_cold_compact_state, encode_cold_lease, encode_pending_marker, release, renew,
			take, worker,
		},
		decode_cold_compact_payload, encode_cold_compact_payload,
	},
	keys::{
		branch_commit_key, branch_manifest_cold_drained_txid_key,
		branch_manifest_last_hot_pass_txid_key, branch_meta_cold_compact_key,
		branch_meta_cold_lease_key, branch_meta_compact_key, branch_shard_key, branch_vtx_key,
		branches_list_key,
	},
	types::{
		ActorBranchId, ActorBranchRecord, BookmarkStr, BranchState, CommitRow, LayerKind,
		MetaCompact, PinStatus, PinnedBookmarkRecord, decode_cold_manifest_chunk,
		decode_cold_manifest_index, decode_pinned_bookmark_record, decode_pointer_snapshot,
		encode_actor_branch_record, encode_commit_row, encode_meta_compact,
		encode_pinned_bookmark_record,
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

fn branch_object_prefix() -> String {
	format!("db/{}", actor_branch_id().as_uuid().simple())
}

fn payload() -> SqliteColdCompactPayload {
	SqliteColdCompactPayload::CreatePinnedBookmark {
		actor_id: "actor-a".to_string(),
		actor_branch_id: actor_branch_id(),
		bookmark: bookmark(),
		versionstamp: [7; 16],
	}
}

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new().prefix("sqlite-storage-cold-compactor-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

async fn read_lease(db: &universaldb::Database) -> Result<Option<ColdCompactorLease>> {
	db.run(|tx| async move {
		let Some(value) = tx
			.informal()
			.get(&branch_meta_cold_lease_key(actor_branch_id()), Snapshot)
			.await?
		else {
			return Ok(None);
		};

		Ok(Some(decode_cold_lease(&value)?))
	})
	.await
}

async fn write_lease(db: &universaldb::Database, lease: ColdCompactorLease) -> Result<()> {
	db.run(move |tx| async move {
		tx.informal()
			.set(&branch_meta_cold_lease_key(actor_branch_id()), &encode_cold_lease(lease)?);
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

async fn read_cold_state(
	db: &universaldb::Database,
) -> Result<Option<sqlite_storage::compactor::cold::ColdCompactState>> {
	read_value(db, branch_meta_cold_compact_key(actor_branch_id()))
		.await?
		.as_deref()
		.map(decode_cold_compact_state)
		.transpose()
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
	tier: &FilesystemColdTier,
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

async fn seed_phase_a_branch(db: &universaldb::Database) -> Result<()> {
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
		tx.informal().set(
			&sqlite_storage::keys::bookmark_pinned_key("actor-a", bookmark().as_str()),
			&encode_pinned_bookmark_record(PinnedBookmarkRecord {
				bookmark: bookmark(),
				actor_branch_id: branch_id,
				versionstamp: [7; 16],
				status: PinStatus::Pending,
				pin_object_key: None,
				created_at_ms: 1_000,
				updated_at_ms: 1_000,
			})?,
		);
		Ok(())
	})
	.await
}

#[derive(Clone)]
struct ObservingColdTier {
	inner: FilesystemColdTier,
	db: Arc<universaldb::Database>,
	saw_committed_handoff: Arc<AtomicBool>,
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
				let state = sqlite_storage::compactor::cold::ColdCompactState {
					cold_drained_txid: 4,
					in_flight_uuid: Some(uuid::Uuid::nil()),
				};
				tx.informal().set(
					&branch_meta_cold_compact_key(actor_branch_id()),
					&encode_cold_compact_state(state)?,
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

#[test]
fn cold_subject_uses_constant_subject_string() {
	assert_eq!(SqliteColdCompactSubject.to_string(), SQLITE_COLD_COMPACT_SUBJECT);
	assert_eq!(SQLITE_COLD_COMPACT_SUBJECT, "sqlite.cold_compact");
}

#[test]
fn cold_payload_round_trips_with_embedded_version() {
	let encoded = encode_cold_compact_payload(payload()).expect("payload should encode");
	assert_eq!(
		u16::from_le_bytes([encoded[0], encoded[1]]),
		SQLITE_COLD_COMPACT_PAYLOAD_VERSION
	);

	let decoded = decode_cold_compact_payload(&encoded).expect("payload should decode");
	assert_eq!(decoded, payload());
}

#[tokio::test]
async fn cold_lease_acquire_renew_and_release() -> Result<()> {
	let db = test_db().await?;
	let holder = NodeId::new();
	let branch_id = actor_branch_id();

	let outcome = db
		.run(move |tx| async move { take(&tx, branch_id, holder, 30_000, 1_000).await })
		.await?;
	assert_eq!(outcome, ColdTakeOutcome::Acquired);
	assert_eq!(
		read_lease(&db).await?,
		Some(ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: 31_000,
		})
	);

	let outcome = db
		.run(move |tx| async move { renew(&tx, branch_id, holder, 40_000, 2_000).await })
		.await?;
	assert_eq!(outcome, ColdRenewOutcome::Renewed);
	assert_eq!(
		read_lease(&db).await?,
		Some(ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: 42_000,
		})
	);

	db.run(move |tx| async move { release(&tx, branch_id, holder).await })
		.await?;
	assert_eq!(read_lease(&db).await?, None);

	Ok(())
}

#[tokio::test]
async fn cold_worker_handles_payload_and_releases_lease() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let cold_root = Builder::new().prefix("sqlite-cold-worker").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig {
			lease_ttl_ms: 200,
			lease_renew_interval_ms: 40,
			lease_margin_ms: 80,
			cold_compact_delta_threshold: 1024,
			phase_a_read_timeout_ms: 5_000,
			max_concurrent_workers: 4,
			ups_subject: SQLITE_COLD_COMPACT_SUBJECT.to_string(),
		},
		CancellationToken::new(),
		tier,
	)
	.await?;

	assert_eq!(read_lease(&db).await?, None);

	Ok(())
}

#[tokio::test]
async fn cold_phase_a_writes_handoff_then_pending_marker_and_snapshot_plan() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_phase_a_branch(&db).await?;

	let cold_root = Builder::new().prefix("sqlite-cold-phase-a").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig::default(),
		CancellationToken::new(),
		tier.clone(),
	)
	.await?;

	let state = read_cold_state(&db)
		.await?
		.expect("cold compact state should be written");
	assert_eq!(state.cold_drained_txid, 7);
	assert_eq!(state.in_flight_uuid, None);

	let (_marker_key, marker) = single_pending_marker(&tier).await?;

	assert_eq!(marker.branch_id, actor_branch_id());
	assert_eq!(marker.cold_drained_txid, 3);
	assert_eq!(marker.materialized_txid, 7);
	assert_eq!(marker.last_hot_pass_txid, 7);
	assert!(
		marker
			.planned_object_keys
			.contains(&format!("{}/branch_record.bare", branch_object_prefix()))
	);
	assert!(
		marker
			.planned_object_keys
			.iter()
			.any(|key| key.ends_with("/delta/0000000000000006-0000000000000006.ltx"))
	);
	assert!(
		marker
			.planned_object_keys
			.iter()
			.any(|key| key.ends_with("/pin/07070707070707070707070707070707.ltx"))
	);

	Ok(())
}

#[tokio::test]
async fn cold_phase_b_uploads_layers_manifest_snapshot_and_cleans_stale_markers() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_phase_a_branch(&db).await?;

	let cold_root = Builder::new().prefix("sqlite-cold-phase-b").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));
	let stale_object_key = format!("{}/delta/leaked.ltx", branch_object_prefix());
	tier.put_object(&stale_object_key, b"leaked").await?;
	let stale_uuid = uuid::Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
	let stale_marker_key = format!(
		"{}/pending/{}.marker",
		branch_object_prefix(),
		stale_uuid.simple()
	);
	tier.put_object(
		&stale_marker_key,
		&encode_pending_marker(sqlite_storage::compactor::cold::ColdPendingMarker {
			schema_version: sqlite_storage::types::SQLITE_STORAGE_COLD_SCHEMA_VERSION,
			branch_id: actor_branch_id(),
			pass_uuid: stale_uuid,
			created_at_ms: 0,
			cold_drained_txid: 0,
			materialized_txid: 1,
			last_hot_pass_txid: 1,
			planned_object_keys: vec![stale_object_key.clone()],
		})?,
	)
	.await?;

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig::default(),
		CancellationToken::new(),
		tier.clone(),
	)
	.await?;

	let state = read_cold_state(&db)
		.await?
		.expect("cold compact state should be written");
	assert_eq!(state.cold_drained_txid, 7);
	assert_eq!(state.in_flight_uuid, None);
	assert_eq!(
		read_u64_be(&db, branch_manifest_cold_drained_txid_key(actor_branch_id())).await?,
		Some(7)
	);

	let (_marker_key, marker) = single_pending_marker(&tier).await?;
	let pass_uuid = marker.pass_uuid;

	let image_key = format!(
		"{}/image/00000000/00000002-0000000000000005.ltx",
		branch_object_prefix()
	);
	let delta_key = format!(
		"{}/delta/0000000000000006-0000000000000006.ltx",
		branch_object_prefix()
	);
	let pin_key = format!("{}/pin/07070707070707070707070707070707.ltx", branch_object_prefix());
	let chunk_key = format!(
		"{}/cold_manifest/chunks/{}.bare",
		branch_object_prefix(),
		pass_uuid.simple()
	);
	let index_key = format!("{}/cold_manifest/index.bare", branch_object_prefix());
	let snapshot_key = format!(
		"{}/pointer_snapshot/{}.bare",
		branch_object_prefix(),
		pass_uuid.simple()
	);

	assert_eq!(
		tier.get_object(&image_key).await?,
		Some(b"shard-five".to_vec())
	);
	assert_eq!(
		tier.get_object(&delta_key).await?,
		Some(b"delta-six".to_vec())
	);
	assert_eq!(tier.get_object(&pin_key).await?, Some(b"shard-five".to_vec()));
	assert!(
		tier.get_object(&format!("{}/branch_record.bare", branch_object_prefix()))
			.await?
			.is_some()
	);

	let chunk = decode_cold_manifest_chunk(
		&tier
			.get_object(&chunk_key)
			.await?
			.expect("manifest chunk should exist"),
	)?;
	assert_eq!(chunk.branch_id, actor_branch_id());
	assert_eq!(chunk.layers.len(), 3);
	assert!(chunk.layers.iter().any(|layer| layer.kind == LayerKind::Image));
	assert!(chunk.layers.iter().any(|layer| layer.kind == LayerKind::Delta));
	assert!(chunk.layers.iter().any(|layer| layer.kind == LayerKind::Pin));
	assert_eq!(chunk.bookmarks.len(), 1);
	assert_eq!(chunk.bookmarks[0].pin_object_key.as_deref(), Some(pin_key.as_str()));

	let index = decode_cold_manifest_index(
		&tier
			.get_object(&index_key)
			.await?
			.expect("manifest index should exist"),
	)?;
	assert_eq!(index.branch_id, actor_branch_id());
	assert_eq!(index.chunks.len(), 1);
	assert_eq!(index.chunks[0].object_key, chunk_key);

	let snapshot = decode_pointer_snapshot(
		&tier
			.get_object(&snapshot_key)
			.await?
			.expect("pointer snapshot should exist"),
	)?;
	assert_eq!(snapshot.actors.len(), 1);
	assert_eq!(snapshot.actors[0].0, "actor-a");
	assert_eq!(snapshot.actors[0].2, actor_branch_id());

	assert_eq!(tier.get_object(&stale_object_key).await?, None);
	assert_eq!(tier.get_object(&stale_marker_key).await?, None);

	let marker_key = format!("{}/pending/{}.marker", branch_object_prefix(), pass_uuid.simple());
	let marker = decode_pending_marker(
		&tier
			.get_object(&marker_key)
			.await?
			.expect("current pending marker should exist"),
	)?;
	assert!(marker.planned_object_keys.contains(&image_key));
	assert!(marker.planned_object_keys.contains(&chunk_key));
	assert!(marker.planned_object_keys.contains(&snapshot_key));

	let pinned_bytes = read_value(
		&db,
		sqlite_storage::keys::bookmark_pinned_key("actor-a", bookmark().as_str()),
	)
	.await?
	.expect("pinned bookmark record should exist");
	let pinned = decode_pinned_bookmark_record(&pinned_bytes)?;
	assert_eq!(pinned.status, PinStatus::Ready);
	assert_eq!(pinned.pin_object_key.as_deref(), Some(pin_key.as_str()));

	Ok(())
}

#[tokio::test]
async fn cold_phase_c_aborts_when_cold_drained_txid_changes() -> Result<()> {
	let db = Arc::new(test_db().await?);
	seed_phase_a_branch(&db).await?;

	let cold_root = Builder::new().prefix("sqlite-cold-phase-c-occ").tempdir()?;
	let tier = Arc::new(AdvancingColdDrainedTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		db: Arc::clone(&db),
		advanced: Arc::new(AtomicBool::new(false)),
	});

	let err = worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig::default(),
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
	let pinned_bytes = read_value(
		&db,
		sqlite_storage::keys::bookmark_pinned_key("actor-a", bookmark().as_str()),
	)
	.await?
	.expect("pinned bookmark record should exist");
	let pinned = decode_pinned_bookmark_record(&pinned_bytes)?;
	assert_eq!(pinned.status, PinStatus::Pending);

	Ok(())
}

#[tokio::test]
async fn cold_phase_a_puts_pending_marker_after_handoff_tx_commits() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let cold_root = Builder::new().prefix("sqlite-cold-phase-a").tempdir()?;
	let saw_committed_handoff = Arc::new(AtomicBool::new(false));
	let tier = Arc::new(ObservingColdTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		db: Arc::clone(&db),
		saw_committed_handoff: Arc::clone(&saw_committed_handoff),
	});

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig::default(),
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
async fn cold_worker_skips_when_another_holder_has_lease() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let holder = NodeId::new();
	write_lease(
		&db,
		ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: i64::MAX,
		},
	)
	.await?;

	worker::test_hooks::handle_payload_once(
		Arc::clone(&db),
		payload(),
		ColdCompactorConfig::default(),
		CancellationToken::new(),
	)
	.await?;

	assert_eq!(
		read_lease(&db).await?,
		Some(ColdCompactorLease {
			holder_id: holder,
			expires_at_ms: i64::MAX,
		})
	);

	Ok(())
}
