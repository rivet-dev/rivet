use anyhow::{Context, Result};
use depot::fault::{
	ColdCompactionFaultPoint, ColdTierFaultPoint, CommitFaultPoint, DepotFaultPoint,
	DepotFaultReplayEvent, DepotFaultReplayEventKind, FaultBoundary, HotCompactionFaultPoint,
	ReadFaultPoint, ReclaimFaultPoint,
};
use depot::workflows::compaction::{CompactionJobKind, ForceCompactionWork};
use std::time::Duration;

use super::oracle::{AmbiguousOracleOutcome, OracleCommitSemantics};
use super::{FaultProfile, FaultReplayPhase, FaultScenario, LogicalOp};

#[test]
fn fault_scenario_runs_setup_workload_reload_and_verify() -> Result<()> {
	FaultScenario::new("scenario_harness_reload_smoke")
		.seed(42)
		.profile(FaultProfile::Simple)
		.setup(|ctx| async move {
			ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
				.await
		})
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::Read(ReadFaultPoint::BeforeReturnPages))
				.optional()
				.once()
				.delay(Duration::from_millis(1))?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "alpha".to_string(),
				value: vec![1, 2, 3],
			})
			.await?;
			ctx.checkpoint("after-alpha").await?;
			let result = ctx
				.force_compaction(ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: false,
					final_settle: true,
				})
				.await?;
			assert!(result.terminal_error.is_none());
			ctx.reload_database().await?;
			ctx.exec(LogicalOp::Put {
				key: "beta".to_string(),
				value: vec![4, 5, 6],
			})
			.await
		})
		.verify(|ctx| async move {
			ctx.verify_sqlite_integrity().await?;
			ctx.verify_against_native_oracle().await?;
			ctx.verify_depot_invariants().await?;
			let rows = ctx.query("SELECT k, hex(v) FROM kv ORDER BY k;").await?;
			assert_eq!(
				rows,
				vec![
					vec!["alpha".to_string(), "010203".to_string()],
					vec!["beta".to_string(), "040506".to_string()],
				]
			);
			let replay = ctx.replay_record().await;
			assert_eq!(replay.seed, 42);
			assert_eq!(replay.checkpoints, vec!["after-alpha".to_string()]);
			assert_eq!(replay.workload.len(), 2);
			assert!(replay.branch_head_before_faults.is_some());
			assert!(replay.branch_head_after_workload.is_some());
			assert_eq!(replay.oracle_result.as_deref(), Some("matched"));
			assert_faults(
				&replay.fault_events,
				&[(
					DepotFaultPoint::Read(ReadFaultPoint::BeforeReturnPages),
					FaultBoundary::ReadOnly,
					FaultReplayPhase::Workload,
				)],
			);
			Ok(())
		})
		.run()
}

#[test]
fn strict_reload_read_fault_returns_reload_error_instead_of_empty_database() -> Result<()> {
	FaultScenario::new("strict_reload_read_fault_returns_reload_error")
		.seed(43)
		.profile(FaultProfile::Simple)
		.setup(|ctx| async move {
			ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
				.await
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "alpha".to_string(),
				value: vec![1, 2, 3],
			})
			.await?;
			ctx.fault_controller()
				.at(DepotFaultPoint::Read(ReadFaultPoint::BeforeReturnPages))
				.once()
				.fail("strict reload read failure")?;
			let err = ctx
				.reload_database()
				.await
				.expect_err("strict reload should fail on injected depot read failure");
			let message = format!("{err:#}");
			assert!(message.contains("sqlite initial page fetch failed"));
			assert!(message.contains("strict reload read failure"));
			assert!(!message.contains("no such table"));
			Ok(())
		})
		.verify(|ctx| async move {
			let replay = ctx.replay_record().await;
			assert_eq!(replay.seed, 43);
			assert_eq!(replay.fault_events.len(), 1);
			let event = &replay.fault_events[0];
			assert_eq!(event.event.kind, DepotFaultReplayEventKind::Fired);
			assert_eq!(
				event.event.point,
				DepotFaultPoint::Read(ReadFaultPoint::BeforeReturnPages)
			);
			assert_eq!(event.event.boundary, FaultBoundary::ReadOnly);
			assert_eq!(event.phase, FaultReplayPhase::Workload);
			Ok(())
		})
		.run()
}

#[test]
fn simple_failed_commit_fault_rolls_back() -> Result<()> {
	FaultScenario::new("simple_failed_commit_fault_rolls_back")
		.seed(1_201)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::Commit(CommitFaultPoint::BeforeDeltaWrites))
				.once()
				.fail("simple failed commit")?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec_with_oracle_semantics(
				LogicalOp::Put {
					key: "failed".to_string(),
					value: vec![0xfa, 0x11],
				},
				OracleCommitSemantics::PreCommitFailure,
			)
			.await?;
			ctx.checkpoint("after-failed-commit").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, Vec::new()).await?;
			verify_simple_replay(
				&ctx,
				1_201,
				&["after-failed-commit"],
				&[(
					DepotFaultPoint::Commit(CommitFaultPoint::BeforeDeltaWrites),
					FaultBoundary::PreDurableCommit,
				)],
			)
			.await
		})
		.run()
}

