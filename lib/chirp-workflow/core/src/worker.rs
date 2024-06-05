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
	registry: RegistryHandle,
	db: DatabaseHandle,
}

impl Worker {
	pub fn new(registry: RegistryHandle, db: DatabaseHandle) -> Self {
		Worker { registry, db }
	}

	pub async fn start(mut self, pools: rivet_pools::Pools) -> GlobalResult<()> {
		let mut interval = tokio::time::interval(TICK_INTERVAL);

		let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
		let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

		loop {
			interval.tick().await;
			self.tick(&shared_client, &pools, &cache).await?;
		}
	}

	// Query the database for new workflows and run them.
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
		let workflows = self.db.pull_workflows(&registered_workflows).await?;
		for workflow in workflows {
			let conn = new_conn(
				&shared_client,
				pools,
				cache,
				workflow.workflow_id,
				&workflow.workflow_name,
				workflow.ray_id,
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

fn new_conn(
	shared_client: &chirp_client::SharedClientHandle,
	pools: &rivet_pools::Pools,
	cache: &rivet_cache::Cache,
	workflow_id: Uuid,
	name: &str,
	ray_id: Uuid,
) -> rivet_connection::Connection {
	let req_id = workflow_id;
	let client = shared_client.clone().wrap(
		req_id,
		ray_id,
		vec![chirp_client::TraceEntry {
			context_name: name.into(),
			req_id: Some(req_id.into()),
			ts: rivet_util::timestamp::now(),
			run_context: match rivet_util::env::run_context() {
				rivet_util::env::RunContext::Service => chirp_client::RunContext::Service,
				rivet_util::env::RunContext::Test => chirp_client::RunContext::Test,
			} as i32,
		}],
	);

	rivet_connection::Connection::new(client, pools.clone(), cache.clone())
}
