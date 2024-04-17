use serde::Deserialize;
use std::{collections::HashMap, ops::Deref};

use crate::{config, context::ProjectContext};

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
pub struct Cert {
	pub cert_pem: String,
	pub key_pem: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct K8sInfra {
	pub traefik_tunnel_external_ip: TerraformOutputValue<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tls {
	pub tls_cert_locally_signed_tunnel_server: TerraformOutputValue<Cert>,
	pub tls_cert_locally_signed_job: TerraformOutputValue<Cert>,
	pub tls_cert_locally_signed_gg: TerraformOutputValue<Cert>,
	pub root_ca_cert_pem: TerraformOutputValue<String>,
	pub acme_account_private_key_pem: TerraformOutputValue<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dns {
	pub cloudflare_zone_ids: TerraformOutputValue<DnsZones>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DnsZones {
	pub main: String,
	pub cdn: String,
	pub job: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KubernetesClusterAws {
	pub eks_admin_role_arn: TerraformOutputValue<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cockroach {
	pub host: TerraformOutputValue<String>,
	pub port: TerraformOutputValue<u32>,
	pub cluster_identifier: TerraformOutputValue<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClickHouse {
	pub host: TerraformOutputValue<String>,
	pub port_https: TerraformOutputValue<u32>,
	pub port_native_secure: TerraformOutputValue<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Redis {
	pub host: TerraformOutputValue<HashMap<String, String>>,
	pub port: TerraformOutputValue<HashMap<String, u32>>,
}

pub async fn read_k8s_infra(ctx: &ProjectContext) -> K8sInfra {
	read_plan::<K8sInfra>(ctx, "k8s_infra").await
}

pub async fn read_tls(ctx: &ProjectContext) -> Tls {
	read_plan::<Tls>(ctx, "tls").await
}

pub async fn read_dns(ctx: &ProjectContext) -> Dns {
	read_plan::<Dns>(ctx, "dns").await
}

pub async fn read_k8s_cluster_aws(ctx: &ProjectContext) -> KubernetesClusterAws {
	read_plan::<KubernetesClusterAws>(ctx, "k8s_cluster_aws").await
}

pub async fn read_crdb(ctx: &ProjectContext) -> Cockroach {
	match &ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			read_plan::<Cockroach>(ctx, "cockroachdb_k8s").await
		}
		config::ns::ClusterKind::Distributed { .. } => {
			read_plan::<Cockroach>(ctx, "cockroachdb_managed").await
		}
	}
}

pub async fn read_clickhouse(ctx: &ProjectContext) -> ClickHouse {
	match &ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			read_plan::<ClickHouse>(ctx, "clickhouse_k8s").await
		}
		config::ns::ClusterKind::Distributed { .. } => {
			read_plan::<ClickHouse>(ctx, "clickhouse_managed").await
		}
	}
}

pub async fn read_redis(ctx: &ProjectContext) -> Redis {
	match &ctx.ns().redis.provider {
		config::ns::RedisProvider::Kubernetes { .. } => read_plan::<Redis>(ctx, "redis_k8s").await,
		config::ns::RedisProvider::Aws { .. } => read_plan::<Redis>(ctx, "redis_aws").await,
		config::ns::RedisProvider::Aiven { .. } => read_plan::<Redis>(ctx, "redis_aiven").await,
	}
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