#[test]
fn simple_ambiguous_post_commit_fault_classifies_durable_outcome() -> Result<()> {
	FaultScenario::new("simple_ambiguous_post_commit_fault_classifies_durable_outcome")
		.seed(1_202)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::Commit(CommitFaultPoint::AfterUdbCommit))
				.once()
				.fail("simple ambiguous commit")?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec_with_oracle_semantics(
				LogicalOp::Put {
					key: "ambiguous".to_string(),
					value: vec![0xab, 0xcd],
				},
				OracleCommitSemantics::AmbiguousPostCommit,
			)
			.await?;
			ctx.checkpoint("after-ambiguous-commit").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			ctx.verify_sqlite_integrity().await?;
			ctx.verify_against_native_oracle().await?;
			ctx.verify_depot_invariants().await?;
			let replay = ctx.replay_record().await;
			assert_eq!(replay.seed, 1_202);
			assert_eq!(replay.profile, FaultProfile::Simple);
			assert_eq!(
				replay.checkpoints,
				vec!["after-ambiguous-commit".to_string()]
			);
			assert_eq!(replay.workload.len(), 1);
			assert!(replay.branch_head_before_faults.is_some());
			assert!(replay.branch_head_after_workload.is_some());
			let outcome = replay
				.ambiguous_oracle_outcome
				.expect("ambiguous commit should record an oracle outcome");
			assert!(matches!(
				outcome,
				AmbiguousOracleOutcome::Old | AmbiguousOracleOutcome::New
			));
			let expected_oracle_result = format!("ambiguous:{}", outcome.as_str());
			assert_eq!(
				replay.oracle_result.as_deref(),
				Some(expected_oracle_result.as_str())
			);
			match outcome {
				AmbiguousOracleOutcome::Old => assert_kv_rows(&ctx, Vec::new()).await?,
				AmbiguousOracleOutcome::New => {
					assert_kv_rows(&ctx, vec![("ambiguous", "ABCD")]).await?
				}
				AmbiguousOracleOutcome::Invalid => {
					panic!("ambiguous verification should not classify valid state as invalid")
				}
			}
			assert_faults(
				&replay.fault_events,
				&[(
					DepotFaultPoint::Commit(CommitFaultPoint::AfterUdbCommit),
					FaultBoundary::AmbiguousAfterDurableCommit,
					FaultReplayPhase::Workload,
				)],
			);
			Ok(())
		})
		.run()
}

#[test]
fn simple_failed_hot_compaction_preserves_vfs_state() -> Result<()> {
	FaultScenario::new("simple_failed_hot_compaction_preserves_vfs_state")
		.seed(1_203)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::HotCompaction(
					HotCompactionFaultPoint::InstallAfterShardPublishBeforePidxClear,
				))
				.once()
				.fail("simple hot compaction failure")?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "hot".to_string(),
				value: vec![0x10, 0x20],
			})
			.await?;
			let result = ctx.force_hot_compaction().await?;
			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert!(
				result
					.terminal_error
					.as_deref()
					.is_some_and(|err| err.contains("simple hot compaction failure"))
			);
			ctx.checkpoint("after-failed-hot-compaction").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, vec![("hot", "1020")]).await?;
			verify_simple_replay(
				&ctx,
				1_203,
				&["after-failed-hot-compaction"],
				&[(
					DepotFaultPoint::HotCompaction(
						HotCompactionFaultPoint::InstallAfterShardPublishBeforePidxClear,
					),
					FaultBoundary::WorkflowOnly,
				)],
			)
			.await
		})
		.run()
}

#[test]
fn simple_failed_cold_publish_preserves_vfs_state() -> Result<()> {
	FaultScenario::new("simple_failed_cold_publish_preserves_vfs_state")
		.seed(1_204)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::ColdCompaction(
					ColdCompactionFaultPoint::PublishBeforeColdRefWrite,
				))
				.once()
				.fail("simple cold publish failure")?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "cold".to_string(),
				value: vec![0xca, 0xfe],
			})
			.await?;
			let restore_point = ctx.create_restore_point().await?;
			let result = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					cold: true,
					reclaim: false,
					final_settle: false,
				})
				.await?;
			assert!(
				result
					.attempted_job_kinds
					.contains(&CompactionJobKind::Cold)
			);
			assert!(
				result
					.terminal_error
					.as_deref()
					.is_some_and(|err| err.contains("simple cold publish failure"))
			);
			ctx.delete_restore_point(restore_point).await?;
			ctx.checkpoint("after-failed-cold-publish").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, vec![("cold", "CAFE")]).await?;
			verify_simple_replay(
				&ctx,
				1_204,
				&["after-failed-cold-publish"],
				&[(
					DepotFaultPoint::ColdCompaction(
						ColdCompactionFaultPoint::PublishBeforeColdRefWrite,
					),
					FaultBoundary::WorkflowOnly,
				)],
			)
			.await
		})
		.run()
}

