use anyhow::*;
use serde_json::{json, Value};

use crate::{
	config::{self, service::RuntimeKind},
	context::{ProjectContext, S3Provider},
	dep,
};

#[derive(Clone, Debug, Default)]
pub struct BuildOpts {
	/// Skips providing S3 context. This is needed if applying partial plans before the S3 Terraform plan has applied.
	pub skip_s3: bool,
}

/// Generates a config that will be exposed to Salt.
pub async fn build(ctx: &ProjectContext, opts: &BuildOpts) -> Result<Value> {
	let mut vars = json!({});

	vars["namespace"] = json!(ctx.ns_id());
	match &ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode {
			restrict_service_resources,
			..
		} => {
			vars["deploy"] = json!({
				"local": {
					"backend_repo_path": ctx.path(),
					"restrict_service_resources": restrict_service_resources,
				},
			});
		}
		config::ns::ClusterKind::Distributed { .. } => {
			vars["deploy"] = json!({
				"cluster": {},
			});
		}
	}
	vars["domain"] = json!({
		"base": ctx.domain_main(),
		"game": ctx.domain_job(),
		"job": ctx.domain_job(),
	});
	vars["primary_region"] = json!(ctx.primary_region_or_local());

	vars["leader_count"] = json!(ctx.leader_count());

	vars["pools"] = json!(crate::dep::terraform::pools::build_pools(ctx).await?);

	vars["cloudflare"] = cloudflare(ctx)?;

	vars["s3"] = s3(ctx, opts.skip_s3).await?;

	vars["redis"] = redis(ctx).await?;

	vars["traefik"] = traefik(ctx)?;

	Ok(vars)
}

async fn s3(ctx: &ProjectContext, skip: bool) -> Result<Value> {
	let mut res = serde_json::Map::with_capacity(1);

	if skip {
		// Provide filler values so the pillars can still render
		res.insert(
			"default".to_string(),
			json!({
				"endpoint_internal": "",
				"endpoint_external": "",
				"region": "",
			}),
		);

		return Ok(res.into());
	}

	let (default_provider, _) = ctx.default_s3_provider()?;
	let default_s3_config = ctx.s3_config(default_provider).await?;
	res.insert(
		"default".to_string(),
		json!({
			"endpoint_internal": default_s3_config.endpoint_internal,
			"endpoint_external": default_s3_config.endpoint_external,
			"region": default_s3_config.region,
		}),
	);

	let providers = &ctx.ns().s3.providers;
	if providers.minio.is_some() {
		let s3_config = ctx.s3_config(S3Provider::Minio).await?;
		res.insert(
			"minio".to_string(),
			json!({
				"endpoint_internal": s3_config.endpoint_internal,
				"endpoint_external": s3_config.endpoint_external,
				"region": s3_config.region,
			}),
		);
	}
	if providers.backblaze.is_some() {
		let s3_config = ctx.s3_config(S3Provider::Backblaze).await?;
		res.insert(
			"backblaze".to_string(),
			json!({
				"endpoint_internal": s3_config.endpoint_internal,
				"endpoint_external": s3_config.endpoint_external,
				"region": s3_config.region,
			}),
		);
	}
	if providers.aws.is_some() {
		let s3_config = ctx.s3_config(S3Provider::Aws).await?;
		res.insert(
			"aws".to_string(),
			json!({
				"endpoint_internal": s3_config.endpoint_internal,
				"endpoint_external": s3_config.endpoint_external,
				"region": s3_config.region,
			}),
		);
	}

	Ok(res.into())
}

fn cloudflare(ctx: &ProjectContext) -> Result<Value> {
	#[allow(irrefutable_let_patterns)]
	let config::ns::DnsProvider::Cloudflare { access, .. } = &ctx.ns().dns.provider else {
		return Ok(json!(null));
	};

	let access = if access.is_some() {
		json!({})
	} else {
		json!(null)
	};

	Ok(json!({
		"access": access,
	}))
}

async fn redis(ctx: &ProjectContext) -> Result<Value> {
	let mut dbs = json!({});

	for redis_dep in ctx.all_services().await {
		let (persistent, index) = match redis_dep.config().runtime {
			RuntimeKind::Redis { persistent, index } => (persistent, index),
			_ => continue,
		};

		let port = dep::redis::server_port(redis_dep);

		dbs[redis_dep.name()] = json!({
			"index": index,
			"port": port,
			"persistent": persistent,
		});
	}

	Ok(json!({
		"dbs": dbs,
	}))
}

fn traefik(ctx: &ProjectContext) -> Result<Value> {
	Ok(json!({
		"log_level": ctx.ns().traefik.log_level,
		"access_logs": ctx.ns().traefik.access_logs,
	}))
}
