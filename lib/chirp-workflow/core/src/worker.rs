use global_error::GlobalResult;
use tokio::time::Duration;
use tracing::Instrument;
use uuid::Uuid;

use crate::{util, DatabaseHandle, RegistryHandle, WorkflowCtx};

const TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Used to spawn a new thread that indefinitely polls the database for new workflows. Only pulls workflows
/// that are registered in its registry. After pulling, the workflows are ran and their state is written to
/// the database.
pub struct Worker {
	worker_instance_id: Uuid,
	registry: RegistryHandle,
	db: DatabaseHandle,
}

impl Worker {
	pub fn new(registry: RegistryHandle, db: DatabaseHandle) -> Self {
		Worker {
			worker_instance_id: Uuid::new_v4(),
			registry,
			db,
		}
	}

	pub async fn start(mut self, pools: rivet_pools::Pools) -> GlobalResult<()> {
		tracing::info!(
			worker_instance_id=?self.worker_instance_id,
			"starting worker instance with {} registered workflows",
			self.registry.size(),
		);

		let mut interval = tokio::time::interval(TICK_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
		let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

		loop {
			interval.tick().await;
			self.tick(&shared_client, &pools, &cache).await?;
		}
	}

	/// Query the database for new workflows and run them.
	async fn tick(
		&mut self,
		shared_client: &chirp_client::SharedClientHandle,
		pools: &rivet_pools::Pools,
		cache: &rivet_cache::Cache,
	) -> GlobalResult<()> {
		tracing::trace!("tick");

		let registered_workflows = self
			.registry
			.workflows
			.keys()
			.map(|k| k.as_str())
			.collect::<Vec<_>>();

		// Query awake workflows
		let workflows = self
			.db
			.pull_workflows(self.worker_instance_id, &registered_workflows)
			.await?;
		for workflow in workflows {
			let conn = util::new_conn(
				&shared_client,
				pools,
				cache,
				workflow.ray_id,
				workflow.workflow_id,
				&workflow.workflow_name,
			);
			let wake_deadline_ts = workflow.wake_deadline_ts;
			let ctx = WorkflowCtx::new(self.registry.clone(), self.db.clone(), conn, workflow)?;

			tokio::task::spawn(
				async move {
					// Sleep until deadline
					if let Some(wake_deadline_ts) = wake_deadline_ts {
						util::sleep_until_ts(wake_deadline_ts).await;
					}

					ctx.run_workflow().await;
				}
				.in_current_span(),
			);
		}

		Ok(())
	}
}
