use anyhow::{Context, Result};
use depot::fault::{
	CommitFaultPoint, DepotFaultPoint, DepotFaultReplayEventKind, FaultBoundary,
	HotCompactionFaultPoint, ReadFaultPoint, ReclaimFaultPoint,
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
					reclaim: true,
					final_settle: true,
				})
				.await?;
			assert!(settle.terminal_error.is_none());
			assert!(settle.attempted_job_kinds.contains(&CompactionJobKind::Hot));
			let noop = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
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
fn simple_thread_actor_schema_survives_forced_compaction_reload() -> Result<()> {
	FaultScenario::new("simple_thread_actor_schema_survives_forced_compaction_reload")
		.seed(1_249)
		.profile(FaultProfile::Simple)
		.setup(create_thread_actor_schema)
		.workload(|ctx| async move {
			for cycle in 0..24 {
				thread_actor_write_cycle(&ctx, cycle, 128 * 1024).await?;
				if cycle % 6 == 5 {
					thread_actor_assert_reads(&ctx, "pre-compaction").await?;
					ctx.reload_database().await?;
				}
			}
			thread_actor_assert_reads(&ctx, "before-hot").await?;
			let before_pages = page_count(&ctx).await?;
			assert!(
				before_pages > depot::keys::SHARD_SIZE,
				"thread-actor-shaped workload should cross at least one depot shard, page_count={before_pages}"
			);

			let restore_point = ctx.create_restore_point().await?;
			let hot = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					reclaim: false,
					final_settle: true,
				})
				.await?;
			assert!(
				hot.terminal_error.is_none(),
				"forced hot compaction should succeed: {hot:?}"
			);
			ctx.reload_database().await?;
			thread_actor_assert_reads(&ctx, "after-hot-reload").await?;

			for cycle in 24..32 {
				thread_actor_write_cycle(&ctx, cycle, 128 * 1024).await?;
			}
			ctx.delete_restore_point(restore_point).await?;
			let reclaim = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					reclaim: true,
					final_settle: true,
				})
				.await?;
			assert!(
				reclaim.terminal_error.is_none(),
				"forced reclaim compaction should succeed: {reclaim:?}"
			);
			ctx.checkpoint("after-thread-actor-hot-reclaim").await?;
			ctx.reload_database().await?;
			thread_actor_assert_reads(&ctx, "after-reclaim-reload").await
		})
		.verify(|ctx| async move {
			thread_actor_assert_reads(&ctx, "verify").await?;
			verify_simple_replay(&ctx, 1_249, &["after-thread-actor-hot-reclaim"], &[]).await
		})
		.run()
}

