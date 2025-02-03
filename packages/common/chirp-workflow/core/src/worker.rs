use global_error::GlobalResult;
use tokio::time::Duration;
use tracing::Instrument;
use uuid::Uuid;

use crate::{ctx::WorkflowCtx, db::DatabaseHandle, metrics, registry::RegistryHandle, utils};

pub const TICK_INTERVAL: Duration = Duration::from_secs(120);

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
		// Get rid of metrics that don't exist in the db anymore (declarative)
		metrics::PULL_WORKFLOWS_DURATION.reset();
		metrics::PULL_WORKFLOWS_HISTORY_DURATION.reset();

		Worker {
			worker_instance_id: Uuid::new_v4(),
			registry,
			db,
		}
	}

	/// Polls the database periodically
	pub async fn poll_start(
		mut self,
		config: rivet_config::Config,
		pools: rivet_pools::Pools,
	) -> GlobalResult<()> {
		tracing::debug!(
			worker_instance_id = ?self.worker_instance_id,
			registered_workflows = ?self.registry.size(),
			"started worker instance",
		);

		let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
		let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

		// Regular tick interval to poll the database
		let mut interval = tokio::time::interval(TICK_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			interval.tick().await;
			self.tick(&shared_client, &config, &pools, &cache).await?;
		}
	}

	/// Polls the database periodically or wakes immediately when `Database::wake` finishes
	pub async fn wake_start(
		mut self,
		config: rivet_config::Config,
		pools: rivet_pools::Pools,
	) -> GlobalResult<()> {
		tracing::debug!(
			worker_instance_id = ?self.worker_instance_id,
			registered_workflows = ?self.registry.size(),
			"started worker instance",
		);

		let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
		let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

		// Regular tick interval to poll the database
		let mut interval = tokio::time::interval(TICK_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			tokio::select! {
				_ = interval.tick() => {},
				res = self.db.wake() => res?,
			}

			self.tick(&shared_client, &config, &pools, &cache).await?;
		}
	}

	/// Query the database for new workflows and run them.
	async fn tick(
		&mut self,
		shared_client: &chirp_client::SharedClientHandle,
		config: &rivet_config::Config,
		pools: &rivet_pools::Pools,
		cache: &rivet_cache::Cache,
	) -> GlobalResult<()> {
		tracing::trace!("tick");

		// Create filter from registered workflow names
		let filter = self
			.registry
			.workflows
			.keys()
			.map(|k| k.as_str())
			.collect::<Vec<_>>();

		// Query awake workflows
		let workflows = self
			.db
			.pull_workflows(self.worker_instance_id, &filter)
			.await?;
		for workflow in workflows {
			let conn = utils::new_conn(
				shared_client,
				pools,
				cache,
				workflow.ray_id,
				workflow.workflow_id,
				&workflow.workflow_name,
			);
			let ctx = WorkflowCtx::new(
				self.registry.clone(),
				self.db.clone(),
				config.clone(),
				conn,
				workflow,
			)
			.await?;

			tokio::task::spawn(
				async move {
					if let Err(err) = ctx.run().await {
						tracing::error!(?err, "unhandled error");
					}
				}
				.in_current_span(),
			);
		}

		Ok(())
	}
}
