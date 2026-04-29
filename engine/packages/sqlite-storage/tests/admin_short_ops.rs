mod support;

use std::sync::Arc;

use anyhow::Result;
use gas::prelude::Id;
use namespace::types::SqliteNamespaceConfig;
use rivet_error::RivetError;
use sqlite_storage::{
	admin::{self, AuditFields, OpKind, OpResult, RefcountKind},
	compactor::{
		self, ClearRefcountRequest, DescribeRetentionRequest, GetRetentionRequest,
		SetRetentionRequest,
	},
	keys::{checkpoint_meta_key, meta_checkpoints_key},
	types::{
		CheckpointEntry, CheckpointMeta, Checkpoints, RetentionConfig,
		decode_checkpoint_meta, encode_checkpoint_meta, encode_checkpoints, encode_delta_meta,
	},
};
use universaldb::utils::IsolationLevel::Snapshot;
use uuid::Uuid;

const ACTOR: &str = "admin-short-ops-actor";

fn audit(namespace_id: Uuid) -> AuditFields {
	AuditFields {
		caller_id: "admin-user".to_string(),
		request_origin_ts_ms: 12_345,
		namespace_id,
	}
}

fn namespace_id_for_storage(namespace_id: Uuid) -> Id {
	Id::v1(namespace_id, 0)
}

async fn write_namespace_config(
	db: Arc<universaldb::Database>,
	namespace_id: Uuid,
	config: SqliteNamespaceConfig,
) -> Result<()> {
	db.run(move |tx| {
		let config = config.clone();

		async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			tx.write(
				&namespace::keys::sqlite_config_key(namespace_id_for_storage(namespace_id)),
				config,
			)?;
			Ok(())
		}
	})
	.await
}

async fn seed_checkpoints(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	entries: Vec<CheckpointEntry>,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		let entries = entries.clone();

		async move {
			for entry in &entries {
				let meta = CheckpointMeta {
					taken_at_ms: entry.taken_at_ms,
					head_txid: entry.ckp_txid,
					db_size_pages: 4,
					byte_count: entry.byte_count,
					refcount: entry.refcount,
					pinned_reason: None,
				};
				tx.informal().set(
					&checkpoint_meta_key(&actor_id, entry.ckp_txid),
					&encode_checkpoint_meta(meta)?,
				);
			}
			tx.informal().set(
				&meta_checkpoints_key(&actor_id),
				&encode_checkpoints(Checkpoints { entries })?,
			);
			Ok(())
		}
	})
	.await
}

async fn set_delta_refcount(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	txid: u64,
	refcount: u32,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let key = sqlite_storage::keys::delta_meta_key(&actor_id, txid);
			let bytes = tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.expect("delta meta should exist");
			let mut meta = sqlite_storage::types::decode_delta_meta(&bytes)?;
			meta.refcount = refcount;
			tx.informal().set(&key, &encode_delta_meta(meta)?);
			Ok(())
		}
	})
	.await
}

async fn set_checkpoint_refcount(
	db: Arc<universaldb::Database>,
	actor_id: &str,
	txid: u64,
	refcount: u32,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let key = checkpoint_meta_key(&actor_id, txid);
			let bytes = tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.expect("checkpoint meta should exist");
			let mut meta = decode_checkpoint_meta(&bytes)?;
			meta.refcount = refcount;
			tx.informal().set(&key, &encode_checkpoint_meta(meta)?);
			Ok(())
		}
	})
	.await
}

async fn create_record(
	db: Arc<universaldb::Database>,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: &str,
	namespace_id: Uuid,
) -> Result<()> {
	admin::create_record(
		db,
		op_id,
		op_kind,
		actor_id.to_string(),
		audit(namespace_id),
	)
	.await
}

async fn seed_actor_with_deltas(db: Arc<universaldb::Database>, actor_id: &str) -> Result<()> {
	for txid in 1..=5 {
		support::commit_pages(
			Arc::clone(&db),
			actor_id,
			vec![(txid as u32, txid as u8)],
			8,
			(txid as i64) * 1_000,
		)
		.await?;
	}
	Ok(())
}

