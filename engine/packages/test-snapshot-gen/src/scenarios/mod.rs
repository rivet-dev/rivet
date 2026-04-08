use anyhow::Result;
use async_trait::async_trait;

use crate::test_cluster::TestCluster;

mod epoxy_keys;
mod pb_actor_v1_pre_migration;

#[async_trait(?Send)]
pub trait Scenario {
	/// Unique name for this scenario (used as directory name).
	fn name(&self) -> &'static str;

	/// Number of replicas in the cluster.
	fn replica_count(&self) -> usize;

	/// Populate state in the cluster. Called after the cluster is fully
	/// initialized with all replicas active.
	async fn populate(&self, cluster: &TestCluster) -> Result<()>;
}

pub fn all() -> Vec<Box<dyn Scenario>> {
	vec![
		Box::new(epoxy_keys::EpoxyKeys),
		Box::new(pb_actor_v1_pre_migration::PbActorV1PreMigration),
	]
}
