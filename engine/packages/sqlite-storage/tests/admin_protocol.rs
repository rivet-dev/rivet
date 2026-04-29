use std::sync::Arc;

use anyhow::Result;
use rivet_error::RivetError;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::{
		self, AdminOpRecord, AuditFields, ForkDstSpec, ForkMode, OpKind, OpProgress, OpResult,
		OpStatus, RefcountKind, RestoreMode, RestoreTarget, SQLITE_OP_SUBJECT, SqliteAdminError,
		SqliteOp, SqliteOpRequest, SqliteOpSubject, decode_sqlite_op_request,
		encode_sqlite_op_request,
	},
	pump::types::RetentionConfig,
};
use tempfile::Builder;
use universalpubsub::Subject;
use uuid::Uuid;

const TEST_ACTOR: &str = "admin-protocol-actor";

async fn test_db() -> Result<Arc<universaldb::Database>> {
	let path = Builder::new()
		.prefix("sqlite-storage-admin-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(Arc::new(universaldb::Database::new(Arc::new(driver))))
}

fn audit() -> AuditFields {
	AuditFields {
		caller_id: "user-1".to_string(),
		request_origin_ts_ms: 1_000,
		namespace_id: Uuid::new_v4(),
	}
}

fn roundtrip_request(op: SqliteOp) -> Result<SqliteOpRequest> {
	let request = SqliteOpRequest {
		request_id: Uuid::new_v4(),
		op,
		audit: audit(),
	};
	decode_sqlite_op_request(&encode_sqlite_op_request(request)?)
}

#[test]
fn op_subject_matches_spec() {
	assert_eq!(SQLITE_OP_SUBJECT, "sqlite.op");
	assert_eq!(SqliteOpSubject.to_string(), "sqlite.op");
	assert_eq!(SqliteOpSubject.as_str(), Some("sqlite.op"));
}

#[test]
fn op_request_vbare_roundtrip() -> Result<()> {
	let namespace_id = Uuid::new_v4();
	let ops = vec![
		SqliteOp::Restore {
			actor_id: TEST_ACTOR.to_string(),
			target: RestoreTarget::Txid(7),
			mode: RestoreMode::Apply,
		},
		SqliteOp::Fork {
			src_actor_id: TEST_ACTOR.to_string(),
			target: RestoreTarget::LatestCheckpoint,
			mode: ForkMode::DryRun,
			dst: ForkDstSpec::Allocate {
				dst_namespace_id: namespace_id,
			},
		},
		SqliteOp::DescribeRetention {
			actor_id: TEST_ACTOR.to_string(),
		},
		SqliteOp::SetRetention {
			actor_id: TEST_ACTOR.to_string(),
			config: RetentionConfig {
				retention_ms: 86_400_000,
				checkpoint_interval_ms: 3_600_000,
				max_checkpoints: 25,
			},
		},
		SqliteOp::ClearRefcount {
			actor_id: TEST_ACTOR.to_string(),
			kind: RefcountKind::Delta,
			txid: 42,
		},
	];

	for op in ops {
		let decoded = roundtrip_request(op.clone())?;
		assert_eq!(decoded.op, op);
	}

	Ok(())
}

#[test]
fn restore_target_variants_roundtrip() -> Result<()> {
	for target in [
		RestoreTarget::Txid(7),
		RestoreTarget::TimestampMs(1_700_000),
		RestoreTarget::LatestCheckpoint,
		RestoreTarget::CheckpointTxid(5),
	] {
		let decoded = roundtrip_request(SqliteOp::Restore {
			actor_id: TEST_ACTOR.to_string(),
			target,
			mode: RestoreMode::DryRun,
		})?;
		assert!(matches!(
			decoded.op,
			SqliteOp::Restore {
				target: decoded_target,
				..
			} if decoded_target == target
		));
	}

	Ok(())
}

#[test]
fn fork_dst_spec_variants_roundtrip() -> Result<()> {
	let specs = vec![
		ForkDstSpec::Allocate {
			dst_namespace_id: Uuid::new_v4(),
		},
		ForkDstSpec::Existing {
			dst_actor_id: "dst-actor".to_string(),
		},
	];

	for dst in specs {
		let decoded = roundtrip_request(SqliteOp::Fork {
			src_actor_id: TEST_ACTOR.to_string(),
			target: RestoreTarget::Txid(9),
			mode: ForkMode::Apply,
			dst: dst.clone(),
		})?;
		assert!(matches!(
			decoded.op,
			SqliteOp::Fork {
				dst: decoded_dst,
				..
			} if decoded_dst == dst
		));
	}

	Ok(())
}

#[test]
fn every_admin_error_variant_round_trips_through_rivet_error() {
	let operation_id = Uuid::new_v4();
	let existing_operation_id = Uuid::new_v4();
	let errors = vec![
		(
			SqliteAdminError::InvalidRestorePoint {
				target_txid: 50,
				reachable_hints: vec![1, 2],
			},
			"invalid_restore_point",
			"the requested target is not within the retention window or has had its DELTAs cleaned up",
		),
		(
			SqliteAdminError::ForkDestinationAlreadyExists {
				dst_actor_id: "dst".to_string(),
			},
			"fork_destination_exists",
			"the destination actor already has SQLite state",
		),
		(
			SqliteAdminError::PitrDisabledForNamespace,
			"pitr_disabled_for_namespace",
			"PITR is not enabled for this namespace",
		),
		(
			SqliteAdminError::PitrDestructiveDisabledForNamespace,
			"pitr_destructive_disabled_for_namespace",
			"destructive PITR (Apply mode restore) is not enabled for this namespace",
		),
		(
			SqliteAdminError::RetentionWindowExceeded {
				oldest_reachable_txid: 5,
			},
			"retention_window_exceeded",
			"target predates the retention window",
		),
		(
			SqliteAdminError::RestoreInProgress {
				existing_operation_id,
			},
			"restore_in_progress",
			"a restore operation is already running on this actor",
		),
		(
			SqliteAdminError::ForkInProgress {
				existing_operation_id,
			},
			"fork_in_progress",
			"a fork operation is already targeting this destination actor",
		),
		(
			SqliteAdminError::ActorRestoreInProgress,
			"actor_restore_in_progress",
			"the actor is being restored; commits are temporarily blocked",
		),
		(
			SqliteAdminError::AdminOpRateLimited {
				retry_after_ms: 500,
			},
			"admin_op_rate_limited",
			"too many concurrent admin operations for this namespace",
		),
		(
			SqliteAdminError::PitrNamespaceBudgetExceeded {
				used_bytes: 10,
				budget_bytes: 9,
			},
			"pitr_namespace_budget_exceeded",
			"creating this checkpoint would exceed the namespace PITR budget",
		),
		(
			SqliteAdminError::OperationOrphaned { operation_id },
			"operation_orphaned",
			"operation has been pending without a working pod for too long; please retry",
		),
	];

	for (err, code, message) in errors {
		let err = err.build();
		let rivet_error = RivetError::extract(&err);
		assert_eq!(rivet_error.group(), "sqlite_admin");
		assert_eq!(rivet_error.code(), code);
		assert_eq!(rivet_error.message(), message);
	}
}

#[tokio::test]
async fn record_create_then_read() -> Result<()> {
	let db = test_db().await?;
	let op_id = Uuid::new_v4();
	let audit = audit();

	admin::create_record(
		Arc::clone(&db),
		op_id,
		OpKind::Restore,
		TEST_ACTOR.to_string(),
		audit.clone(),
	)
	.await?;
	let record = admin::read(Arc::clone(&db), op_id)
		.await?
		.expect("record should exist");

	assert_eq!(
		record,
		AdminOpRecord {
			operation_id: op_id,
			op_kind: OpKind::Restore,
			actor_id: TEST_ACTOR.to_string(),
			created_at_ms: record.created_at_ms,
			last_progress_at_ms: record.created_at_ms,
			status: OpStatus::Pending,
			holder_id: None,
			progress: None,
			result: None,
			audit,
		}
	);

	Ok(())
}

#[tokio::test]
async fn record_status_transitions() -> Result<()> {
	let db = test_db().await?;
	let op_id = Uuid::new_v4();
	let holder = NodeId::new();

	admin::create_record(
		Arc::clone(&db),
		op_id,
		OpKind::Restore,
		TEST_ACTOR.to_string(),
		audit(),
	)
	.await?;
	admin::update_status(Arc::clone(&db), op_id, OpStatus::InProgress, Some(holder)).await?;
	admin::complete(
		Arc::clone(&db),
		op_id,
		OpResult::Message {
			message: "done".to_string(),
		},
	)
	.await?;

	let completed = admin::read(Arc::clone(&db), op_id)
		.await?
		.expect("record should exist");
	assert_eq!(completed.status, OpStatus::Completed);
	assert_eq!(completed.holder_id, None);
	assert_eq!(
		completed.result,
		Some(OpResult::Message {
			message: "done".to_string(),
		})
	);

	let op_id = Uuid::new_v4();
	admin::create_record(
		Arc::clone(&db),
		op_id,
		OpKind::Restore,
		TEST_ACTOR.to_string(),
		audit(),
	)
	.await?;
	admin::update_status(Arc::clone(&db), op_id, OpStatus::InProgress, Some(holder)).await?;
	let err = admin::update_status(Arc::clone(&db), op_id, OpStatus::Pending, None)
		.await
		.expect_err("backwards transition should fail");
	assert!(format!("{err:?}").contains("invalid sqlite admin op status transition"));

	Ok(())
}

#[tokio::test]
async fn record_progress_updates() -> Result<()> {
	let db = test_db().await?;
	let op_id = Uuid::new_v4();

	admin::create_record(
		Arc::clone(&db),
		op_id,
		OpKind::Fork,
		TEST_ACTOR.to_string(),
		audit(),
	)
	.await?;
	let before = admin::read(Arc::clone(&db), op_id)
		.await?
		.expect("record should exist");

	admin::update_progress(
		Arc::clone(&db),
		op_id,
		OpProgress {
			step: "copy checkpoint".to_string(),
			bytes_done: 64,
			bytes_total: 128,
			started_at_ms: 1_000,
			eta_ms: Some(2_000),
			current_tx_index: 1,
			total_tx_count: 2,
		},
	)
	.await?;
	let after = admin::read(Arc::clone(&db), op_id)
		.await?
		.expect("record should exist");

	assert_eq!(after.operation_id, before.operation_id);
	assert!(after.last_progress_at_ms > before.last_progress_at_ms);
	assert_eq!(
		after.progress,
		Some(OpProgress {
			step: "copy checkpoint".to_string(),
			bytes_done: 64,
			bytes_total: 128,
			started_at_ms: 1_000,
			eta_ms: Some(2_000),
			current_tx_index: 1,
			total_tx_count: 2,
		})
	);

	Ok(())
}
