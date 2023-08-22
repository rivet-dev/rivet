use derive_builder::Builder;
use maplit::hashmap;
use std::collections::HashMap;

use crate::context::{ProjectContext, S3Provider};

/// Defines the dependency graph for the Terraform plans.
///
/// This is used to automatically generate `terraform_remote_state` blocks
/// for each Terraform plan with the correct state backend.
pub fn dependency_graph(ctx: &ProjectContext) -> HashMap<&'static str, Vec<RemoteState>> {
	hashmap! {
		"dns" => vec![RemoteStateBuilder::default().plan_id("pools").build().unwrap()],
		"master_local" => vec![RemoteStateBuilder::default().plan_id("nebula").build().unwrap()],
		"master_cluster" => vec![RemoteStateBuilder::default().plan_id("nebula").build().unwrap()],
		"nomad" => {
			let (default_s3_provider, _) = ctx.default_s3_provider().unwrap();
			let provider_plan_id = match default_s3_provider {
				S3Provider::Minio => "s3_minio",
				S3Provider::Backblaze => "s3_backblaze",
				S3Provider::Aws => "s3_aws",
			};

			vec![RemoteStateBuilder::default()
			.plan_id(provider_plan_id)
			.data_name("s3")
			.build()
			.unwrap()]
		},
		"pools" => vec![
			RemoteStateBuilder::default().plan_id("nebula").build().unwrap(),
			RemoteStateBuilder::default().plan_id("master_local").condition("var.deploy_method_local").build().unwrap(),
			RemoteStateBuilder::default().plan_id("master_cluster").condition("var.deploy_method_cluster").build().unwrap(),
		],
	}
}

/// Specifies a remote dependency from one Terraform plan to another.
#[derive(Builder)]
#[builder(setter(into))]
pub struct RemoteState {
	/// The remote plan ID to import.
	pub plan_id: &'static str,

	/// The name of the data blog.
	#[builder(setter(strip_option), default)]
	pub data_name: Option<&'static str>,

	/// Condition for wether or not to include the remote sate.
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