#[test]
fn simple_workflow_cold_upload_uses_fault_controller_cold_tier() -> Result<()> {
	FaultScenario::new("simple_workflow_cold_upload_uses_fault_controller_cold_tier")
		.seed(1_207)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject))
				.once()
				.fail("workflow cold tier put failure")?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "workflow-cold-put".to_string(),
				value: vec![0x17],
			})
			.await?;
			let restore_point = ctx.create_restore_point().await?;
			let result = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					cold: true,
					reclaim: false,
					final_settle: false,
				})
				.await?;
			assert!(
				result
					.attempted_job_kinds
					.contains(&CompactionJobKind::Cold)
			);
			ctx.fault_controller().assert_expected_fired()?;
			assert_fault_points(
				&ctx.fault_controller().replay_log(),
				&[DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject)],
			);
			ctx.delete_restore_point(restore_point).await?;
			ctx.checkpoint("after-workflow-cold-put-fault").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, vec![("workflow-cold-put", "17")]).await?;
			verify_simple_replay(
				&ctx,
				1_207,
				&["after-workflow-cold-put-fault"],
				&[(
					DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject),
					FaultBoundary::WorkflowOnly,
				)],
			)
			.await
		})
		.run()
}

#[test]
fn simple_workflow_cold_object_missing_after_reclaim_recovers_on_reload() -> Result<()> {
	FaultScenario::new("simple_workflow_cold_object_missing_after_reclaim_recovers_on_reload")
		.seed(1_205)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::ColdCompaction(
					ColdCompactionFaultPoint::UploadAfterPutObject,
				))
				.once()
				.delay(Duration::from_millis(1))?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "cold-missing".to_string(),
				value: vec![0x01],
			})
			.await?;
			ctx.force_compaction(ForceCompactionWork {
				hot: true,
				cold: true,
				reclaim: false,
				final_settle: false,
			})
			.await?;
			let result = ctx
				.force_compaction(ForceCompactionWork {
					hot: false,
					cold: false,
					reclaim: true,
					final_settle: true,
				})
				.await?;
			assert_eq!(result.requested_work.reclaim, true);
			assert!(result.terminal_error.is_none());
			ctx.checkpoint("after-cold-reclaim").await?;
			ctx.fault_controller()
				.at(DepotFaultPoint::Read(ReadFaultPoint::AfterShardBlobLoad))
				.once()
				.drop_artifact()?;
			ctx.fault_controller()
				.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject))
				.once()
				.drop_artifact()?;
			let before_cold_gets = ctx.cold_gets();
			let mut saw_missing_cold_object = false;
			let mut read_errors = Vec::new();
			for pgno in 1..=4 {
				let read = ctx.read_page_from_depot(pgno).await;
				if let Err(err) = read.as_ref() {
					read_errors.push(format!("page {pgno}: {err:#}"));
				}
				if read
					.as_ref()
					.err()
					.is_some_and(|err| format!("{err:#}").contains("shard coverage is missing"))
				{
					saw_missing_cold_object = true;
					break;
				}
			}
			assert!(
				saw_missing_cold_object,
				"cold object fault should surface shard coverage missing, errors: {read_errors:?}"
			);
			assert!(ctx.cold_gets() > before_cold_gets);
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, vec![("cold-missing", "01")]).await?;
			verify_simple_replay(
				&ctx,
				1_205,
				&["after-cold-reclaim"],
				&[
					(
						DepotFaultPoint::ColdCompaction(
							ColdCompactionFaultPoint::UploadAfterPutObject,
						),
						FaultBoundary::WorkflowOnly,
					),
					(
						DepotFaultPoint::Read(ReadFaultPoint::AfterShardBlobLoad),
						FaultBoundary::ReadOnly,
					),
					(
						DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject),
						FaultBoundary::ReadOnly,
					),
				],
			)
			.await
		})
		.run()
}