#[tokio::test]
async fn describe_retention_basic() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-basic-").await?;
	let namespace_id = Uuid::new_v4();
	let namespace_config = support::namespace_config();
	write_namespace_config(Arc::clone(&db), namespace_id, namespace_config.clone()).await?;
	seed_actor_with_deltas(Arc::clone(&db), ACTOR).await?;
	seed_checkpoints(
		Arc::clone(&db),
		ACTOR,
		vec![
			CheckpointEntry {
				ckp_txid: 1,
				taken_at_ms: 1_100,
				byte_count: 100,
				refcount: 0,
			},
			CheckpointEntry {
				ckp_txid: 2,
				taken_at_ms: 2_100,
				byte_count: 200,
				refcount: 0,
			},
			CheckpointEntry {
				ckp_txid: 3,
				taken_at_ms: 3_100,
				byte_count: 300,
				refcount: 0,
			},
		],
	)
	.await?;
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::DescribeRetention, ACTOR, namespace_id).await?;

	let view = compactor::handle_describe_retention(
		Arc::clone(&db),
		op_id,
		DescribeRetentionRequest {
			actor_id: ACTOR.to_string(),
			audit: audit(namespace_id),
		},
	)
	.await?;

	assert_eq!(view.head.head_txid, 5);
	assert_eq!(view.checkpoints.len(), 3);
	assert_eq!(view.retention_config.retention_ms, namespace_config.default_retention_ms);
	assert!(view.storage_used_live_bytes > 0);
	assert_eq!(
		view.pitr_namespace_budget_bytes,
		namespace_config.pitr_namespace_budget_bytes
	);
	let window = view
		.fine_grained_window
		.expect("fine-grained window should exist");
	assert_eq!(window.from_txid, 4);
	assert_eq!(window.to_txid, 5);
	assert_eq!(window.from_taken_at_ms, 4_000);
	assert_eq!(window.to_taken_at_ms, 5_000);
	assert_eq!(window.delta_count, 2);
	assert!(window.total_bytes > 0);
	assert!(matches!(
		admin::read(Arc::clone(&db), op_id).await?.and_then(|record| record.result),
		Some(OpResult::RetentionView(_))
	));

	Ok(())
}

#[tokio::test]
async fn describe_retention_no_checkpoints() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-no-ckp-").await?;
	let namespace_id = Uuid::new_v4();
	write_namespace_config(Arc::clone(&db), namespace_id, support::namespace_config()).await?;
	seed_actor_with_deltas(Arc::clone(&db), ACTOR).await?;
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::DescribeRetention, ACTOR, namespace_id).await?;

	let view = compactor::handle_describe_retention(
		db,
		op_id,
		DescribeRetentionRequest {
			actor_id: ACTOR.to_string(),
			audit: audit(namespace_id),
		},
	)
	.await?;

	assert!(view.checkpoints.is_empty());
	assert_eq!(view.fine_grained_window, None);

	Ok(())
}

#[tokio::test]
async fn describe_retention_pinned_reason() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-pinned-").await?;
	let namespace_id = Uuid::new_v4();
	write_namespace_config(Arc::clone(&db), namespace_id, support::namespace_config()).await?;
	seed_actor_with_deltas(Arc::clone(&db), ACTOR).await?;
	seed_checkpoints(
		Arc::clone(&db),
		ACTOR,
		vec![CheckpointEntry {
			ckp_txid: 3,
			taken_at_ms: 3_100,
			byte_count: 300,
			refcount: 1,
		}],
	)
	.await?;
	set_checkpoint_refcount(Arc::clone(&db), ACTOR, 3, 1).await?;
	create_record(
		Arc::clone(&db),
		Uuid::new_v4(),
		OpKind::Fork,
		ACTOR,
		namespace_id,
	)
	.await?;
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::DescribeRetention, ACTOR, namespace_id).await?;

	let view = compactor::handle_describe_retention(
		db,
		op_id,
		DescribeRetentionRequest {
			actor_id: ACTOR.to_string(),
			audit: audit(namespace_id),
		},
	)
	.await?;

	assert_eq!(
		view.checkpoints[0].pinned_reason.as_deref(),
		Some("fork in progress")
	);

	Ok(())
}

