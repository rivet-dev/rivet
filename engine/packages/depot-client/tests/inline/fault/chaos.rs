use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use depot::fault::{
	ColdCompactionFaultPoint, ColdTierFaultPoint, CommitFaultPoint, DepotFaultPoint,
	DepotFaultReplayEventKind, FaultBoundary, HotCompactionFaultPoint, ReadFaultPoint,
	ReclaimFaultPoint,
};
use depot::workflows::compaction::ForceCompactionWork;
use futures_util::future;

use super::{
	FaultProfile, FaultReplayPhase, FaultScenario, LogicalOp,
	scenario::{FaultScenarioCtx, FaultScenarioReplayRecord},
};

#[test]
fn chaos_curated_seed_19f0_ba5e() -> Result<()> {
	run_chaos_seed(0x19f0_ba5e, ChaosRunProfile::curated())
}

#[test]
#[ignore = "chaos soak fault scenarios are replayed explicitly by seed"]
fn chaos_replay_seed_3d7a_21c9() -> Result<()> {
	run_chaos_seed(0x3d7a_21c9, ChaosRunProfile::soak(42, 5))
}

#[test]
#[ignore = "chaos soak fault scenarios are replayed explicitly by seed"]
fn chaos_replay_seed_845f_102b() -> Result<()> {
	run_chaos_seed(0x845f_102b, ChaosRunProfile::soak(54, 6))
}