#[test]
fn simple_harness_seeded_cold_ref_verifier_cold_get_fault_is_not_counted() -> Result<()> {
	FaultScenario::new("simple_harness_seeded_cold_ref_verifier_cold_get_fault_is_not_counted")
		.seed(1_208)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(|faults| {
			faults
				.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject))
				.optional()
				.once()
				.fail("verifier cold get should be isolated")?;
			Ok(())
		})
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "verifier-cold-get".to_string(),
				value: vec![0x88],
			})
			.await?;
			ctx.seed_page_as_cold_ref_for_harness_test(1).await?;
			ctx.checkpoint("after-verifier-cold-ref").await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, vec![("verifier-cold-get", "88")]).await?;
			verify_simple_replay(&ctx, 1_208, &["after-verifier-cold-ref"], &[]).await
		})
		.run()
}

#[test]
fn simple_forced_compaction_noops_report_all_requested_work() -> Result<()> {
	FaultScenario::new("simple_forced_compaction_noops_report_all_requested_work")
		.seed(1_206)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::Put {
				key: "noop".to_string(),
				value: vec![0x55],
			})
			.await?;
			let settle = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					cold: true,
					reclaim: true,
					final_settle: true,
				})
				.await?;
			assert!(settle.terminal_error.is_none());
			assert!(settle.attempted_job_kinds.contains(&CompactionJobKind::Hot));
			assert!(
				settle
					.attempted_job_kinds
					.contains(&CompactionJobKind::Cold)
			);
			let noop = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					cold: true,
					reclaim: true,
					final_settle: true,
				})
				.await?;
			assert!(noop.terminal_error.is_none());
			assert!(
				noop.skipped_noop_reasons
					.contains(&"hot:no-actionable-lag".to_string())
			);
			assert!(
				noop.skipped_noop_reasons
					.contains(&"cold:no-actionable-lag".to_string())
			);
			assert!(
				noop.attempted_job_kinds
					.contains(&CompactionJobKind::Reclaim)
					|| noop
						.skipped_noop_reasons
						.iter()
						.any(|reason| reason.starts_with("reclaim:"))
			);
			assert!(
				noop.skipped_noop_reasons
					.contains(&"final-settle:refreshed".to_string())
			);
			ctx.checkpoint("after-forced-noops").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_kv_rows(&ctx, vec![("noop", "55")]).await?;
			verify_simple_replay(&ctx, 1_206, &["after-forced-noops"], &[]).await
		})
		.run()
}

#[test]
fn simple_heavy_workload_crosses_delta_and_shard_boundaries() -> Result<()> {
	FaultScenario::new("simple_heavy_workload_crosses_delta_and_shard_boundaries")
		.seed(1_247)
		.profile(FaultProfile::Simple)
		.setup(configure_heavy_page_size)
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::CreateHeavySchema).await?;
			ctx.exec(LogicalOp::InsertHeavyBlob {
				id: 1,
				bucket: "delta".to_string(),
				payload: heavy_payload(0x31, 768 * 1024),
			})
			.await?;
			let delta_chunks = ctx.latest_delta_chunk_count().await?;
			assert!(
				delta_chunks > 1,
				"large blob commit should span multiple depot delta chunks, got {delta_chunks}"
			);

			for id in 2..=3 {
				ctx.exec(LogicalOp::InsertHeavyBlob {
					id,
					bucket: format!("bucket-{}", id % 2),
					payload: heavy_payload(id as u8, 64 * 1024),
				})
				.await?;
			}
			assert_shard_boundary_page_count(&ctx).await?;

			ctx.exec(LogicalOp::AddHeavyNoteColumn).await?;
			ctx.exec(LogicalOp::SetHeavyNote {
				id: 1,
				note: "schema-change-survived".to_string(),
			})
			.await?;
			ctx.exec(LogicalOp::ExplicitRollbackInsert {
				id: 9_000,
				payload_len: 4 * 1024,
			})
			.await?;
			assert_eq!(
				ctx.query("SELECT COUNT(*) FROM heavy_items WHERE id = 9000;")
					.await?,
				vec![vec!["0".to_string()]]
			);

			ctx.checkpoint("after-heavy-delta-shard").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_eq!(
				ctx.query(
					"SELECT COUNT(*), COUNT(note), \
						(SELECT note FROM heavy_items WHERE id = 1) \
					 FROM heavy_items;"
				)
				.await?,
				vec![vec![
					"3".to_string(),
					"1".to_string(),
					"schema-change-survived".to_string(),
				]]
			);
			verify_simple_replay(&ctx, 1_247, &["after-heavy-delta-shard"], &[]).await
		})
		.run()
}

