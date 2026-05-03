#[cfg(any(debug_assertions, feature = "test-faults"))]
use std::sync::Arc;

#[cfg(any(debug_assertions, feature = "test-faults"))]
use parking_lot::Mutex;
#[cfg(debug_assertions)]
use tokio::sync::Notify;

use super::*;

#[cfg(feature = "test-faults")]
use crate::cold_tier::ColdTier;
#[cfg(feature = "test-faults")]
use crate::fault::{
	ColdCompactionFaultPoint, DepotFaultContext, DepotFaultController, DepotFaultFired,
	DepotFaultPoint, HotCompactionFaultPoint, ReclaimFaultPoint,
};

#[cfg(debug_assertions)]
static PAUSE_AFTER_HOT_STAGE: Mutex<Option<(DatabaseBranchId, Arc<Notify>, Arc<Notify>)>> =
	Mutex::new(None);

#[cfg(debug_assertions)]
pub struct PauseGuard {
	slot: &'static Mutex<Option<(DatabaseBranchId, Arc<Notify>, Arc<Notify>)>>,
}

#[cfg(debug_assertions)]
pub fn pause_after_hot_stage(
	database_branch_id: DatabaseBranchId,
) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
	let reached = Arc::new(Notify::new());
	let release = Arc::new(Notify::new());
	*PAUSE_AFTER_HOT_STAGE.lock() = Some((
		database_branch_id,
		Arc::clone(&reached),
		Arc::clone(&release),
	));

	(
		PauseGuard {
			slot: &PAUSE_AFTER_HOT_STAGE,
		},
		reached,
		release,
	)
}

#[cfg(debug_assertions)]
pub(super) async fn maybe_pause_after_hot_stage(database_branch_id: DatabaseBranchId) {
	let hook = PAUSE_AFTER_HOT_STAGE
		.lock()
		.as_ref()
		.filter(|(hook_branch_id, _, _)| *hook_branch_id == database_branch_id)
		.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));

	if let Some((reached, release)) = hook {
		reached.notify_one();
		release.notified().await;
	}
}

#[cfg(not(debug_assertions))]
pub(super) async fn maybe_pause_after_hot_stage(_database_branch_id: DatabaseBranchId) {}

#[cfg(feature = "test-faults")]
static WORKFLOW_FAULT_CONTROLLERS: Mutex<Vec<(DatabaseBranchId, DepotFaultController)>> =
	Mutex::new(Vec::new());

#[cfg(feature = "test-faults")]
static COLD_OBJECT_DELETE_GRACE_OVERRIDES: Mutex<Vec<(DatabaseBranchId, i64)>> =
	Mutex::new(Vec::new());

#[cfg(feature = "test-faults")]
pub struct WorkflowFaultControllerGuard {
	database_branch_id: DatabaseBranchId,
}

#[cfg(feature = "test-faults")]
pub struct ColdObjectDeleteGraceGuard {
	database_branch_id: DatabaseBranchId,
}

#[cfg(feature = "test-faults")]
pub struct WorkflowColdTierGuard {
	database_branch_id: DatabaseBranchId,
}

#[cfg(feature = "test-faults")]
pub fn register_workflow_fault_controller(
	database_branch_id: DatabaseBranchId,
	controller: DepotFaultController,
) -> WorkflowFaultControllerGuard {
	let mut controllers = WORKFLOW_FAULT_CONTROLLERS.lock();
	if let Some((_, existing)) = controllers
		.iter_mut()
		.find(|(branch_id, _)| *branch_id == database_branch_id)
	{
		*existing = controller;
	} else {
		controllers.push((database_branch_id, controller));
	}

	WorkflowFaultControllerGuard { database_branch_id }
}

#[cfg(feature = "test-faults")]
pub fn install_workflow_cold_tier_for_test(
	database_branch_id: DatabaseBranchId,
	cold_tier: Arc<dyn ColdTier>,
) -> WorkflowColdTierGuard {
	crate::compaction::shared::install_workflow_test_cold_tier_for_test(
		database_branch_id,
		cold_tier,
	);
	WorkflowColdTierGuard { database_branch_id }
}

