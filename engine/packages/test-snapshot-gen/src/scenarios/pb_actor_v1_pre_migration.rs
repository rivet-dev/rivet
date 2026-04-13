use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use gas::prelude::*;
use rivet_types::actors::CrashPolicy;

use crate::test_cluster::TestCluster;

use super::Scenario;

/// Scenario that creates a sleeping actor v1 before envoys were introduced. When loaded after envoys
/// introduced the wf should not die when awoken.
pub struct PbActorV1PreMigration;

#[async_trait(?Send)]
impl Scenario for PbActorV1PreMigration {
	fn name(&self) -> &'static str {
		"pb-actor-v1-pre-migration"
	}

	fn replica_count(&self) -> usize {
		2
	}

	async fn populate(&self, cluster: &TestCluster) -> Result<()> {
		let ctx = cluster.get_ctx(cluster.leader_id());

		let existing_namespace = ctx
			.op(namespace::ops::resolve_for_name_local::Input {
				name: "default".to_string(),
			})
			.await?
			.context("default ns should exist")?;

		let actor_id = Id::new_v1(ctx.config().dc_label());

		ctx.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: existing_namespace.namespace_id,
			name: "test".to_string(),
			key: None,
			runner_name_selector: "default".to_string(),
			input: None,
			crash_policy: CrashPolicy::Sleep,
			start_immediately: true,
			create_ts: None,
			forward_request: false,
			datacenter_name: None,
		})
		.await?;

		// Wait for wf to sleep
		tokio::time::sleep(Duration::from_secs(5)).await;

		Ok(())
	}
}