#[test]
fn simple_heavy_workload_truncates_and_regrows_database() -> Result<()> {
	FaultScenario::new("simple_heavy_workload_truncates_and_regrows_database")
		.seed(1_248)
		.profile(FaultProfile::Simple)
		.setup(configure_heavy_page_size)
		.workload(|ctx| async move {
			ctx.exec(LogicalOp::CreateHeavySchema).await?;
			for id in 1..=3 {
				ctx.exec(LogicalOp::InsertHeavyBlob {
					id,
					bucket: "before-truncate".to_string(),
					payload: heavy_payload(0x70 + id as u8, 256 * 1024),
				})
				.await?;
			}
			let grown_pages = assert_shard_boundary_page_count(&ctx).await?;

			ctx.exec(LogicalOp::DeleteHeavyRange {
				min_id: 1,
				max_id: 3,
			})
			.await?;
			ctx.exec(LogicalOp::Vacuum).await?;
			let truncated_pages = page_count(&ctx).await?;
			assert!(
				truncated_pages < grown_pages,
				"VACUUM should truncate the database, before={grown_pages}, after={truncated_pages}"
			);

			for id in 10..=11 {
				ctx.exec(LogicalOp::InsertHeavyBlob {
					id,
					bucket: "after-regrow".to_string(),
					payload: heavy_payload(0x90 + id as u8, 320 * 1024),
				})
				.await?;
			}
			let regrown_pages = assert_shard_boundary_page_count(&ctx).await?;
			assert!(
				regrown_pages > truncated_pages,
				"regrow writes should increase page count, truncated={truncated_pages}, regrown={regrown_pages}"
			);

			ctx.checkpoint("after-heavy-truncate-regrow").await?;
			ctx.reload_database().await
		})
		.verify(|ctx| async move {
			assert_eq!(
				ctx.query("SELECT COUNT(*), MIN(id), MAX(id) FROM heavy_items;")
					.await?,
				vec![vec!["2".to_string(), "10".to_string(), "11".to_string()]]
			);
			verify_simple_replay(&ctx, 1_248, &["after-heavy-truncate-regrow"], &[]).await
		})
		.run()
}

#[test]
fn simple_high_risk_fault_matrix() -> Result<()> {
	for case in high_risk_fault_matrix_cases() {
		run_high_risk_fault_matrix_case(case)?;
	}
	Ok(())
}

#[derive(Clone)]
struct HighRiskFaultMatrixCase {
	name: &'static str,
	seed: u64,
	point: DepotFaultPoint,
	boundary: FaultBoundary,
	workload: HighRiskFaultMatrixWorkload,
	value: &'static [u8],
	expected_hex: &'static str,
}

#[derive(Clone, Copy)]
enum HighRiskFaultMatrixWorkload {
	CommitAmbiguous,
	CommitDurableError,
	HotCompaction,
	ColdCompaction,
	Reclaim,
}