fn run_chaos_seed(seed: u64, profile: ChaosRunProfile) -> Result<()> {
	let plan = ChaosPlan::new(seed);
	let fault_plan = plan.clone();
	let workload_plan = plan.clone();
	let verify_plan = plan.clone();
	let scenario_name = format!("chaos_{}_seed_{seed:016x}", profile.name);

	FaultScenario::new(scenario_name)
		.seed(seed)
		.profile(FaultProfile::Chaos)
		.setup(|ctx| async move {
			ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB NOT NULL);")
				.await
		})
		.faults(move |faults| {
			faults
				.at(DepotFaultPoint::Commit(
					fault_plan.commit_pause_point.clone(),
				))
				.once()
				.pause(commit_pause_checkpoint(seed))?;
			faults
				.at(DepotFaultPoint::Read(fault_plan.read_delay_point.clone()))
				.once()
				.delay(Duration::from_millis(fault_plan.read_delay_ms))?;
			faults
				.at(DepotFaultPoint::HotCompaction(
					fault_plan.hot_delay_point.clone(),
				))
				.once()
				.delay(Duration::from_millis(fault_plan.hot_delay_ms))?;
			faults
				.at(DepotFaultPoint::ColdCompaction(
					fault_plan.cold_delay_point.clone(),
				))
				.once()
				.delay(Duration::from_millis(fault_plan.cold_delay_ms))?;
			faults
				.at(DepotFaultPoint::Reclaim(
					fault_plan.reclaim_delay_point.clone(),
				))
				.once()
				.delay(Duration::from_millis(fault_plan.reclaim_delay_ms))?;
			faults
				.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject))
				.once()
				.delay(Duration::from_millis(fault_plan.cold_put_delay_ms))?;
			Ok(())
		})
		.workload(move |ctx| async move {
			let pause = ctx
				.fault_controller()
				.pause_handle(commit_pause_checkpoint(seed));
			pause.release();
			ctx.exec(LogicalOp::Put {
				key: format!("seed-{seed:x}-paused"),
				value: vec![0xaa, (seed & 0xff) as u8],
			})
			.await?;
			ctx.checkpoint(format!("commit-pause-released-{seed:016x}"))
				.await?;

			let mut rng = ChaosRng::new(seed ^ 0x9e37_79b9_7f4a_7c15);
			for index in 0..profile.pre_cold_ops {
				ctx.exec(random_logical_op(&mut rng, index)).await?;
				if index % profile.reload_every == 1 {
					ctx.reload_database().await?;
					ctx.checkpoint(format!("reload-{index}-{seed:016x}"))
						.await?;
				}
			}

			run_overlapping_hot_compaction(&ctx, seed).await?;

			for index in profile.pre_cold_ops..profile.total_ops {
				ctx.exec(random_logical_op(&mut rng, index)).await?;
				if index % profile.reload_every == profile.reload_every - 1 {
					ctx.reload_database().await?;
					ctx.checkpoint(format!("late-reload-{index}-{seed:016x}"))
						.await?;
				}
			}
			let restore_point = ctx.create_restore_point().await?;
			ctx.force_compaction(ForceCompactionWork {
				hot: true,
				cold: true,
				reclaim: false,
				final_settle: false,
			})
			.await?;
			ctx.checkpoint(format!("after-cold-{seed:016x}")).await?;

			ctx.fault_controller()
				.at(DepotFaultPoint::Read(ReadFaultPoint::AfterShardBlobLoad))
				.once()
				.drop_artifact()?;
			ctx.fault_controller()
				.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject))
				.once()
				.delay(Duration::from_millis(workload_plan.cold_get_delay_ms))?;
			let cold_read_started = Instant::now();
			ctx.read_page_from_depot(1).await?;
			let cold_read_elapsed = cold_read_started.elapsed();
			assert_delay_elapsed(
				seed,
				"cold-tier-get",
				cold_read_elapsed,
				Duration::from_millis(workload_plan.cold_get_delay_ms),
			)?;
			ctx.checkpoint(format!(
				"after-cold-read-{seed:016x}-elapsed-{}ms",
				cold_read_elapsed.as_millis()
			))
			.await?;

			ctx.delete_restore_point(restore_point).await?;
			ctx.exec(LogicalOp::Put {
				key: format!("seed-{seed:x}-after-cold"),
				value: vec![0xc0, 0x1d],
			})
			.await?;
			let _current_restore_point = ctx.create_restore_point().await?;
			let _grace = ctx.override_cold_object_delete_grace(0).await?;
			ctx.force_compaction(ForceCompactionWork {
				hot: true,
				cold: true,
				reclaim: true,
				final_settle: true,
			})
			.await?;
			ctx.checkpoint(format!("after-reclaim-{seed:016x}")).await?;

			ctx.reload_database().await?;
			ctx.verify_sqlite_integrity().await?;
			ctx.verify_against_native_oracle().await?;
			ctx.verify_depot_invariants().await?;
			Ok(())
		})
		.verify(move |ctx| async move {
			ctx.verify_sqlite_integrity().await?;
			ctx.verify_against_native_oracle().await?;
			ctx.verify_depot_invariants().await?;

			let replay = ctx.replay_record().await;
			assert_eq!(replay.seed, seed);
			assert_eq!(replay.profile, FaultProfile::Chaos);
			assert!(
				replay.checkpoints.len() >= 7,
				"{}",
				chaos_failure_context(
					seed,
					"checkpoint-count",
					"chaos should record reload and compaction checkpoints",
					&replay,
				)
			);
			assert!(
				replay.workload.len() >= profile.total_ops + 2,
				"{}",
				chaos_failure_context(
					seed,
					"workload-count",
					"chaos should record the generated logical workload",
					&replay,
				)
			);
			assert_eq!(replay.oracle_result.as_deref(), Some("matched"));
			assert!(replay.branch_head_before_faults.is_some());
			assert!(replay.branch_head_after_workload.is_some());

			let fired_boundaries = replay
				.fault_events
				.iter()
				.filter(|event| event.event.kind == DepotFaultReplayEventKind::Fired)
				.map(|event| event.event.boundary)
				.collect::<Vec<_>>();
			for boundary in [
				FaultBoundary::PreDurableCommit,
				FaultBoundary::ReadOnly,
				FaultBoundary::WorkflowOnly,
			] {
				assert!(
					fired_boundaries.contains(&boundary),
					"{}",
					chaos_failure_context(
						seed,
						"boundary",
						&format!("missing fired boundary {boundary:?}"),
						&replay,
					)
				);
			}
			assert_eq!(
				replay
					.fault_events
					.iter()
					.filter(|event| event.event.kind == DepotFaultReplayEventKind::Fired)
					.count(),
				9,
				"{}",
				chaos_failure_context(
					seed,
					"fault-count",
					"chaos should fire every scheduled chaos fault",
					&replay,
				)
			);
			for point in [
				DepotFaultPoint::Commit(verify_plan.commit_pause_point.clone()),
				DepotFaultPoint::Read(verify_plan.read_delay_point.clone()),
				DepotFaultPoint::HotCompaction(HotCompactionFaultPoint::StageAfterInputRead),
				DepotFaultPoint::HotCompaction(verify_plan.hot_delay_point.clone()),
				DepotFaultPoint::ColdCompaction(verify_plan.cold_delay_point.clone()),
				DepotFaultPoint::Reclaim(verify_plan.reclaim_delay_point.clone()),
				DepotFaultPoint::ColdTier(ColdTierFaultPoint::PutObject),
				DepotFaultPoint::Read(ReadFaultPoint::AfterShardBlobLoad),
				DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject),
			] {
				assert!(
					replay.fault_events.iter().any(|event| {
						event.event.kind == DepotFaultReplayEventKind::Fired
							&& event.event.point == point
							&& event.phase == FaultReplayPhase::Workload
					}),
					"{}",
					chaos_failure_context(
						seed,
						"fault-point",
						&format!("missing workload fault point {point:?}"),
						&replay,
					)
				);
			}
			Ok(())
		})
		.run()
		.with_context(|| format!("chaos replay failed for seed {seed:016x}"))
}

