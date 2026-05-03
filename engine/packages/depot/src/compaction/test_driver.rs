use std::time::Duration;

use gas::db::debug::{DatabaseDebug, EventData, SignalState};

use super::*;

const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct DepotCompactionTestDriver<'a> {
	test_ctx: &'a TestCtx,
	wait_timeout: Duration,
}

impl<'a> DepotCompactionTestDriver<'a> {
	pub fn new(test_ctx: &'a TestCtx) -> Self {
		DepotCompactionTestDriver {
			test_ctx,
			wait_timeout: DEFAULT_WAIT_TIMEOUT,
		}
	}

	pub fn with_wait_timeout(mut self, wait_timeout: Duration) -> Self {
		self.wait_timeout = wait_timeout;
		self
	}

	pub async fn start_manager(
		&self,
		database_branch_id: DatabaseBranchId,
		actor_id: Option<String>,
		disable_planning_timers: bool,
	) -> Result<Id> {
		let input = if disable_planning_timers {
			DbManagerInput::with_planning_timers_disabled(database_branch_id, actor_id)
		} else {
			DbManagerInput::new(database_branch_id, actor_id)
		};

		self.test_ctx
			.workflow(input)
			.tag(
				DATABASE_BRANCH_ID_TAG,
				&database_branch_tag_value(database_branch_id),
			)
			.unique()
			.dispatch()
			.await
	}

	pub async fn force_compaction(
		&self,
		manager_workflow_id: Id,
		database_branch_id: DatabaseBranchId,
		work: ForceCompactionWork,
	) -> Result<ForceCompactionResult> {
		self.force_compaction_with_request_id(
			manager_workflow_id,
			database_branch_id,
			Id::new_v1(self.test_ctx.config().dc_label()),
			work,
		)
		.await
	}

	pub async fn force_compaction_with_request_id(
		&self,
		manager_workflow_id: Id,
		database_branch_id: DatabaseBranchId,
		request_id: Id,
		work: ForceCompactionWork,
	) -> Result<ForceCompactionResult> {
		let signal_id = self
			.test_ctx
			.signal(ForceCompaction {
				database_branch_id,
				request_id,
				requested_work: work,
			})
			.to_workflow_id(manager_workflow_id)
			.send()
			.await?
			.context("force compaction signal should target manager workflow")?;

		self.wait_for_signal_ack(signal_id).await?;
		self.wait_for_force_result(manager_workflow_id, request_id)
			.await
	}

	async fn wait_for_signal_ack(&self, signal_id: Id) -> Result<()> {
		self.wait_until("force compaction signal ack", || async {
			let signal = DatabaseDebug::get_signals(self.test_ctx.debug_db(), vec![signal_id])
				.await?
				.into_iter()
				.next();

			if signal.is_some_and(|signal| signal.state == SignalState::Acked) {
				return Ok(Some(()));
			}

			Ok(None)
		})
		.await
	}

	async fn wait_for_force_result(
		&self,
		manager_workflow_id: Id,
		request_id: Id,
	) -> Result<ForceCompactionResult> {
		self.wait_until("force compaction result", || async {
			let history = DatabaseDebug::get_workflow_history(
				self.test_ctx.debug_db(),
				manager_workflow_id,
				true,
			)
			.await?
			.context("manager workflow history not found")?;

			for event in history.events.into_iter().rev() {
				if let EventData::Loop(loop_event) = event.data {
					let state = serde_json::from_value::<DbManagerState>(loop_event.state)?;
					if let Some(result) = state
						.force_compactions
						.recent_results
						.into_iter()
						.find(|result| result.request_id == request_id)
					{
						return Ok(Some(result));
					}
				}
			}

			Ok(None)
		})
		.await
	}

	async fn wait_until<T, F, Fut>(&self, description: &'static str, mut check: F) -> Result<T>
	where
		F: FnMut() -> Fut,
		Fut: std::future::Future<Output = Result<Option<T>>>,
	{
		let started_at = tokio::time::Instant::now();
		loop {
			if let Some(value) = check().await? {
				return Ok(value);
			}

			if started_at.elapsed() > self.wait_timeout {
				bail!("timed out waiting for {description}");
			}

			// Gasoline debug rows do not expose a waiter for this test-driver observation.
			tokio::time::sleep(Duration::from_millis(25)).await;
		}
	}
}
