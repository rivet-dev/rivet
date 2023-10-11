use derive_builder::Builder;
use maplit::hashmap;
use std::collections::HashMap;

use crate::context::ProjectContext;

/// Defines the dependency graph for the Terraform plans.
///
/// This is used to automatically generate `terraform_remote_state` blocks
/// for each Terraform plan with the correct state backend.
pub fn dependency_graph(_ctx: &ProjectContext) -> HashMap<&'static str, Vec<RemoteState>> {
	hashmap! {
		"dns" => vec![RemoteStateBuilder::default().plan_id("pools").build().unwrap(), RemoteStateBuilder::default().plan_id("k8s_infra").build().unwrap()],
		"redis_aws" => vec![
			RemoteStateBuilder::default().plan_id("k8s_cluster_aws").build().unwrap()
		],
		"cockroachdb_managed" => vec![
			RemoteStateBuilder::default().plan_id("k8s_cluster_aws").build().unwrap()
		],
		"clickhouse_managed" => vec![
			RemoteStateBuilder::default().plan_id("k8s_cluster_aws").build().unwrap()
		],
		"cloudflare_workers" => vec![
			RemoteStateBuilder::default().plan_id("dns").build().unwrap()
		],
	}
}

/// Specifies a remote dependency from one Terraform plan to another.
#[derive(Clone, Builder)]
#[builder(setter(into))]
pub struct RemoteState {
	/// The remote plan ID to import.
	pub plan_id: &'static str,

	/// The name of the data blog.
	#[builder(setter(strip_option), default)]
	pub data_name: Option<&'static str>,

	/// Condition for whether or not to include the remote state.
	///
	/// This will add a `count` under the hood.
	#[builder(setter(strip_option), default)]
	pub condition: Option<String>,
}

impl RemoteState {
	pub fn data_name(&self) -> &'static str {
		self.data_name.unwrap_or(self.plan_id)
	}
}