#[test]
#[ignore = "focused investigation for hot compaction input cap corruption"]
fn simple_hot_compaction_window_cap_reopens_cleanly() -> Result<()> {
	FaultScenario::new("simple_hot_compaction_window_cap_reopens_cleanly")
		.seed(1_250)
		.profile(FaultProfile::Simple)
		.setup(create_hot_cap_schema)
		.workload(|ctx| async move {
			for cycle in 0..540 {
				hot_cap_write_cycle(&ctx, cycle).await?;
			}
			hot_cap_assert_reads(&ctx, "before-hot").await?;

			let result = ctx
				.force_compaction(ForceCompactionWork {
					hot: true,
					reclaim: false,
					final_settle: true,
				})
				.await?;
			assert_eq!(result.attempted_job_kinds, vec![CompactionJobKind::Hot]);
			assert!(
				result.terminal_error.is_none(),
				"forced hot compaction should succeed: {result:?}"
			);

			ctx.checkpoint("after-window-cap-hot-compaction").await?;
			ctx.reload_database().await?;
			hot_cap_assert_reads(&ctx, "after-hot-reload").await?;

			hot_cap_write_cycle(&ctx, 540).await?;
			ctx.reload_database().await?;
			hot_cap_assert_reads(&ctx, "after-post-hot-commit-reload").await
		})
		.verify(|ctx| async move {
			hot_cap_assert_reads(&ctx, "verify").await?;
			verify_simple_replay(&ctx, 1_250, &["after-window-cap-hot-compaction"], &[]).await
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
	Reclaim,
}

fn high_risk_fault_matrix_cases() -> Vec<HighRiskFaultMatrixCase> {
	use HighRiskFaultMatrixWorkload::{
		CommitAmbiguous, CommitDurableError, HotCompaction, Reclaim,
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
					HighRiskFaultMatrixWorkload::Reclaim => {
						ctx.exec(LogicalOp::Put {
							key: case.name.to_string(),
							value: vec![0xee],
						})
						.await?;
						let restore_point = ctx.create_restore_point().await?;
						let settle = ctx
							.force_compaction(ForceCompactionWork {
								hot: true,
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

fn create_thread_actor_schema(
	ctx: super::scenario::FaultScenarioCtx,
) -> impl std::future::Future<Output = Result<()>> {
	async move {
		ctx.sql(
			"CREATE TABLE compaction_summaries (
				summary_id TEXT PRIMARY KEY,
				summary_text TEXT NOT NULL,
				cut_message_id TEXT NOT NULL,
				created_at TEXT NOT NULL
			);
			CREATE INDEX idx_compaction_summaries_created_at
				ON compaction_summaries(created_at);
			CREATE TABLE tool_calls (
				call_id TEXT PRIMARY KEY,
				message_id TEXT NOT NULL,
				state TEXT NOT NULL,
				created_at TEXT NOT NULL,
				payload TEXT NOT NULL
			);
			CREATE INDEX idx_tool_calls_state ON tool_calls(state);
			CREATE TABLE messages (
				message_id TEXT PRIMARY KEY,
				body TEXT NOT NULL,
				created_at TEXT NOT NULL
			);
			CREATE TABLE thread_events (
				event_id INTEGER PRIMARY KEY AUTOINCREMENT,
				event_type TEXT NOT NULL,
				message_id TEXT,
				created_at TEXT NOT NULL
			);",
		)
		.await
	}
}

fn create_hot_cap_schema(
	ctx: super::scenario::FaultScenarioCtx,
) -> impl std::future::Future<Output = Result<()>> {
	async move {
		ctx.sql(
			"CREATE TABLE hot_cap (
				id INTEGER PRIMARY KEY,
				bucket TEXT NOT NULL,
				payload BLOB NOT NULL,
				updated_at TEXT NOT NULL
			);
			CREATE INDEX hot_cap_bucket_idx ON hot_cap(bucket, id);",
		)
		.await
	}
}

async fn hot_cap_write_cycle(ctx: &super::scenario::FaultScenarioCtx, cycle: usize) -> Result<()> {
	let id = cycle + 1;
	ctx.exec(LogicalOp::Sql(format!(
		"INSERT INTO hot_cap (id, bucket, payload, updated_at)
		 VALUES ({id}, 'b{}', zeroblob(3072), '2026-05-16T08:{:02}:{:02}.{:03}Z')
		 ON CONFLICT(id) DO UPDATE SET
			bucket = excluded.bucket,
			payload = excluded.payload,
			updated_at = excluded.updated_at;",
		id % 11,
		cycle % 60,
		(cycle * 7) % 60,
		(cycle * 13) % 1000,
	)))
	.await
}

async fn hot_cap_assert_reads(ctx: &super::scenario::FaultScenarioCtx, phase: &str) -> Result<()> {
	let snapshot = hot_cap_snapshot(ctx, phase).await;
	ctx.verify_sqlite_integrity_rows()
		.await
		.with_context(|| format!("hot cap sqlite integrity failed during {phase}: {snapshot}"))?;
	let row_count = ctx
		.query("SELECT count(*) FROM hot_cap;")
		.await
		.with_context(|| format!("hot cap count failed during {phase}"))?;
	assert_eq!(
		row_count,
		vec![vec![
			if phase == "after-post-hot-commit-reload" || phase == "verify" {
				"541".to_string()
			} else {
				"540".to_string()
			}
		]],
		"hot cap row count mismatch during {phase}: {row_count:?}"
	);
	let indexed = ctx
		.query(
			"SELECT id, length(payload)
			 FROM hot_cap
			 WHERE bucket = 'b3'
			 ORDER BY id DESC
			 LIMIT 8;",
		)
		.await
		.with_context(|| format!("hot cap indexed read failed during {phase}"))?;
	assert!(
		!indexed.is_empty(),
		"hot cap indexed read should return rows during {phase}"
	);
	Ok(())
}

async fn hot_cap_snapshot(ctx: &super::scenario::FaultScenarioCtx, phase: &str) -> String {
	let page_count = ctx.query("PRAGMA page_count;").await;
	let freelist_count = ctx.query("PRAGMA freelist_count;").await;
	let row_count = ctx.query("SELECT count(*) FROM hot_cap;").await;
	let max_id = ctx.query("SELECT coalesce(max(id), 0) FROM hot_cap;").await;
	format!(
		"phase={phase} page_count={page_count:?} freelist_count={freelist_count:?} row_count={row_count:?} max_id={max_id:?}"
	)
}

async fn thread_actor_write_cycle(
	ctx: &super::scenario::FaultScenarioCtx,
	cycle: usize,
	payload_bytes: usize,
) -> Result<()> {
	let summary_payload_blob_bytes = (payload_bytes / 2).max(1);
	let tool_payload_blob_bytes = (payload_bytes / 32).max(1);
	let mut sql = "BEGIN;".to_string();
	for summary_idx in 0..5 {
		sql.push_str(&format!(
			"INSERT INTO compaction_summaries
				(summary_id, summary_text, cut_message_id, created_at)
			VALUES
				('CS-{summary_idx}', lower(hex(zeroblob({summary_payload_blob_bytes}))) || '-{cycle:04}-{summary_idx:02}',
				 'M-cut-{cycle:04}-{summary_idx:02}', '2026-05-16T04:{:02}:{:02}.{:03}Z')
			ON CONFLICT(summary_id) DO UPDATE SET
				summary_text = excluded.summary_text,
				cut_message_id = excluded.cut_message_id,
				created_at = excluded.created_at;",
			(cycle + summary_idx) % 60,
			(cycle * 7 + summary_idx) % 60,
			(cycle * 37 + summary_idx) % 1000,
		));
	}
	for tool_idx in 0..32 {
		let state = match (cycle + tool_idx) % 4 {
			0 => "running",
			1 => "completed",
			2 => "failed",
			3 => "cancelled",
			_ => unreachable!(),
		};
		sql.push_str(&format!(
			"INSERT INTO tool_calls (call_id, message_id, state, created_at, payload)
			VALUES
				('TC-{cycle:04}-{tool_idx:02}', 'M-{cycle:04}', '{state}',
				 '2026-05-16T07:{:02}:{:02}.{:03}Z',
				 lower(hex(zeroblob({tool_payload_blob_bytes}))))
			ON CONFLICT(call_id) DO UPDATE SET
				state = excluded.state,
				payload = excluded.payload;",
			cycle % 60,
			(cycle * 11 + tool_idx) % 60,
			(cycle * 13 + tool_idx) % 1000,
		));
	}
	sql.push_str(&format!(
		"INSERT INTO messages (message_id, body, created_at)
		VALUES ('M-{cycle:04}', 'are you working? {cycle}', '2026-05-16T07:39:03.509Z')
		ON CONFLICT(message_id) DO UPDATE SET body = excluded.body;
		INSERT INTO thread_events (event_type, message_id, created_at)
		VALUES ('message_added', 'M-{cycle:04}', '2026-05-16T07:39:03.509Z');"
	));
	if cycle % 7 == 0 {
		sql.push_str(
			"DELETE FROM tool_calls WHERE rowid IN (
				SELECT rowid FROM tool_calls ORDER BY created_at ASC, rowid ASC LIMIT 8
			);",
		);
	}
	sql.push_str("COMMIT;");
	ctx.exec(LogicalOp::Sql(sql)).await
}

async fn thread_actor_assert_reads(
	ctx: &super::scenario::FaultScenarioCtx,
	phase: &str,
) -> Result<()> {
	ctx.verify_sqlite_integrity_rows()
		.await
		.with_context(|| format!("thread actor sqlite integrity failed during {phase}"))?;
	let summaries = ctx
		.query(
			"SELECT summary_id, length(summary_text), cut_message_id, created_at
			 FROM compaction_summaries
			 ORDER BY created_at ASC, rowid ASC;",
		)
		.await
		.with_context(|| format!("thread actor getAll query failed during {phase}"))?;
	assert_eq!(
		summaries.len(),
		5,
		"thread actor getAll should return five summaries during {phase}: {summaries:?}"
	);
	let latest = ctx
		.query(
			"SELECT summary_id, length(summary_text), cut_message_id, created_at
			 FROM compaction_summaries
			 ORDER BY created_at DESC, rowid DESC
			 LIMIT 1;",
		)
		.await
		.with_context(|| format!("thread actor getLatest query failed during {phase}"))?;
	assert_eq!(
		latest.len(),
		1,
		"thread actor getLatest should return one summary during {phase}: {latest:?}"
	);
	let running_tools = ctx
		.query(
			"SELECT call_id, state
			 FROM tool_calls
			 WHERE state = 'running'
			 ORDER BY created_at DESC, rowid DESC
			 LIMIT 16;",
		)
		.await
		.with_context(|| format!("thread actor tool_calls index query failed during {phase}"))?;
	assert!(
		!running_tools.is_empty(),
		"thread actor tool_calls indexed query should return running tools during {phase}"
	);
	Ok(())
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