#[derive(Clone, Copy)]
struct ChaosRunProfile {
	name: &'static str,
	pre_cold_ops: usize,
	total_ops: usize,
	reload_every: usize,
}

impl ChaosRunProfile {
	fn curated() -> Self {
		Self {
			name: "curated",
			pre_cold_ops: 10,
			total_ops: 16,
			reload_every: 4,
		}
	}

	fn soak(total_ops: usize, reload_every: usize) -> Self {
		Self {
			name: "soak",
			pre_cold_ops: total_ops / 2,
			total_ops,
			reload_every,
		}
	}
}

async fn run_overlapping_hot_compaction(ctx: &FaultScenarioCtx, seed: u64) -> Result<()> {
	let checkpoint = format!("chaos-hot-overlap-{seed:016x}");
	let pause = ctx.fault_controller().pause_handle(checkpoint.clone());
	ctx.fault_controller()
		.at(DepotFaultPoint::HotCompaction(
			HotCompactionFaultPoint::StageAfterInputRead,
		))
		.once()
		.pause(checkpoint.clone())?;

	let compaction_ctx = ctx.clone();
	let overlap_ctx = ctx.clone();
	let compaction = async move {
		let started = Instant::now();
		let result = compaction_ctx.force_hot_compaction().await;
		(result, started.elapsed())
	};
	let overlap = async move {
		pause.wait_reached().await;
		overlap_ctx
			.checkpoint(format!("hot-overlap-paused-{seed:016x}"))
			.await?;
		overlap_ctx.reload_database().await?;
		overlap_ctx.read_page_from_depot(1).await?;
		overlap_ctx
			.checkpoint(format!("hot-overlap-read-{seed:016x}"))
			.await?;
		pause.release();
		Ok::<(), anyhow::Error>(())
	};

	let ((compaction_result, elapsed), overlap_result) = future::join(compaction, overlap).await;
	overlap_result?;
	compaction_result?;
	ensure!(
		elapsed >= Duration::from_millis(1),
		"seed {seed:016x} hot-overlap completed without measurable paused overlap: elapsed={elapsed:?}",
	);
	ctx.checkpoint(format!("after-hot-overlap-{seed:016x}"))
		.await
}

fn assert_delay_elapsed(
	seed: u64,
	label: &str,
	elapsed: Duration,
	expected: Duration,
) -> Result<()> {
	ensure!(
		elapsed >= expected,
		"seed {seed:016x} delay classification {label} elapsed={elapsed:?} expected_at_least={expected:?}",
	);
	Ok(())
}

fn chaos_failure_context(
	seed: u64,
	checkpoint: &str,
	reason: &str,
	replay: &FaultScenarioReplayRecord,
) -> String {
	format!(
		"seed {seed:016x} checkpoint={checkpoint} {reason}; workload={:?}; replay={:?}",
		replay.workload, replay.fault_events
	)
}