fn high_risk_fault_matrix_cases() -> Vec<HighRiskFaultMatrixCase> {
	use HighRiskFaultMatrixWorkload::{
		ColdCompaction, CommitAmbiguous, CommitDurableError, HotCompaction, Reclaim,
	};

	vec![
		HighRiskFaultMatrixCase {
			name: "commit_after_udb_commit",
			seed: 1_230,
			point: DepotFaultPoint::Commit(CommitFaultPoint::AfterUdbCommit),
			boundary: FaultBoundary::AmbiguousAfterDurableCommit,
			workload: CommitAmbiguous,
			value: &[0xa0],
			expected_hex: "A0",
		},
		HighRiskFaultMatrixCase {
			name: "commit_before_compaction_signal",
			seed: 1_231,
			point: DepotFaultPoint::Commit(CommitFaultPoint::BeforeCompactionSignal),
			boundary: FaultBoundary::PostDurableNonData,
			workload: CommitDurableError,
			value: &[0xa1],
			expected_hex: "A1",
		},
		HighRiskFaultMatrixCase {
			name: "commit_after_compaction_signal",
			seed: 1_232,
			point: DepotFaultPoint::Commit(CommitFaultPoint::AfterCompactionSignal),
			boundary: FaultBoundary::PostDurableNonData,
			workload: CommitDurableError,
			value: &[0xa2],
			expected_hex: "A2",
		},
		HighRiskFaultMatrixCase {
			name: "hot_install_before_shard_publish",
			seed: 1_233,
			point: DepotFaultPoint::HotCompaction(
				HotCompactionFaultPoint::InstallBeforeShardPublish,
			),
			boundary: FaultBoundary::WorkflowOnly,
			workload: HotCompaction,
			value: &[0xb0],
			expected_hex: "B0",
		},
		HighRiskFaultMatrixCase {
			name: "hot_install_after_shard_publish_before_pidx_clear",
			seed: 1_234,
			point: DepotFaultPoint::HotCompaction(
				HotCompactionFaultPoint::InstallAfterShardPublishBeforePidxClear,
			),
			boundary: FaultBoundary::WorkflowOnly,
			workload: HotCompaction,
			value: &[0xb1],
			expected_hex: "B1",
		},
		HighRiskFaultMatrixCase {
			name: "hot_install_before_root_update",
			seed: 1_235,
			point: DepotFaultPoint::HotCompaction(HotCompactionFaultPoint::InstallBeforeRootUpdate),
			boundary: FaultBoundary::WorkflowOnly,
			workload: HotCompaction,
			value: &[0xb2],
			expected_hex: "B2",
		},
		HighRiskFaultMatrixCase {
			name: "hot_install_after_root_update",
			seed: 1_236,
			point: DepotFaultPoint::HotCompaction(HotCompactionFaultPoint::InstallAfterRootUpdate),
			boundary: FaultBoundary::WorkflowOnly,
			workload: HotCompaction,
			value: &[0xb3],
			expected_hex: "B3",
		},
		HighRiskFaultMatrixCase {
			name: "cold_upload_after_put_object",
			seed: 1_237,
			point: DepotFaultPoint::ColdCompaction(ColdCompactionFaultPoint::UploadAfterPutObject),
			boundary: FaultBoundary::WorkflowOnly,
			workload: ColdCompaction,
			value: &[0xc0],
			expected_hex: "C0",
		},
		HighRiskFaultMatrixCase {
			name: "cold_publish_after_cold_ref_write",
			seed: 1_238,
			point: DepotFaultPoint::ColdCompaction(
				ColdCompactionFaultPoint::PublishAfterColdRefWriteBeforeRootUpdate,
			),
			boundary: FaultBoundary::WorkflowOnly,
			workload: ColdCompaction,
			value: &[0xc1],
			expected_hex: "C1",
		},
		HighRiskFaultMatrixCase {
			name: "cold_publish_after_root_update",
			seed: 1_239,
			point: DepotFaultPoint::ColdCompaction(
				ColdCompactionFaultPoint::PublishAfterRootUpdate,
			),
			boundary: FaultBoundary::WorkflowOnly,
			workload: ColdCompaction,
			value: &[0xc2],
			expected_hex: "C2",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_before_hot_delete",
			seed: 1_240,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::BeforeHotDelete),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd0],
			expected_hex: "D0",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_after_hot_delete",
			seed: 1_241,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::AfterHotDelete),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd1],
			expected_hex: "D1",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_before_cold_retire",
			seed: 1_242,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::BeforeColdRetire),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd2],
			expected_hex: "D2",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_after_cold_retire",
			seed: 1_243,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::AfterColdRetire),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd3],
			expected_hex: "D3",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_before_cold_delete",
			seed: 1_244,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::BeforeColdDelete),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd4],
			expected_hex: "D4",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_after_cold_delete",
			seed: 1_245,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::AfterColdDelete),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd5],
			expected_hex: "D5",
		},
		HighRiskFaultMatrixCase {
			name: "reclaim_before_cleanup_rows",
			seed: 1_246,
			point: DepotFaultPoint::Reclaim(ReclaimFaultPoint::BeforeCleanupRows),
			boundary: FaultBoundary::WorkflowOnly,
			workload: Reclaim,
			value: &[0xd6],
			expected_hex: "D6",
		},
	]
}

