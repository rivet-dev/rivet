#![cfg(feature = "test-faults")]

use std::time::Duration;

use anyhow::Result;
use depot::fault::{
	ColdTierFaultPoint, CommitFaultPoint, DepotFaultContext, DepotFaultController, DepotFaultPoint,
	DepotFaultReplayEventKind, FaultBoundary, HotCompactionFaultPoint, ReadFaultPoint,
};
use depot::types::DatabaseBranchId;

#[tokio::test]
async fn fault_controller_matches_scope_and_invocation() -> Result<()> {
	let controller = DepotFaultController::new();
	let branch_id = DatabaseBranchId::new_v4();
	let point = DepotFaultPoint::Commit(CommitFaultPoint::BeforeHeadWrite);

	controller
		.at(point.clone())
		.database_id("db-a")
		.database_branch_id(branch_id)
		.checkpoint("after-write")
		.seed(7)
		.nth(2)
		.drop_artifact()?;

	let wrong_scope = DepotFaultContext {
		database_id: Some("db-b".to_string()),
		database_branch_id: Some(branch_id),
		checkpoint: Some("after-write".to_string()),
		seed: Some(7),
		..DepotFaultContext::default()
	};
	assert!(
		controller
			.maybe_fire(point.clone(), wrong_scope)
			.await?
			.is_none()
	);

	let matching_scope = DepotFaultContext::new()
		.database_id("db-a")
		.database_branch_id(branch_id)
		.checkpoint("after-write")
		.seed(7);
	assert!(
		controller
			.maybe_fire(point.clone(), matching_scope.clone())
			.await?
			.is_none()
	);
	let fired = controller
		.maybe_fire(point.clone(), matching_scope.clone())
		.await?
		.expect("second matching invocation should fire");

	assert_eq!(fired.invocation, 2);
	assert_eq!(fired.boundary, FaultBoundary::PreDurableCommit);
	assert_eq!(controller.replay_log().len(), 1);
	controller.assert_expected_fired()?;

	Ok(())
}

#[tokio::test]
async fn unready_rules_do_not_block_later_matching_rules() -> Result<()> {
	let controller = DepotFaultController::new();
	let point = DepotFaultPoint::Commit(CommitFaultPoint::BeforeTx);

	controller.at(point.clone()).nth(3).drop_artifact()?;
	controller.at(point.clone()).once().drop_artifact()?;

	let fired = controller
		.maybe_fire(point, DepotFaultContext::default())
		.await?
		.expect("later ready rule should fire");

	assert_eq!(fired.invocation, 1);
	assert_eq!(controller.replay_log().len(), 1);

	Ok(())
}

#[tokio::test]
async fn fail_action_returns_error_and_records_replay() -> Result<()> {
	let controller = DepotFaultController::new();
	let point = DepotFaultPoint::Read(ReadFaultPoint::ColdObjectMissing);

	controller
		.at(point.clone())
		.once()
		.fail("cold object disappeared")?;

	let err = controller
		.maybe_fire(point, DepotFaultContext::default())
		.await
		.expect_err("fail actions should return an error");

	assert!(err.to_string().contains("cold object disappeared"));
	assert_eq!(
		controller.replay_log()[0].kind,
		DepotFaultReplayEventKind::Fired
	);
	assert_eq!(controller.replay_log()[0].boundary, FaultBoundary::ReadOnly);

	Ok(())
}

#[tokio::test]
async fn pause_action_waits_for_release() -> Result<()> {
	let controller = DepotFaultController::new();
	let point =
		DepotFaultPoint::HotCompaction(HotCompactionFaultPoint::AfterStageBeforeFinishSignal);
	let pause = controller.pause_handle("hot-staged");

	controller.at(point.clone()).once().pause("hot-staged")?;

	let task_controller = controller.clone();
	let task = tokio::spawn(async move {
		task_controller
			.maybe_fire(point, DepotFaultContext::default())
			.await
	});

	pause.wait_reached().await;
	assert_eq!(pause.checkpoint(), "hot-staged");
	pause.release();

	let fired = task.await??.expect("pause action should fire");
	assert_eq!(fired.boundary, FaultBoundary::WorkflowOnly);
	controller.assert_expected_fired()?;

	Ok(())
}

#[tokio::test(start_paused = true)]
async fn delay_action_is_bounded_and_fires() -> Result<()> {
	let controller = DepotFaultController::new();
	let point = DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject);

	controller
		.at(point.clone())
		.once()
		.delay(Duration::from_millis(10))?;

	let task_controller = controller.clone();
	let task = tokio::spawn(async move {
		task_controller
			.maybe_fire(point, DepotFaultContext::default())
			.await
	});

	tokio::time::advance(Duration::from_millis(10)).await;
	let fired = task.await??.expect("delay action should fire");
	assert_eq!(fired.boundary, FaultBoundary::ReadOnly);

	Ok(())
}

#[test]
fn unfired_expected_faults_are_reported_in_replay() -> Result<()> {
	let controller = DepotFaultController::new();
	let point = DepotFaultPoint::Commit(CommitFaultPoint::AfterUdbCommit);

	controller.at(point).once().drop_artifact()?;

	let err = controller
		.assert_expected_fired()
		.expect_err("unfired expected rule should fail the test");
	assert!(
		err.to_string()
			.contains("expected depot faults did not fire")
	);

	let replay = controller.replay_log_with_unfired();
	assert_eq!(replay.len(), 1);
	assert_eq!(
		replay[0].kind,
		DepotFaultReplayEventKind::ExpectedButUnfired
	);
	assert_eq!(
		replay[0].boundary,
		FaultBoundary::AmbiguousAfterDurableCommit
	);

	Ok(())
}