#[tokio::test]
async fn set_retention_validates_max() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-retention-max-").await?;
	let namespace_id = Uuid::new_v4();
	let mut namespace_config = support::namespace_config();
	namespace_config.max_retention_ms = 10;
	write_namespace_config(Arc::clone(&db), namespace_id, namespace_config).await?;
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::SetRetention, ACTOR, namespace_id).await?;

	let err = compactor::handle_set_retention(
		Arc::clone(&db),
		op_id,
		SetRetentionRequest {
			actor_id: ACTOR.to_string(),
			config: RetentionConfig {
				retention_ms: 11,
				checkpoint_interval_ms: 1,
				max_checkpoints: 1,
			},
			audit: audit(namespace_id),
		},
	)
	.await
	.expect_err("retention above namespace max should fail");
	let extracted = RivetError::extract(&err);
	assert_eq!(extracted.group(), "sqlite_admin");
	assert_eq!(extracted.code(), "retention_window_exceeded");
	assert!(matches!(
		admin::read(db, op_id).await?.map(|record| record.status),
		Some(admin::OpStatus::Failed)
	));

	Ok(())
}

#[tokio::test]
async fn set_retention_persists() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-retention-set-").await?;
	let namespace_id = Uuid::new_v4();
	write_namespace_config(Arc::clone(&db), namespace_id, support::namespace_config()).await?;
	let config = RetentionConfig {
		retention_ms: 42_000,
		checkpoint_interval_ms: 2_000,
		max_checkpoints: 7,
	};
	let set_op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), set_op_id, OpKind::SetRetention, ACTOR, namespace_id).await?;

	compactor::handle_set_retention(
		Arc::clone(&db),
		set_op_id,
		SetRetentionRequest {
			actor_id: ACTOR.to_string(),
			config: config.clone(),
			audit: audit(namespace_id),
		},
	)
	.await?;

	let get_op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), get_op_id, OpKind::GetRetention, ACTOR, namespace_id).await?;
	let got = compactor::handle_get_retention(
		db,
		get_op_id,
		GetRetentionRequest {
			actor_id: ACTOR.to_string(),
			audit: audit(namespace_id),
		},
	)
	.await?;
	assert_eq!(got, config);

	Ok(())
}

#[tokio::test]
async fn clear_refcount_resets_to_zero() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-clear-").await?;
	let namespace_id = Uuid::new_v4();
	seed_actor_with_deltas(Arc::clone(&db), ACTOR).await?;
	set_delta_refcount(Arc::clone(&db), ACTOR, 2, 2).await?;
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::ClearRefcount, ACTOR, namespace_id).await?;

	compactor::handle_clear_refcount(
		Arc::clone(&db),
		op_id,
		ClearRefcountRequest {
			actor_id: ACTOR.to_string(),
			kind: RefcountKind::Delta,
			txid: 2,
			audit: audit(namespace_id),
		},
	)
	.await?;

	assert_eq!(support::read_delta_meta(db, ACTOR, 2).await?.refcount, 0);

	Ok(())
}

#[tokio::test]
async fn clear_refcount_invalid_txid() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-clear-missing-").await?;
	let namespace_id = Uuid::new_v4();
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::ClearRefcount, ACTOR, namespace_id).await?;

	let err = compactor::handle_clear_refcount(
		Arc::clone(&db),
		op_id,
		ClearRefcountRequest {
			actor_id: ACTOR.to_string(),
			kind: RefcountKind::Delta,
			txid: 99,
			audit: audit(namespace_id),
		},
	)
	.await
	.expect_err("missing txid should fail");
	assert_eq!(
		RivetError::extract(&err).code(),
		"invalid_restore_point"
	);

	Ok(())
}

#[tokio::test]
async fn clear_refcount_emits_audit() -> Result<()> {
	let db = support::test_db("sqlite-storage-admin-short-clear-audit-").await?;
	let namespace_id = Uuid::new_v4();
	let _ = compactor::admin::test_hooks::take_clear_refcount_audit_log();
	seed_actor_with_deltas(Arc::clone(&db), ACTOR).await?;
	set_delta_refcount(Arc::clone(&db), ACTOR, 2, 1).await?;
	let op_id = Uuid::new_v4();
	create_record(Arc::clone(&db), op_id, OpKind::ClearRefcount, ACTOR, namespace_id).await?;

	compactor::handle_clear_refcount(
		db,
		op_id,
		ClearRefcountRequest {
			actor_id: ACTOR.to_string(),
			kind: RefcountKind::Delta,
			txid: 2,
			audit: audit(namespace_id),
		},
	)
	.await?;
	let audit_entries = compactor::admin::test_hooks::take_clear_refcount_audit_log();

	assert!(audit_entries.iter().any(|entry| {
		entry.actor_id == ACTOR && entry.kind == RefcountKind::Delta && entry.txid == 2
	}));

	Ok(())
}
