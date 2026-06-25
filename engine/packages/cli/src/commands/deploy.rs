use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;
use reqwest::Method;
use serde_json::json;

use crate::{
	DEFAULT_CLOUD_API, DEFAULT_NAMESPACE,
	cloud::{
		CloudClient, TokenInspectResponse, create_or_update_pool, dashboard_endpoint,
		ensure_namespace, registry_endpoint, wait_for_pool,
	},
	credentials::{resolve_token, write_credentials},
	util::{
		build_resources, default_image_tag, docker_build, docker_login, encode, parse_env_vars,
		run_command,
	},
};

#[derive(Parser)]
pub struct Opts {
	/// Rivet Cloud API token. Also writes ~/.rivet/credentials for later commands.
	#[arg(long)]
	token: Option<String>,
	/// Cloud namespace to deploy to.
	#[arg(long, default_value = DEFAULT_NAMESPACE)]
	namespace: String,
	/// Override project from /tokens/api/inspect.
	#[arg(long)]
	project: Option<String>,
	/// Override organization from /tokens/api/inspect.
	#[arg(long)]
	org: Option<String>,
	/// Dockerfile to build.
	#[arg(long, default_value = "Dockerfile")]
	dockerfile: PathBuf,
	/// Docker build context.
	#[arg(long, default_value = ".")]
	build_context: PathBuf,
	/// Environment override, repeatable as KEY=VAL.
	#[arg(long = "env")]
	env_vars: Vec<String>,
	/// Skip prompts.
	#[arg(long)]
	yes: bool,
	/// Cloud API endpoint.
	#[arg(long, default_value = DEFAULT_CLOUD_API)]
	cloud_api: String,
	/// Image repository name in Rivet's registry. Defaults to the project slug.
	#[arg(long)]
	image: Option<String>,
	/// Image tag. Defaults to the current git short SHA, or a timestamp outside git.
	#[arg(long)]
	tag: Option<String>,
	/// vCPU count per actor. Range 0.08 to 8; must be 1, 2, 4, or 8, or a value
	/// in [0.08, 1) with at most two decimals. Defaults to 1 server-side.
	#[arg(long)]
	cpu: Option<f64>,
	/// Memory per actor as <number>Mi or <number>Gi, between 512Mi and 4Gi.
	/// Defaults to 512Mi server-side.
	#[arg(long)]
	memory: Option<String>,
	/// Minimum number of actors to keep running. Range 0 to 100. Defaults to 0 server-side.
	#[arg(long)]
	min_scale: Option<u32>,
	/// Maximum number of actors to scale to. Range 1 to 500. Defaults to 1 server-side.
	#[arg(long)]
	max_scale: Option<u32>,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let token = resolve_token(self.token.as_deref())?;
		if let Some(token) = &self.token {
			write_credentials(token)?;
		}

		if !self.dockerfile.exists() {
			bail!("Dockerfile not found: {}", self.dockerfile.display());
		}

		// Validate compute resources client-side before doing any build work.
		let resources = build_resources(
			self.cpu,
			self.memory.as_deref(),
			self.min_scale,
			self.max_scale,
		)?;

		let cloud = CloudClient::new(&self.cloud_api, token.clone())?;
		tracing::info!("inspecting Rivet Cloud token");
		let inspect: TokenInspectResponse = cloud
			.request(Method::GET, "/tokens/api/inspect", None)
			.await?
			.context("token inspect returned no body")?;
		let project = self.project.unwrap_or(inspect.project);
		let organization = self.org.unwrap_or(inspect.organization);
		let namespace = ensure_namespace(&cloud, &project, &organization, &self.namespace).await?;

		let registry = registry_endpoint(&self.cloud_api)?;
		let dashboard = dashboard_endpoint(&self.cloud_api)?;
		let image_name = self.image.unwrap_or_else(|| project.clone());
		let tag = self.tag.unwrap_or_else(default_image_tag);
		let image_ref = format!("{registry}/{image_name}:{tag}");
		let dashboard_url = format!(
			"{dashboard}/orgs/{}/projects/{}/ns/{}?skipOnboarding=1",
			encode(&organization),
			encode(&project),
			encode(&namespace.name)
		);

		if !self.yes {
			tracing::info!(
				context = %self.build_context.display(),
				%project,
				namespace = %namespace.name,
				image = %image_ref,
				"deploying"
			);
		}

		tracing::info!("enabling managed pool");
		create_or_update_pool(
			&cloud,
			&project,
			&organization,
			&namespace.name,
			json!({ "displayName": "Default" }),
		)
		.await?;
		wait_for_pool(&cloud, &project, &organization, &namespace.name, false).await?;

		tracing::info!("logging in to Rivet registry");
		docker_login(&registry, &token)?;

		tracing::info!("building Docker image");
		docker_build(&self.build_context, &self.dockerfile, &image_ref)?;

		tracing::info!("pushing Docker image");
		run_command("docker", &["push", &image_ref], None)?;

		tracing::info!("upserting managed pool");
		let mut pool_body = json!({
			"displayName": "Default",
			"maxConcurrentActors": 1000,
			"image": {
				"repository": image_name,
				"tag": tag,
			},
		});
		let env_map = parse_env_vars(&self.env_vars)?;
		if !env_map.is_empty() {
			pool_body["environment"] = serde_json::to_value(env_map)?;
		}
		if let Some(resources) = resources {
			pool_body["resources"] = resources;
		}
		create_or_update_pool(&cloud, &project, &organization, &namespace.name, pool_body).await?;
		wait_for_pool(&cloud, &project, &organization, &namespace.name, true).await?;

		// The dashboard URL is the command's result; print it to stdout so it
		// can be captured by scripts.
		println!("{dashboard_url}");
		Ok(())
	}
}