#[cfg(feature = "test-faults")]
pub fn override_cold_object_delete_grace_for_test(
	database_branch_id: DatabaseBranchId,
	grace_ms: i64,
) -> ColdObjectDeleteGraceGuard {
	let mut overrides = COLD_OBJECT_DELETE_GRACE_OVERRIDES.lock();
	if let Some((_, existing)) = overrides
		.iter_mut()
		.find(|(branch_id, _)| *branch_id == database_branch_id)
	{
		*existing = grace_ms;
	} else {
		overrides.push((database_branch_id, grace_ms));
	}

	ColdObjectDeleteGraceGuard { database_branch_id }
}

#[cfg(feature = "test-faults")]
pub(crate) fn cold_object_delete_grace_ms(database_branch_id: DatabaseBranchId) -> i64 {
	COLD_OBJECT_DELETE_GRACE_OVERRIDES
		.lock()
		.iter()
		.find(|(branch_id, _)| *branch_id == database_branch_id)
		.map(|(_, grace_ms)| *grace_ms)
		.unwrap_or(CMP_COLD_OBJECT_DELETE_GRACE_MS)
}

#[cfg(not(feature = "test-faults"))]
pub(crate) fn cold_object_delete_grace_ms(_database_branch_id: DatabaseBranchId) -> i64 {
	CMP_COLD_OBJECT_DELETE_GRACE_MS
}

#[cfg(feature = "test-faults")]
pub(crate) async fn maybe_fire_hot_compaction_fault(
	database_branch_id: DatabaseBranchId,
	point: HotCompactionFaultPoint,
) -> Result<Option<DepotFaultFired>> {
	maybe_fire_workflow_fault(database_branch_id, DepotFaultPoint::HotCompaction(point)).await
}

#[cfg(feature = "test-faults")]
pub(crate) async fn maybe_fire_cold_compaction_fault(
	database_branch_id: DatabaseBranchId,
	point: ColdCompactionFaultPoint,
) -> Result<Option<DepotFaultFired>> {
	maybe_fire_workflow_fault(database_branch_id, DepotFaultPoint::ColdCompaction(point)).await
}

#[cfg(feature = "test-faults")]
pub(crate) async fn maybe_fire_reclaim_fault(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
) -> Result<Option<DepotFaultFired>> {
	maybe_fire_workflow_fault(database_branch_id, DepotFaultPoint::Reclaim(point)).await
}

#[cfg(feature = "test-faults")]
async fn maybe_fire_workflow_fault(
	database_branch_id: DatabaseBranchId,
	point: DepotFaultPoint,
) -> Result<Option<DepotFaultFired>> {
	let controller = WORKFLOW_FAULT_CONTROLLERS
		.lock()
		.iter()
		.find(|(branch_id, _)| *branch_id == database_branch_id)
		.map(|(_, controller)| controller.clone());

	let Some(controller) = controller else {
		return Ok(None);
	};

	controller
		.maybe_fire(
			point,
			DepotFaultContext::new().database_branch_id(database_branch_id),
		)
		.await
}

#[cfg(debug_assertions)]
impl Drop for PauseGuard {
	fn drop(&mut self) {
		*self.slot.lock() = None;
	}
}

#[cfg(feature = "test-faults")]
impl Drop for WorkflowFaultControllerGuard {
	fn drop(&mut self) {
		let mut controllers = WORKFLOW_FAULT_CONTROLLERS.lock();
		controllers.retain(|(branch_id, _)| *branch_id != self.database_branch_id);
	}
}

#[cfg(feature = "test-faults")]
impl Drop for WorkflowColdTierGuard {
	fn drop(&mut self) {
		crate::compaction::shared::clear_workflow_test_cold_tier_for_test(self.database_branch_id);
	}
}

#[cfg(feature = "test-faults")]
impl Drop for ColdObjectDeleteGraceGuard {
	fn drop(&mut self) {
		let mut overrides = COLD_OBJECT_DELETE_GRACE_OVERRIDES.lock();
		overrides.retain(|(branch_id, _)| *branch_id != self.database_branch_id);
	}
}
