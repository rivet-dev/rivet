use serde::Deserialize;
use std::{collections::HashMap, ops::Deref};

use crate::context::ProjectContext;

#[derive(Debug, Clone, Deserialize)]
pub struct TerraformOutputValue<T> {
	pub value: T,
}

impl<T> Deref for TerraformOutputValue<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct MasterCluster {
	pub salt_master_host: TerraformOutputValue<String>,
	pub nebula_lighthouse_host: TerraformOutputValue<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pools {
	pub servers: TerraformOutputValue<HashMap<String, PoolServer>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolServer {
	// Server
	pub region_id: String,
	pub pool_id: String,
	pub version_id: String,
	pub index: usize,

	// New data
	pub public_ipv4: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cert {
	pub cert_pem: String,
	pub key_pem: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tls {
	pub tls_cert_cloudflare_rivet_gg: TerraformOutputValue<Cert>,
}

pub async fn read_master_cluster(ctx: &ProjectContext) -> MasterCluster {
	read_plan::<MasterCluster>(ctx, "master_cluster").await
}

pub async fn read_pools(ctx: &ProjectContext) -> Pools {
	read_plan::<Pools>(ctx, "pools").await
}

pub async fn read_tls_cert(ctx: &ProjectContext) -> Tls {
	read_plan::<Tls>(ctx, "tls").await
}

/// Reads a Terraform plan's output and decodes in to type.
pub async fn read_plan<T: serde::de::DeserializeOwned>(ctx: &ProjectContext, plan_id: &str) -> T {
	let terraform_plans = crate::tasks::infra::all_terraform_plans(ctx).unwrap();
	assert!(
		terraform_plans.iter().any(|x| x == plan_id),
		"reading terraform output not in plan: {}",
		plan_id
	);

	// Read the Terraform
	let output_raw = if let Some(x) = ctx
		.cache(|cache| {
			cache
				.terraform_output_cache
				.get(ctx.ns_id())
				.and_then(|x| x.get(plan_id))
				.cloned()
		})
		.await
	{
		// eprintln!("  * Reading Terraform output (cached)");
		x
	} else {
		// eprintln!("  * Reading Terraform output");
		let output_raw = super::cli::output(ctx, plan_id, true).await;
		ctx.cache_mut(|cache| {
			cache
				.terraform_output_cache
				.entry(ctx.ns_id().into())
				.or_default()
				.insert(plan_id.into(), output_raw.clone())
		})
		.await;
		output_raw
	};

	let output = serde_json::from_value::<T>(output_raw).expect("invalid terraform output");

	output
}

/// Clears the cached output for a Terraform plan. This should be called any time the output of the `terraform output` command will change.
pub async fn clear_cache(ctx: &ProjectContext, plan_id: &str) {
	ctx.cache_mut(|cache| {
		cache
			.terraform_output_cache
			.get_mut(ctx.ns_id())
			.map(|x| x.remove(plan_id))
	})
	.await;
}