#[derive(Clone)]
struct ChaosPlan {
	commit_pause_point: CommitFaultPoint,
	read_delay_point: ReadFaultPoint,
	hot_delay_point: HotCompactionFaultPoint,
	cold_delay_point: ColdCompactionFaultPoint,
	reclaim_delay_point: ReclaimFaultPoint,
	read_delay_ms: u64,
	hot_delay_ms: u64,
	cold_delay_ms: u64,
	reclaim_delay_ms: u64,
	cold_put_delay_ms: u64,
	cold_get_delay_ms: u64,
}

impl ChaosPlan {
	fn new(seed: u64) -> Self {
		let mut rng = ChaosRng::new(seed);
		Self {
			commit_pause_point: choose_commit_point(&mut rng),
			read_delay_point: choose_read_point(&mut rng),
			hot_delay_point: choose_hot_point(&mut rng),
			cold_delay_point: choose_cold_point(&mut rng),
			reclaim_delay_point: choose_reclaim_point(&mut rng),
			read_delay_ms: rng.delay_ms(),
			hot_delay_ms: rng.delay_ms(),
			cold_delay_ms: rng.delay_ms(),
			reclaim_delay_ms: rng.delay_ms(),
			cold_put_delay_ms: rng.delay_ms(),
			cold_get_delay_ms: rng.delay_ms(),
		}
	}
}

struct ChaosRng {
	state: u64,
}

impl ChaosRng {
	fn new(seed: u64) -> Self {
		Self { state: seed | 1 }
	}

	fn next_u64(&mut self) -> u64 {
		self.state = self
			.state
			.wrapping_mul(6_364_136_223_846_793_005)
			.wrapping_add(1_442_695_040_888_963_407);
		self.state
	}

	fn index(&mut self, len: usize) -> usize {
		(self.next_u64() as usize) % len
	}

	fn delay_ms(&mut self) -> u64 {
		1 + (self.next_u64() % 5)
	}
}

fn random_logical_op(rng: &mut ChaosRng, index: usize) -> LogicalOp {
	let key = format!("k{:02}", rng.index(6));
	if index % 5 == 4 {
		return LogicalOp::Delete { key };
	}

	let len = 1 + rng.index(5);
	let value = (0..len)
		.map(|offset| (rng.next_u64().wrapping_add(offset as u64) & 0xff) as u8)
		.collect();
	LogicalOp::Put { key, value }
}

fn choose_commit_point(rng: &mut ChaosRng) -> CommitFaultPoint {
	[
		CommitFaultPoint::AfterHeadRead,
		CommitFaultPoint::BeforeDeltaWrites,
		CommitFaultPoint::BeforeHeadWrite,
	][rng.index(3)]
	.clone()
}

fn choose_read_point(rng: &mut ChaosRng) -> ReadFaultPoint {
	[
		ReadFaultPoint::AfterPidxScan,
		ReadFaultPoint::ColdRefSelected,
		ReadFaultPoint::BeforeReturnPages,
	][rng.index(3)]
	.clone()
}

fn choose_hot_point(rng: &mut ChaosRng) -> HotCompactionFaultPoint {
	[
		HotCompactionFaultPoint::StageBeforeInputRead,
		HotCompactionFaultPoint::InstallAfterStagedRead,
		HotCompactionFaultPoint::InstallBeforeRootUpdate,
	][rng.index(3)]
	.clone()
}

fn choose_cold_point(rng: &mut ChaosRng) -> ColdCompactionFaultPoint {
	[
		ColdCompactionFaultPoint::UploadBeforePutObject,
		ColdCompactionFaultPoint::PublishBeforeColdRefWrite,
		ColdCompactionFaultPoint::PublishAfterRootUpdate,
	][rng.index(3)]
	.clone()
}

fn choose_reclaim_point(rng: &mut ChaosRng) -> ReclaimFaultPoint {
	[
		ReclaimFaultPoint::PlanBeforeSnapshot,
		ReclaimFaultPoint::PlanAfterSnapshot,
		ReclaimFaultPoint::BeforeCleanupRows,
	][rng.index(3)]
	.clone()
}

fn commit_pause_checkpoint(seed: u64) -> String {
	format!("chaos-commit-pause-{seed:016x}")
}