fn run_high_risk_fault_matrix_case(case: HighRiskFaultMatrixCase) -> Result<()> {
	let scenario_name = format!("simple_high_risk_fault_matrix_{}", case.name);
	let checkpoint = format!("after-{}", case.name);
	let fault_message = format!("matrix fault {}", case.name);
	let fault_point = case.point.clone();
	let expected_point = case.point.clone();
	let expected_checkpoint = checkpoint.clone();
	let workload_case = case.clone();
	let verify_case = case;

	FaultScenario::new(scenario_name)
		.seed(workload_case.seed)
		.profile(FaultProfile::Simple)
		.setup(create_kv_table)
		.faults(move |faults| {
			faults.at(fault_point).once().fail(fault_message)?;
			Ok(())
		})
		.workload(move |ctx| {
			let checkpoint = checkpoint.clone();
			let case = workload_case.clone();
			let value = case.value.to_vec();
			async move {
				match case.workload {
					HighRiskFaultMatrixWorkload::CommitAmbiguous => {
						ctx.exec_with_oracle_semantics(
							LogicalOp::Put {
								key: case.name.to_string(),
								value,
							},
							OracleCommitSemantics::AmbiguousPostCommit,
						)
						.await?;
					}
					HighRiskFaultMatrixWorkload::CommitDurableError => {
						for warmup in 0..30 {
							ctx.exec(LogicalOp::Put {
								key: case.name.to_string(),
								value: vec![warmup],
							})
							.await?;
						}
						ctx.exec_with_durable_error(LogicalOp::Put {
							key: case.name.to_string(),
							value,
						})
						.await?;
					}
					HighRiskFaultMatrixWorkload::HotCompaction => {
						ctx.exec(LogicalOp::Put {
							key: case.name.to_string(),
							value,
						})
						.await?;
						let result = ctx.force_hot_compaction().await?;
						assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
						assert!(
							result.terminal_error.is_some(),
							"{} should report a terminal hot compaction error: {result:?}",
							case.name
						);
					}
					HighRiskFaultMatrixWorkload::ColdCompaction => {
						ctx.exec(LogicalOp::Put {
							key: case.name.to_string(),
							value,
						})
						.await?;
						let restore_point = ctx.create_restore_point().await?;
						let result = ctx
							.force_compaction(ForceCompactionWork {
								hot: true,
								cold: true,
								reclaim: false,
								final_settle: false,
							})
							.await?;
						assert!(
							result
								.attempted_job_kinds
								.contains(&CompactionJobKind::Cold)
						);
						assert!(
							result.terminal_error.is_some(),
							"{} should report a terminal cold compaction error: {result:?}",
							case.name
						);
						ctx.delete_restore_point(restore_point).await?;
					}
					HighRiskFaultMatrixWorkload::Reclaim => {
						let _grace_guard = ctx.override_cold_object_delete_grace(0).await?;
						ctx.exec(LogicalOp::Put {
							key: case.name.to_string(),
							value: vec![0xee],
						})
						.await?;
						let restore_point = ctx.create_restore_point().await?;
						let settle = ctx
							.force_compaction(ForceCompactionWork {
								hot: true,
								cold: true,
								reclaim: false,
								final_settle: false,
							})
							.await?;
						assert!(settle.terminal_error.is_none());
						ctx.delete_restore_point(restore_point).await?;
						ctx.exec(LogicalOp::Put {
							key: case.name.to_string(),
							value,
						})
						.await?;
						let result = ctx
							.force_compaction(ForceCompactionWork {
								hot: true,
								cold: true,
								reclaim: true,
								final_settle: false,
							})
							.await?;
						assert!(
							result
								.attempted_job_kinds
								.contains(&CompactionJobKind::Reclaim)
						);
					}
				}

				ctx.checkpoint(checkpoint).await?;
				ctx.reload_database().await
			}
		})
		.verify(move |ctx| async move {
			match verify_case.workload {
				HighRiskFaultMatrixWorkload::CommitAmbiguous => {
					verify_ambiguous_matrix_replay(
						&ctx,
						verify_case.seed,
						&expected_checkpoint,
						expected_point,
						verify_case.boundary,
						verify_case.name,
						verify_case.expected_hex,
					)
					.await
				}
				HighRiskFaultMatrixWorkload::CommitDurableError
				| HighRiskFaultMatrixWorkload::HotCompaction
				| HighRiskFaultMatrixWorkload::ColdCompaction
				| HighRiskFaultMatrixWorkload::Reclaim => {
					assert_kv_rows(&ctx, vec![(verify_case.name, verify_case.expected_hex)])
						.await?;
					verify_simple_replay(
						&ctx,
						verify_case.seed,
						&[expected_checkpoint.as_str()],
						&[(expected_point, verify_case.boundary)],
					)
					.await
				}
			}
		})
		.run()
}

fn create_kv_table(
	ctx: super::scenario::FaultScenarioCtx,
) -> impl std::future::Future<Output = Result<()>> {
	async move {
		ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
			.await
	}
}

fn configure_heavy_page_size(
	ctx: super::scenario::FaultScenarioCtx,
) -> impl std::future::Future<Output = Result<()>> {
	async move {
		ctx.sql(
			"PRAGMA page_size = 512; \
			 CREATE TABLE page_size_anchor (id INTEGER PRIMARY KEY); \
			 INSERT INTO page_size_anchor (id) VALUES (1);",
		)
		.await
	}
}

async fn assert_shard_boundary_page_count(ctx: &super::scenario::FaultScenarioCtx) -> Result<u32> {
	let pages = page_count(ctx).await?;
	let target = depot::keys::SHARD_SIZE * 2 + 2;
	assert!(
		pages >= target,
		"heavy workload should cover pages around shard boundaries 63/64/65 and 127/128/129; page_count={pages}, target={target}"
	);
	Ok(pages)
}

async fn page_count(ctx: &super::scenario::FaultScenarioCtx) -> Result<u32> {
	let rows = ctx.query("PRAGMA page_count;").await?;
	let value = rows
		.first()
		.and_then(|row| row.first())
		.context("PRAGMA page_count should return one row")?;
	value
		.parse::<u32>()
		.with_context(|| format!("page_count should be an integer: {value}"))
}

