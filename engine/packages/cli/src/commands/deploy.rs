use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use clap::Parser;
use serde_json::{Value, json};

use crate::{
	DEFAULT_NAMESPACE,
	auth::Auth,
	cloud::{
		CloudClient, create_or_update_pool, create_registry_token, dashboard_endpoint,
		ensure_namespace, pool_exists, registry_endpoint, revoke_api_token, v2, wait_for_pool,
	},
	cluster_env::parse_set_args,
	credentials::store_cloud_token,
	target::{CloudArgs, ClusterTarget, ProjectTarget, Target},
	util::{
		build_resources, build_runner_config, default_image_tag, docker_build, docker_login,
		encode, parse_env_vars, run_command,
	},
};

pub(crate) mod byoc;
pub(crate) mod cluster;

#[derive(Parser)]
pub struct Opts {
	#[command(flatten)]
	cloud: CloudArgs,
	/// Cloud namespace to deploy to. Defaults to `production`. Projects only.
	#[arg(long)]
	namespace: Option<String>,
	/// Compute pool to deploy to. On a cluster, the runner pool.
	#[arg(long, default_value = crate::POOL_NAME)]
	pool: String,
	/// Cluster region to deploy to. Managed clusters only, where it defaults to
	/// the cluster's only region. Refused for a project and for a BYOC cluster,
	/// which runs in one region.
	#[arg(long)]
	region: Option<String>,
	/// Roll out an existing build instead of building and uploading a new one.
	/// Clusters only.
	#[arg(long)]
	build: Option<String>,
	/// Dockerfile to build.
	#[arg(long, default_value = "Dockerfile")]
	dockerfile: PathBuf,
	/// Docker build context.
	#[arg(long, default_value = ".")]
	build_context: PathBuf,
	/// Environment override, repeatable as KEY=VAL. On a managed cluster, sets
	/// these runner pool variables and keeps the pool's others. On a BYOC
	/// cluster, replaces the pool's variables; without it they are kept.
	#[arg(long = "env")]
	env_vars: Vec<String>,
	/// A runner pool variable taking a cluster secret's latest version,
	/// repeatable as KEY (a secret named like the variable) or
	/// KEY=SECRET_NAME, and kept with the pool's others. Managed clusters only;
	/// set the secret first with `rivet cluster secret set`.
	#[arg(long = "secret", value_name = "KEY[=SECRET_NAME]")]
	secrets: Vec<String>,
	/// Skip prompts.
	#[arg(long)]
	yes: bool,
	/// Image repository name in Rivet's registry. Defaults to the project slug.
	#[arg(long)]
	image: Option<String>,
	/// Image tag. Defaults to the current git short SHA, or a timestamp outside git.
	#[arg(long)]
	tag: Option<String>,
	/// vCPUs. Project: per actor, 1, 2, 4 or 8, or a value in [0.08, 1) with at
	/// most two decimals; defaults to 1 server-side. BYOC cluster: per actor
	/// node, any positive number such as 0.5 or 2; optional, keeps the pool's
	/// current value, or 1 vCPU for a new pool. Refused for a managed cluster.
	#[arg(long)]
	cpu: Option<f64>,
	/// Memory. Project: per actor, <number>Mi or <number>Gi between 512Mi and
	/// 4Gi; defaults to 512Mi server-side. BYOC cluster: per actor node, a
	/// Kubernetes quantity such as 2Gi; optional, keeps the pool's current
	/// value, or 2 GiB for a new pool. Refused for a managed cluster.
	#[arg(long)]
	memory: Option<String>,
	/// Fewest actor nodes the pool runs, 1 to 1000. Without it the pool keeps
	/// its current value, or 1 for a new pool. BYOC clusters only.
	#[arg(long)]
	min_replicas: Option<u32>,
	/// Most actor nodes the pool scales to, 1 to 1000. Without it the pool
	/// keeps its current value, raised to --min-replicas when below it, or
	/// --min-replicas for a new pool. BYOC clusters only.
	#[arg(long)]
	max_replicas: Option<u32>,
	/// A pool variable read from a Kubernetes Secret in the cluster, repeatable
	/// as KEY=SECRET/KEY. Replaces the pool's secret variables; without it they
	/// are kept. BYOC clusters only.
	#[arg(long = "secret-env", value_name = "KEY=SECRET/KEY")]
	secret_env: Vec<String>,
	/// The Engine's configuration of the pool as a JSON object, replacing the
	/// pool's. BYOC clusters only.
	#[arg(long, value_name = "JSON")]
	pool_config: Option<String>,
	/// Minimum number of actors to keep running. Range 0 to 100. Defaults to 0 server-side.
	#[arg(long)]
	min_scale: Option<u32>,
	/// Maximum number of actors to scale to. Range 1 to 5000. Defaults to 1 server-side.
	#[arg(long)]
	max_scale: Option<u32>,
	/// Maximum number of concurrent actors the pool runs. Range 1 to 50000.
	/// Defaults to 1000 server-side.
	#[arg(long)]
	max_concurrent_actors: Option<u32>,
	/// Seconds a draining runner instance waits for running actors to finish
	/// before stopping. Range 5 to 3200. Defaults to 1800 server-side.
	#[arg(long)]
	drain_grace_period: Option<u32>,
	/// Whether deploying a new version drains runner instances on the old
	/// version. Pass --drain-on-version-upgrade=false to disable. Defaults to
	/// true server-side.
	#[arg(long, num_args = 0..=1, default_missing_value = "true")]
	drain_on_version_upgrade: Option<bool>,
	/// Number of concurrent requests each actor instance handles. Range 1 to 500.
	/// Defaults to 80 server-side.
	#[arg(long)]
	instance_request_concurrency: Option<u32>,
	/// Reuse the image already deployed to the pool instead of building and
	/// pushing a new one. Skips the Docker build/push and leaves the pool's image
	/// unchanged, which is useful for updating resources without a rebuild.
	#[arg(long)]
	reuse_image: bool,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		if let Some(token) = &self.cloud.token {
			store_cloud_token(token)?;
		}

