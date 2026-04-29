use std::collections::BTreeSet;

use anyhow::Result;
use rivet_pools::NodeId;
use sqlite_storage::{
	admin::types::{
		AdminOpRecord, AuditFields, OpKind, OpProgress, OpResult, OpStatus,
		decode_admin_op_record, encode_admin_op_record,
	},
	pump::{
		keys::{
			actor_prefix, checkpoint_meta_key, checkpoint_pidx_delta_key, checkpoint_prefix,
			checkpoint_shard_key, delta_meta_key, meta_admin_op_key, meta_checkpoints_key,
			meta_fork_in_progress_key, meta_restore_in_progress_key, meta_retention_key,
			meta_storage_used_live_key, meta_storage_used_pitr_key,
		},
		types::{
			ForkMarker, ForkStep, RestoreMarker, RestoreStep, RetentionConfig,
			decode_fork_marker, decode_restore_marker, decode_retention_config,
			encode_fork_marker, encode_restore_marker, encode_retention_config,
		},
	},
};
use uuid::Uuid;

const TEST_ACTOR: &str = "pitr-actor";

#[test]
fn keys_unique() {
	let op_id = Uuid::new_v4();
	let actor_prefix = actor_prefix(TEST_ACTOR);
	let keys = vec![
		meta_retention_key(TEST_ACTOR),
		meta_checkpoints_key(TEST_ACTOR),
		meta_storage_used_live_key(TEST_ACTOR),
		meta_storage_used_pitr_key(TEST_ACTOR),
		meta_admin_op_key(TEST_ACTOR, op_id),
		meta_restore_in_progress_key(TEST_ACTOR),
		meta_fork_in_progress_key(TEST_ACTOR),
		checkpoint_prefix(TEST_ACTOR, 9),
		checkpoint_meta_key(TEST_ACTOR, 9),
		checkpoint_shard_key(TEST_ACTOR, 9, 3),
		checkpoint_pidx_delta_key(TEST_ACTOR, 9, 17),
		delta_meta_key(TEST_ACTOR, 12),
	];

	for key in &keys {
		assert!(key.starts_with(&actor_prefix));
	}

	let unique = keys.iter().collect::<BTreeSet<_>>();
	assert_eq!(unique.len(), keys.len());
}

#[test]
fn retention_config_vbare_roundtrip() -> Result<()> {
	let default_config = RetentionConfig::default();
	assert_eq!(default_config.retention_ms, 0);
	assert_eq!(default_config.checkpoint_interval_ms, 3_600_000);
	assert_eq!(default_config.max_checkpoints, 25);

	let config = RetentionConfig {
		retention_ms: 86_400_000,
		checkpoint_interval_ms: 600_000,
		max_checkpoints: 12,
	};
	let decoded = decode_retention_config(&encode_retention_config(config.clone())?)?;

	assert_eq!(decoded, config);
	Ok(())
}

#[test]
fn restore_marker_vbare_roundtrip() -> Result<()> {
	for step in [
		RestoreStep::Started,
		RestoreStep::CheckpointCopied,
		RestoreStep::DeltasReplayed,
		RestoreStep::MetaWritten,
	] {
		let marker = RestoreMarker {
			target_txid: 42,
			ckp_txid: 24,
			started_at_ms: 1_900,
			last_completed_step: step,
			holder_id: NodeId::from(Uuid::new_v4()),
			op_id: Uuid::new_v4(),
		};
		let decoded = decode_restore_marker(&encode_restore_marker(marker.clone())?)?;
		assert_eq!(decoded, marker);
	}

	Ok(())
}

#[test]
fn fork_marker_vbare_roundtrip() -> Result<()> {
	for step in [
		ForkStep::Started,
		ForkStep::CheckpointCopied,
		ForkStep::DeltasReplayed,
		ForkStep::MetaWritten,
	] {
		let marker = ForkMarker {
			src_actor_id: "src-actor".to_string(),
			ckp_txid: 24,
			target_txid: 42,
			started_at_ms: 1_900,
			last_completed_step: step,
			holder_id: NodeId::from(Uuid::new_v4()),
			op_id: Uuid::new_v4(),
		};
		let decoded = decode_fork_marker(&encode_fork_marker(marker.clone())?)?;
		assert_eq!(decoded, marker);
	}

	Ok(())
}

#[test]
fn admin_op_record_vbare_roundtrip() -> Result<()> {
	for status in [
		OpStatus::Pending,
		OpStatus::InProgress,
		OpStatus::Completed,
		OpStatus::Failed,
		OpStatus::Orphaned,
	] {
		let record = AdminOpRecord {
			operation_id: Uuid::new_v4(),
			op_kind: OpKind::Restore,
			actor_id: TEST_ACTOR.to_string(),
			created_at_ms: 1_000,
			last_progress_at_ms: 1_500,
			status,
			holder_id: Some(NodeId::from(Uuid::new_v4())),
			progress: Some(OpProgress {
				step: "copy checkpoint".to_string(),
				bytes_done: 128,
				bytes_total: 256,
				started_at_ms: 1_050,
				eta_ms: Some(2_000),
				current_tx_index: 1,
				total_tx_count: 3,
			}),
			result: Some(OpResult::Message {
				message: "ok".to_string(),
			}),
			audit: AuditFields {
				caller_id: "user-1".to_string(),
				request_origin_ts_ms: 999,
				namespace_id: Uuid::new_v4(),
			},
		};
		let decoded = decode_admin_op_record(&encode_admin_op_record(record.clone())?)?;
		assert_eq!(decoded, record);
	}

	Ok(())
}