fn heavy_payload(seed: u8, len: usize) -> Vec<u8> {
	let mut state = u64::from(seed).wrapping_mul(0x9E37_79B9_7F4A_7C15);
	(0..len)
		.map(|_| {
			state ^= state << 13;
			state ^= state >> 7;
			state ^= state << 17;
			state as u8
		})
		.collect()
}

async fn assert_kv_rows(
	ctx: &super::scenario::FaultScenarioCtx,
	expected: Vec<(&str, &str)>,
) -> Result<()> {
	let expected = expected
		.into_iter()
		.map(|(key, value)| vec![key.to_string(), value.to_string()])
		.collect::<Vec<_>>();
	assert_eq!(
		ctx.query("SELECT k, hex(v) FROM kv ORDER BY k;").await?,
		expected
	);
	Ok(())
}

async fn verify_ambiguous_matrix_replay(
	ctx: &super::scenario::FaultScenarioCtx,
	seed: u64,
	checkpoint: &str,
	expected_point: DepotFaultPoint,
	expected_boundary: FaultBoundary,
	key: &str,
	expected_hex: &str,
) -> Result<()> {
	ctx.verify_sqlite_integrity().await?;
	ctx.verify_against_native_oracle().await?;
	ctx.verify_depot_invariants().await?;
	let replay = ctx.replay_record().await;
	assert_eq!(replay.seed, seed);
	assert_eq!(replay.profile, FaultProfile::Simple);
	assert_eq!(replay.checkpoints, vec![checkpoint.to_string()]);
	assert_eq!(replay.workload.len(), 1);
	assert!(replay.branch_head_before_faults.is_some());
	assert!(replay.branch_head_after_workload.is_some());
	let outcome = replay
		.ambiguous_oracle_outcome
		.expect("ambiguous matrix case should record an oracle outcome");
	assert!(matches!(
		outcome,
		AmbiguousOracleOutcome::Old | AmbiguousOracleOutcome::New
	));
	let expected_oracle_result = format!("ambiguous:{}", outcome.as_str());
	assert_eq!(
		replay.oracle_result.as_deref(),
		Some(expected_oracle_result.as_str())
	);
	match outcome {
		AmbiguousOracleOutcome::Old => assert_kv_rows(ctx, Vec::new()).await?,
		AmbiguousOracleOutcome::New => assert_kv_rows(ctx, vec![(key, expected_hex)]).await?,
		AmbiguousOracleOutcome::Invalid => {
			panic!("ambiguous matrix case should not classify valid state as invalid")
		}
	}
	assert_faults(
		&replay.fault_events,
		&[(
			expected_point,
			expected_boundary,
			FaultReplayPhase::Workload,
		)],
	);
	Ok(())
}

async fn verify_simple_replay(
	ctx: &super::scenario::FaultScenarioCtx,
	seed: u64,
	checkpoints: &[&str],
	expected_faults: &[(DepotFaultPoint, FaultBoundary)],
) -> Result<()> {
	ctx.verify_sqlite_integrity().await?;
	ctx.verify_against_native_oracle().await?;
	ctx.verify_depot_invariants().await?;
	let replay = ctx.replay_record().await;
	assert_eq!(replay.seed, seed);
	assert_eq!(replay.profile, FaultProfile::Simple);
	assert_eq!(
		replay.checkpoints,
		checkpoints
			.iter()
			.map(|checkpoint| checkpoint.to_string())
			.collect::<Vec<_>>()
	);
	assert!(!replay.workload.is_empty());
	assert!(replay.branch_head_before_faults.is_some());
	assert!(replay.branch_head_after_workload.is_some());
	assert_eq!(replay.oracle_result.as_deref(), Some("matched"));
	assert_eq!(replay.ambiguous_oracle_outcome, None);
	let expected_faults = expected_faults
		.iter()
		.cloned()
		.map(|(point, boundary)| (point, boundary, FaultReplayPhase::Workload))
		.collect::<Vec<_>>();
	assert_faults(&replay.fault_events, &expected_faults);
	Ok(())
}

fn assert_faults(
	events: &[super::scenario::FaultScenarioReplayEvent],
	expected: &[(DepotFaultPoint, FaultBoundary, FaultReplayPhase)],
) {
	assert_eq!(events.len(), expected.len());
	for (event, (point, boundary, phase)) in events.iter().zip(expected) {
		assert_eq!(event.event.kind, DepotFaultReplayEventKind::Fired);
		assert_eq!(event.event.point, *point);
		assert_eq!(event.event.boundary, *boundary);
		assert_eq!(event.phase, *phase);
	}
}

fn assert_fault_points(events: &[DepotFaultReplayEvent], points: &[DepotFaultPoint]) {
	assert_eq!(events.len(), points.len());
	for (event, point) in events.iter().zip(points) {
		assert_eq!(event.kind, DepotFaultReplayEventKind::Fired);
		assert_eq!(event.point, *point);
	}
}