		if !self.reuse_image && self.build.is_none() && !self.dockerfile.exists() {
			bail!("Dockerfile not found: {}", self.dockerfile.display());
		}

		let (cloud, auth, target) = self.cloud.connect().await?;
		match target {
			Target::Project(project) => {
				reject_flags(
					"project",
					&[
						("--region", self.region.is_some()),
						("--build", self.build.is_some()),
						("--secret", !self.secrets.is_empty()),
						("--min-replicas", self.min_replicas.is_some()),
						("--max-replicas", self.max_replicas.is_some()),
						("--secret-env", !self.secret_env.is_empty()),
						("--pool-config", self.pool_config.is_some()),
					],
				)?;

				// Validate compute resources client-side before doing any build work.
				let resources = build_resources(
					self.cpu,
					self.memory.as_deref(),
					self.min_scale,
					self.max_scale,
					self.instance_request_concurrency,
				)?;
				let runner_config = build_runner_config(
					self.max_concurrent_actors,
					self.drain_grace_period,
					self.drain_on_version_upgrade,
				)?;
				let env = parse_env_vars(&self.env_vars)?;
				self.deploy_project(cloud, auth, project, env, resources, runner_config)
					.await
			}
			Target::Cluster(ClusterTarget { cluster, .. }) => {
				if v2::is_byoc_cluster(&cloud, &cluster).await? {
					return self.deploy_byoc(&cloud, cluster).await;
				}

				reject_flags(
					"managed cluster",
					&[
						("--namespace", self.namespace.is_some()),
						("--cpu", self.cpu.is_some()),
						("--memory", self.memory.is_some()),
						("--min-replicas", self.min_replicas.is_some()),
						("--max-replicas", self.max_replicas.is_some()),
						("--secret-env", !self.secret_env.is_empty()),
						("--pool-config", self.pool_config.is_some()),
						("--min-scale", self.min_scale.is_some()),
						("--max-scale", self.max_scale.is_some()),
						(
							"--max-concurrent-actors",
							self.max_concurrent_actors.is_some(),
						),
						("--drain-grace-period", self.drain_grace_period.is_some()),
						(
							"--drain-on-version-upgrade",
							self.drain_on_version_upgrade.is_some(),
						),
						(
							"--instance-request-concurrency",
							self.instance_request_concurrency.is_some(),
						),
						("--reuse-image", self.reuse_image),
					],
				)?;
				let env = parse_set_args(&self.env_vars, &self.secrets)?;
				cluster::deploy(
					&cloud,
					cluster::ClusterDeploy {
						cluster,
						pool: self.pool,
						region: self.region,
						build: self.build,
						dockerfile: self.dockerfile,
						build_context: self.build_context,
						image: self.image,
						tag: self.tag,
						env,
					},
				)
				.await
			}
		}
	}

	async fn deploy_byoc(self, cloud: &CloudClient, cluster: String) -> Result<()> {
		reject_flags("BYOC cluster", &self.byoc_refused_flags())?;
		byoc::validate_pool_name(&self.pool)?;
		let pool_apply = byoc::pool_settings(
			self.cpu,
			self.memory.as_deref(),
			self.min_replicas,
			self.max_replicas,
			&self.env_vars,
			&self.secret_env,
			self.pool_config.as_deref(),
		)?;
		byoc::deploy(
			cloud,
			byoc::ByocDeploy {
				cluster,
				pool: self.pool,
				build: self.build,
				dockerfile: self.dockerfile,
				build_context: self.build_context,
				image: self.image,
				tag: self.tag,
				pool_apply,
			},
		)
		.await
	}

	/// The flags that are set but mean nothing on a BYOC cluster: a project's
	/// pool settings, and a managed cluster's region and Rivet-stored secrets.
	fn byoc_refused_flags(&self) -> Vec<(&'static str, bool)> {
		vec![
			("--namespace", self.namespace.is_some()),
			("--region", self.region.is_some()),
			("--secret (use --secret-env)", !self.secrets.is_empty()),
			("--min-scale (use --min-replicas)", self.min_scale.is_some()),
			("--max-scale (use --max-replicas)", self.max_scale.is_some()),
			(
				"--max-concurrent-actors",
				self.max_concurrent_actors.is_some(),
			),
			("--drain-grace-period", self.drain_grace_period.is_some()),
			(
				"--drain-on-version-upgrade",
				self.drain_on_version_upgrade.is_some(),
			),
			(
				"--instance-request-concurrency",
				self.instance_request_concurrency.is_some(),
			),
			("--reuse-image", self.reuse_image),
		]
	}

	async fn deploy_project(
		self,
		cloud: CloudClient,
		auth: Auth,
		ProjectTarget {
			org: organization,
			project,
		}: ProjectTarget,
		env_map: BTreeMap<String, String>,
		resources: Option<Value>,
		runner_config: Option<Value>,
	) -> Result<()> {
		let namespace = ensure_namespace(
			&cloud,
			&project,
			&organization,
			self.namespace.as_deref().unwrap_or(DEFAULT_NAMESPACE),
		)
		.await?;

		// Surface the pool in logs only when it is not the default, so the common
		// single-pool case stays uncluttered. A `None` field is omitted by tracing.
		let pool_field = (self.pool != crate::POOL_NAME).then_some(self.pool.as_str());

		let dashboard = dashboard_endpoint(&self.cloud.cloud_api)?;
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
				pool = pool_field,
				reuse_image = self.reuse_image,
				"deploying"
			);
		}

		if self.reuse_image {
			// Reusing an image requires a pool that already has one; do not enable
			// a fresh pool here.
			if !pool_exists(&cloud, &project, &organization, &namespace.name, &self.pool).await? {
				bail!(
					"cannot reuse image: no managed pool exists for this namespace, deploy an image first"
				);
			}
		} else {
			tracing::info!(pool = pool_field, "enabling managed pool");
			create_or_update_pool(
				&cloud,
				&project,
				&organization,
				&namespace.name,
				&self.pool,
				json!({ "displayName": self.pool }),
			)
			.await?;
			wait_for_pool(
				&cloud,
				&project,
				&organization,
				&namespace.name,
				&self.pool,
				false,
			)
			.await?;
		}

		// Build and push a new image unless reusing the image already deployed to
		// the pool. When reusing, the `image` field is omitted from the upsert so
		// the pool keeps its current build. Sending `null` would instead clear it.
		let image = if self.reuse_image {
			tracing::info!(
				pool = pool_field,
				"reusing existing pool image, skipping build"
			);
			None
		} else {
			let registry = registry_endpoint(&self.cloud.cloud_api)?;
			let image_name = self.image.unwrap_or_else(|| project.clone());
			let tag = self.tag.unwrap_or_else(default_image_tag);
			let image_ref = format!("{registry}/{image_name}:{tag}");

			// The registry only accepts Cloud API tokens, so a login session
			// pushes with a short-lived token that is revoked afterwards.
			let registry_token = if auth.is_login() {
				Some(create_registry_token(&cloud, &project, &organization).await?)
			} else {
				None
			};
			let pushed = build_and_push(
				&registry,
				registry_token
					.as_ref()
					.map_or(auth.token.as_str(), |token| token.token.as_str()),
				&self.build_context,
				&self.dockerfile,
				&image_ref,
			);
			if let Some(token) = &registry_token {
				if let Err(error) =
					revoke_api_token(&cloud, &project, &organization, &token.id).await
				{
					tracing::warn!(
						?error,
						"could not revoke the temporary registry token; it expires on its own"
					);
				}
			}
			pushed?;

			Some((image_name, tag))
		};

		tracing::info!(pool = pool_field, "upserting managed pool");
		let mut pool_body = json!({
			"displayName": self.pool,
		});
		if let Some((image_name, tag)) = image {
			pool_body["image"] = json!({
				"repository": image_name,
				"tag": tag,
			});
		}
		if !env_map.is_empty() {
			pool_body["environment"] = serde_json::to_value(env_map)?;
		}
		if let Some(runner_config) = runner_config {
			pool_body["runnerConfig"] = runner_config;
		}
		if let Some(resources) = resources {
			pool_body["resources"] = resources;
		}
		create_or_update_pool(
			&cloud,
			&project,
			&organization,
			&namespace.name,
			&self.pool,
			pool_body,
		)
		.await?;
		wait_for_pool(
			&cloud,
			&project,
			&organization,
			&namespace.name,
			&self.pool,
			true,
		)
		.await?;

		// The dashboard URL is the command's result; print it to stdout so it
		// can be captured by scripts.
		println!("{dashboard_url}");
		Ok(())
	}
}

fn reject_flags(target: &str, flags: &[(&str, bool)]) -> Result<()> {
	let set: Vec<&str> = flags
		.iter()
		.filter(|(_, set)| *set)
		.map(|(name, _)| *name)
		.collect();
	if set.is_empty() {
		return Ok(());
	}
	bail!(
		"{} cannot be used when deploying to a {target}",
		set.join(", ")
	)
}

fn build_and_push(
	registry: &str,
	token: &str,
	build_context: &Path,
	dockerfile: &Path,
	image_ref: &str,
) -> Result<()> {
	tracing::info!("logging in to Rivet registry");
	docker_login(registry, token)?;

	tracing::info!("building Docker image");
	docker_build(build_context, dockerfile, image_ref)?;

	tracing::info!("pushing Docker image");
	run_command("docker", &["push", image_ref], None)
}
