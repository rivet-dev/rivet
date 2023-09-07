use anyhow::{ensure, Context, Result};
use async_recursion::async_recursion;
use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tokio::{fs, process::Command, sync::RwLock};

use crate::{
	config::{
		self,
		ns::S3Provider,
		service::{RuntimeKind, ServiceKind},
	},
	context::{self, BuildContext, ProjectContext, RunContext},
	dep::{self, k8s, s3},
	utils,
};

use super::BuildOptimization;

pub type ServiceContext = Arc<ServiceContextData>;

pub struct ServiceContextData {
	project: RwLock<Weak<context::project::ProjectContextData>>,
	/// If this is overriding a service from an additional root, then this will be specified.
	overridden_service: Option<ServiceContext>,
	config: config::service::ServiceConfig,
	path: PathBuf,
	workspace_path: PathBuf,
	cargo: Option<config::service::CargoConfig>,
}

impl PartialEq for ServiceContextData {
	fn eq(&self, other: &Self) -> bool {
		self.config.service.name == other.config.service.name
	}
}

impl Hash for ServiceContextData {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.config.service.name.hash(state);
	}
}

impl ServiceContextData {
	/// Sets the reference to the project once the project context is finished initiating.
	#[async_recursion]
	pub(in crate::context) async fn set_project(
		&self,
		project: Weak<context::project::ProjectContextData>,
	) {
		*self.project.write().await = project.clone();
		if let Some(overridden_service) = &self.overridden_service {
			overridden_service.set_project(project.clone()).await;
		}
	}

	pub async fn project(&self) -> ProjectContext {
		self.project
			.read()
			.await
			.upgrade()
			.expect("missing project")
	}

	pub fn config(&self) -> &config::service::ServiceConfig {
		&self.config
	}

	pub fn path(&self) -> &Path {
		self.path.as_path()
	}

	pub fn workspace_path(&self) -> &Path {
		self.workspace_path.as_path()
	}
}

impl ServiceContextData {
	pub async fn from_path(
		project: Weak<context::project::ProjectContextData>,
		svc_ctxs_map: &HashMap<String, ServiceContext>,
		workspace_path: &Path,
		path: &Path,
	) -> Option<ServiceContext> {
		// Read config
		let config_path = path.join("Service.toml");
		let config_str = match fs::read_to_string(config_path).await {
			Ok(v) => v,
			Err(_) => return None,
		};
		let config = toml::from_str::<config::service::ServiceConfig>(&config_str)
			.expect(&format!("failed to read config: {}", path.display()));

		let cargo_path = path.join("Cargo.toml");
		let cargo = match fs::read_to_string(&cargo_path).await {
			Ok(v) => Some(
				toml::from_str::<config::service::CargoConfig>(&v).expect(&format!(
					"failed to read cargo config: {}",
					cargo_path.display()
				)),
			),
			Err(_) => None,
		};

		// Read overridden service
		let overridden_service = svc_ctxs_map.get(&config.service.name).cloned();

		Some(Arc::new(ServiceContextData::new(
			project,
			overridden_service,
			config,
			workspace_path,
			path,
			cargo,
		)))
	}

	fn new(
		project: Weak<context::project::ProjectContextData>,
		overridden_service: Option<ServiceContext>,
		config: config::service::ServiceConfig,
		workspace_path: &Path,
		path: &Path,
		cargo: Option<config::service::CargoConfig>,
	) -> ServiceContextData {
		// Build context
		let ctx = ServiceContextData {
			project: RwLock::new(project),
			overridden_service,
			config,
			path: path.to_owned(),
			workspace_path: workspace_path.to_owned(),
			cargo,
		};

		// Validate context
		ctx.validate();

		ctx
	}

	fn validate(&self) {
		// Max service name length is 63 characters (RFC 1123), so we need this
		// name to be much shorter so we can append things to it.
		assert!(
			self.config.service.name.len() <= 42,
			"name '{}' must be less than 42 characters",
			self.config.service.name
		);

		let component_class = self.config.kind.component_class();
		assert!(
			self.config
				.runtime
				.supports_component_class(&component_class),
			"runtime does not support component class"
		);

		// TODO: Validate that all services in `config.databases` are actually databases
	}
}

impl ServiceContextData {
	pub fn name(&self) -> String {
		self.config.service.name.clone()
	}

	pub fn name_snake(&self) -> String {
		self.config.service.name_snake()
	}

	pub fn name_screaming_snake(&self) -> String {
		self.config.service.name_screaming_snake()
	}

	pub fn name_camel(&self) -> String {
		self.config.service.name_camel_case()
	}

	pub fn name_single_word(&self) -> String {
		self.config.service.name_single_word()
	}

	pub fn name_matches_pattern(&self, pattern: impl AsRef<str>) -> bool {
		wildmatch::WildMatch::new(pattern.as_ref()).matches(&self.config.service.name)
	}

	pub fn cargo_name(&self) -> Option<&str> {
		self.cargo.as_ref().map(|x| x.package.name.as_str())
	}
}

impl ServiceContextData {
	pub async fn relative_path(&self) -> PathBuf {
		self.path()
			.strip_prefix(self.project().await.path())
			.expect("strip path")
			.to_owned()
	}

	pub async fn relative_gen_path(&self) -> PathBuf {
		self.gen_path()
			.await
			.strip_prefix(self.project().await.path())
			.expect("strip path")
			.to_owned()
	}

	pub async fn gen_path(&self) -> PathBuf {
		self.project()
			.await
			.gen_path()
			.join("svc")
			.join(self.name())
	}

	pub async fn gen_proto_path(&self) -> PathBuf {
		self.gen_path().await.join("proto")
	}

	pub fn migrations_path(&self) -> PathBuf {
		self.path().join("migrations")
	}

	/// Path to the executable binary.
	pub async fn rust_bin_path(&self, optimization: &BuildOptimization) -> PathBuf {
		self.project()
			.await
			.path()
			.join("target")
			.join(match optimization {
				BuildOptimization::Release => "release",
				BuildOptimization::Debug => "debug",
			})
			.join(self.cargo_name().expect("no cargo name"))
	}
}

// Deploy meta
impl ServiceContextData {
	pub fn has_migrations(&self) -> bool {
		matches!(
			self.config().runtime,
			// TODO: Add back ClickHouse
			// RuntimeKind::CRDB { .. } | RuntimeKind::ClickHouse { .. }
			RuntimeKind::CRDB { .. }
		)
	}

	pub fn crdb_db_name(&self) -> String {
		self.name_snake()
	}

	pub fn clickhouse_db_name(&self) -> String {
		self.name_snake()
	}

	pub fn redis_db_name(&self) -> String {
		self.config().service.name.clone()
	}

	pub async fn s3_bucket_name(&self) -> String {
		// Include the namespace name since it needs to be globally unique
		format!("{}-{}", self.project().await.ns_id(), self.name())
	}

	pub fn depends_on_nomad_api(&self) -> bool {
		true
		// self.name().starts_with("job-")
		// 	|| self.name().starts_with("nomad-")
		// 	|| self.name() == "api-job"
		// 	|| self.name() == "mm-lobby-create"
		// 	|| self.name() == "mm-lobby-stop"
		// 	|| self.name() == "job-gc"
		// 	|| self.name() == "nomad-monitor"
		// 	|| self.name() == "playground"
	}

	pub fn depends_on_traefik_api(&self) -> bool {
		true
		// self.name() == "job-run"
	}

	pub fn depends_on_prometheus_api(&self) -> bool {
		true
		// self.name() == "job-run-metrics-log"
	}

	pub fn depends_on_clickhouse(&self) -> bool {
		true
	}

	pub fn depends_on_jwt_key_private(&self) -> bool {
		true
		// self.name() == "token-create"
	}

	pub fn depends_on_sendgrid_key(&self) -> bool {
		true
		// self.name() == "email-send"
	}

	pub fn depends_on_s3(&self) -> bool {
		true
		// self.name().starts_with("upload-")
	}

	pub fn depends_on_s3_backfill(&self) -> bool {
		self.name() == "upload-provider-fill"
	}

	pub fn depends_on_cloudflare(&self) -> bool {
		true
		// 	|| self.name() == "cf-custom-hostname-create"
		// 	|| self.name() == "cf-custom-hostname-delete"
		// 	|| self.name() == "api-cloud"
	}

	pub fn depends_on_hcaptcha(&self) -> bool {
		// TODO:
		true
	}
}

impl ServiceContextData {
	pub fn enable_tokio_console(&self) -> bool {
		// TODO: This seems to have a significant performance impact
		false
	}
}

// Build info
#[derive(Debug, Clone)]
pub enum ServiceBuildPlan {
	/// Build the service locally.
	BuildLocally {
		/// Absolute path to the executable on the host system.
		exec_path: PathBuf,
	},

	/// Build exists on Docker.
	ExistingUploadedBuild { image_tag: String },

	/// Build the service and upload to Docker.
	BuildAndUpload { image_tag: String },
}

impl ServiceContextData {
	/// Determines if this service needs to be recompiled.
	pub async fn build_plan(
		&self,
		build_context: &BuildContext,
		force_build: bool,
	) -> Result<ServiceBuildPlan> {
		let project_ctx = self.project().await;

		match &project_ctx.ns().cluster.kind {
			// Build locally
			config::ns::ClusterKind::SingleNode { .. } => {
				// Derive the build path
				let optimization = match &build_context {
					BuildContext::Bin { optimization } => optimization,
					BuildContext::Test => &BuildOptimization::Debug,
				};
				let output_path = self.rust_bin_path(optimization).await;

				// Rust libs always attempt to rebuild (handled by cargo)
				Ok(ServiceBuildPlan::BuildLocally {
					exec_path: output_path,
				})
			}
			// Build and upload to S3
			config::ns::ClusterKind::Distributed { .. } => {
				let image_tag = self.docker_image_tag().await?;

				if !force_build {
					// TODO: Check docker if build was pushed
					// let mut cmd = Command::new("docker");
					// cmd.arg("images");
					// cmd.arg("-q").arg(&image_tag);
					// let image_exists = !cmd.output().await?.stdout.is_empty();

					// if image_exists {
					// 	return Ok(ServiceBuildPlan::ExistingUploadedBuild { image_tag });
					// }
				}

				// Default to building
				Ok(ServiceBuildPlan::BuildAndUpload { image_tag })
			}
		}
	}
}

// Dependencies
impl ServiceContextData {
	#[async_recursion]
	pub async fn dependencies(&self, run_context: RunContext) -> Vec<ServiceContext> {
		let project = self.project().await;

		let all_svcs = project.all_services().await;

		let mut dep_ctxs = Vec::<ServiceContext>::new();

		// TODO: Add dev dependencies if building for tests
		// Add operation dependencies from Cargo.toml
		//
		// You cannot depend on these services from the Service.toml, only as a Cargo dependency
		if let Some(cargo) = &self.cargo {
			let svcs = cargo
				.dependencies
				.iter()
				// Add dev dependencies for tests
				.chain(
					cargo
						.dev_dependencies
						.iter()
						.filter(|x| run_context == RunContext::Test),
				)
				.filter_map(|(name, dep)| {
					if let config::service::CargoDependency::Path { .. } = dep {
						Some(name)
					} else {
						None
					}
				})
				// Remove overridden service from deps list
				.filter(|name| {
					self.overridden_service
						.as_ref()
						.map(|osvc| &&osvc.name() != name)
						.unwrap_or(true)
				})
				.filter_map(|name| {
					all_svcs
						.iter()
						.filter(|x| x.name() == *name)
						.next()
						.cloned()
				});
			// TOOD: Use the path to find the service instead of the name. This is difficult with multiple roots.
			// .filter_map(|path| {
			// 	let absolute_path = svc_path.join(path);
			// 	all_svcs
			// 		.iter()
			// 		.filter(|x| x.path() == absolute_path)
			// 		.next()
			// 		.cloned()
			// });

			dep_ctxs.extend(svcs);
		}

		// Inherit dependencies from the service that was overridden
		if let Some(overriden_svc) = &self.overridden_service {
			dep_ctxs.extend(overriden_svc.dependencies(run_context).await);
		}

		// Check that these are services you can explicitly depend on in the Service.toml
		for dep in &dep_ctxs {
			if run_context == RunContext::Service
				&& !self.config().service.test_only
				&& dep.config().service.test_only
			{
				panic!(
					"{} -> {}: cannot depend on a `service.test-only` service outside of `test-dependencies`",
					self.name(),
					dep.name()
				);
			}

			if !matches!(
				dep.config().kind,
				ServiceKind::Database { .. }
					| ServiceKind::Cache { .. }
					| ServiceKind::Operation { .. }
			) {
				panic!(
					"{} -> {}: cannot explicitly depend on this kind of service",
					self.name(),
					dep.name()
				);
			}
		}

		dep_ctxs
	}

	pub async fn database_dependencies(
		&self,
		run_context: RunContext,
	) -> HashMap<String, config::service::Database> {
		let dbs = self
			.project()
			.await
			.recursive_dependencies(&[self.name()], run_context)
			.await
			.into_iter()
			// Filter filter services to include only operations, since these run in-process
			.filter(|svc| {
				**svc == *self || matches!(svc.config().kind, ServiceKind::Operation { .. })
			})
			// Aggregate secrets from all dependencies
			.flat_map(|x| x.config().databases.clone().into_iter())
			// Dedupe
			.collect::<HashMap<_, _>>();

		dbs
	}

	pub async fn crdb_dependencies(&self, run_context: RunContext) -> Vec<ServiceContext> {
		let dep_names = self
			.database_dependencies(run_context)
			.await
			.into_iter()
			.map(|(k, _)| k)
			.collect::<Vec<_>>();
		self.project()
			.await
			.services_with_names(&dep_names)
			.await
			.into_iter()
			.filter(|svc| matches!(svc.config().runtime, RuntimeKind::CRDB { .. }))
			.collect()
	}

	pub async fn redis_dependencies(&self, run_context: RunContext) -> Vec<ServiceContext> {
		let default_deps = ["redis-chirp".to_string(), "redis-cache".to_string()];

		let dep_names = self
			.database_dependencies(run_context)
			.await
			.into_iter()
			.map(|(k, _)| k)
			.chain(default_deps)
			.collect::<Vec<_>>();

		self.project()
			.await
			.services_with_names(&dep_names)
			.await
			.into_iter()
			.filter(|svc| matches!(svc.config().runtime, RuntimeKind::Redis { .. }))
			.collect()
	}

	pub async fn s3_dependencies(&self, run_context: RunContext) -> Vec<ServiceContext> {
		let dep_names = self
			.database_dependencies(run_context)
			.await
			.into_iter()
			.map(|(k, _)| k)
			.collect::<Vec<_>>();
		self.project()
			.await
			.services_with_names(&dep_names)
			.await
			.into_iter()
			.filter(|svc| matches!(svc.config().runtime, RuntimeKind::S3 { .. }))
			.collect()
	}

	pub async fn nats_dependencies(&self, run_context: RunContext) -> Vec<ServiceContext> {
		let dep_names = self
			.database_dependencies(run_context)
			.await
			.into_iter()
			.map(|(k, _)| k)
			.collect::<Vec<_>>();
		self.project()
			.await
			.services_with_names(&dep_names)
			.await
			.into_iter()
			.filter(|svc| matches!(svc.config().runtime, RuntimeKind::Nats { .. }))
			.collect()
	}
}

impl ServiceContextData {
	pub async fn source_hash_dev(&self, build_optimization: &BuildOptimization) -> Result<String> {
		match self.config().runtime {
			// Use binary modified timestamp for rust runtimes
			RuntimeKind::Rust { .. } => {
				let bin_ts = if let Ok(metadata) =
					fs::metadata(self.rust_bin_path(&build_optimization).await).await
				{
					metadata
						.modified()?
						.duration_since(std::time::UNIX_EPOCH)?
						.as_millis()
				} else {
					0
				};

				Ok(bin_ts.to_string())
			}
			// Use source folder hash for other runtimes
			_ => {
				if let Some(source_hash) = self.source_hash_git().await? {
					Ok(source_hash)
				} else {
					let path = self.path().to_owned();
					let modified_ts =
						tokio::task::spawn_blocking(move || utils::deep_modified_ts(&path))
							.await??;
					Ok(modified_ts.to_string())
				}
			}
		}
	}

	/// Checks if there's modifications in the svc directory before returning
	/// the source hash.
	pub async fn source_hash_git(&self) -> Result<Option<String>> {
		let git_diff_cmd = Command::new("git")
			.arg("diff-index")
			.arg("HEAD")
			.arg("--")
			.arg(self.relative_path().await.display().to_string())
			.output()
			.await?;
		assert!(git_diff_cmd.status.success());
		let git_diff = String::from_utf8(git_diff_cmd.stdout)?;

		if git_diff.trim().is_empty() {
			let source_hash = self.source_hash_git_unchecked().await?;
			let source_hash = source_hash.trim();
			if !source_hash.is_empty() {
				Ok(Some(source_hash.into()))
			} else {
				// Service is likely new and has not been committed in the past
				Ok(None)
			}
		} else {
			// Directory has changes
			Ok(None)
		}
	}

	/// Unique hash indicating the version of this service that's deployed.
	///
	/// This value doesn't have a consistent format. In development, it uses the
	/// folder's modified timestamp. In production, it uses the git hash of the
	/// directory.
	pub async fn source_hash_git_unchecked(&self) -> Result<String> {
		let git_hash_cmd = Command::new("git")
			.arg("rev-parse")
			.arg(format!("HEAD:{}", self.relative_path().await.display()))
			.output()
			.await?;
		if git_hash_cmd.status.success() {
			Ok(String::from_utf8(git_hash_cmd.stdout)?.trim()[..8].to_string())
		} else {
			// File is not committed yet
			Ok(String::new())
		}
	}

	async fn required_secrets(
		&self,
		run_context: RunContext,
	) -> Result<Vec<(Vec<String>, config::service::Secret)>> {
		let mut secrets = self
			.project()
			.await
			.recursive_dependencies(&[self.name()], run_context)
			.await
			.into_iter()
			// Filter filter services to include only operations, since these run in-process
			.filter(|x| **x == *self || matches!(x.config().kind, ServiceKind::Operation { .. }))
			// Aggregate secrets from all dependencies
			.flat_map(|x| x.config().secrets.clone().into_iter())
			// Convert keys to string array
			.map(|(k, v)| (k.split("/").map(|x| x.to_string()).collect::<Vec<_>>(), v))
			// Dedupe
			.collect::<HashMap<_, _>>()
			.into_iter()
			.collect::<Vec<_>>();

		secrets.sort_by_cached_key(|x| x.0.clone());

		Ok(secrets)
	}

	pub async fn env(
		&self,
		run_context: RunContext,
	) -> Result<(Vec<(String, String)>, Vec<utils::PortForwardConfig>)> {
		let project_ctx = self.project().await;

		let region_id = project_ctx.primary_region_or_local();
		let mut env = Vec::new();
		let mut forward_configs = Vec::new();

		// HACK: Link to dynamically linked libraries in /nix/store
		//
		// We build the binaries dynamically linked to dependencies from Nix, so
		// the output binaries are hardcoded to /nix/store.
		//
		// The `/nix/store` directory is mounted as a volume.
		if let config::ns::ClusterKind::SingleNode { .. } = project_ctx.ns().cluster.kind {
			env.push((
				"LD_LIBRARY_PATH".into(),
				std::env::var("LD_LIBRARY_PATH").context("missing LD_LIBRARY_PATH")?,
			));
		}

		// TODO: Convert this to use the bolt taint
		// TODO: This is re-running the hashing function for every service when we already did this in the planning step
		// Provide source hash to purge the cache when the service is updated
		let source_hash = self.source_hash_dev(&BuildOptimization::Debug).await?;
		env.push(("RIVET_SOURCE_HASH".into(), source_hash.clone()));

		let ns_service_config = self.ns_service_config().await;
		env.push((
			"TOKIO_WORKER_THREADS".into(),
			match ns_service_config.resources.cpu {
				config::ns::CpuResources::CpuCores(cores) => cores.max(cores),
				config::ns::CpuResources::Cpu(_) => 2,
			}
			.to_string(),
		));

		// Provide default Nomad variables if in test
		if matches!(run_context, RunContext::Test) {
			env.push(("KUBERNETES_REGION".into(), "global".into()));
			env.push(("KUBERNETES_DC".into(), region_id.clone()));
			env.push((
				"KUBERNETES_TASK_DIR".into(),
				project_ctx.gen_path().display().to_string(),
			));
		}

		// Generic context
		env.push(("RIVET_RUN_CONTEXT".into(), run_context.short().into()));
		env.push(("RIVET_NAMESPACE".into(), project_ctx.ns_id().into()));
		env.push((
			"RIVET_CLUSTER_ID".into(),
			project_ctx.ns().cluster.id.to_string(),
		));

		if self.enable_tokio_console() {
			env.push(("TOKIO_CONSOLE_ENABLE".into(), "1".into()));
			env.push((
				"TOKIO_CONSOLE_BIND".into(),
				format!("0.0.0.0:{}", k8s::gen::TOKIO_CONSOLE_PORT),
			));
		}

		// Domains
		env.push(("RIVET_DOMAIN_MAIN".into(), project_ctx.domain_main()));
		env.push(("RIVET_DOMAIN_CDN".into(), project_ctx.domain_cdn()));
		env.push(("RIVET_DOMAIN_JOB".into(), project_ctx.domain_job()));
		env.push(("RIVET_ORIGIN_HUB".into(), project_ctx.origin_hub()));

		// Regions
		env.push(("RIVET_REGION".into(), region_id.clone()));
		env.push(("RIVET_PRIMARY_REGION".into(), project_ctx.primary_region()));

		// Networking
		if run_context == RunContext::Service {
			env.push(("HEALTH_PORT".into(), k8s::gen::HEALTH_PORT.to_string()));
			env.push(("METRICS_PORT".into(), k8s::gen::METRICS_PORT.to_string()));
			if self.config().kind.has_server() {
				env.push(("PORT".into(), k8s::gen::HTTP_SERVER_PORT.to_string()));
			}
		}

		// Configure TLS
		env.push(("IS_SECURE".to_owned(), "1".into()));

		// Add billing flag
		if project_ctx.config().billing_enabled {
			env.push(("IS_BILLING_ENABLED".to_owned(), "1".into()));
		}

		// Expose URLs to env
		//
		// We expose all services instead of just dependencies since we need to configure CORS
		// for some services which don't have an explicit dependency.
		env.extend(project_ctx.all_router_url_env().await);

		let config::ns::DnsProvider::Cloudflare { zones, .. } = &project_ctx.ns().dns.provider;
		env.push(("CLOUDFLARE_ZONE_ID_BASE".into(), zones.root.clone()));
		env.push(("CLOUDFLARE_ZONE_ID_GAME".into(), zones.game.clone()));
		env.push(("CLOUDFLARE_ZONE_ID_JOB".into(), zones.job.clone()));

		if let Some(hcaptcha) = &project_ctx.ns().captcha.hcaptcha {
			if self.depends_on_hcaptcha() {
				env.push((
					"HCAPTCHA_SITE_KEY_EASY".into(),
					hcaptcha.site_keys.easy.clone(),
				));
				env.push((
					"HCAPTCHA_SITE_KEY_MODERATE".into(),
					hcaptcha.site_keys.moderate.clone(),
				));
				env.push((
					"HCAPTCHA_SITE_KEY_DIFFICULT".into(),
					hcaptcha.site_keys.difficult.clone(),
				));
				env.push((
					"HCAPTCHA_SITE_KEY_ALWAYS_ON".into(),
					hcaptcha.site_keys.always_on.clone(),
				));
			}
		}

		if self.depends_on_nomad_api() {
			env.push((
				"NOMAD_URL".into(),
				format!(
					"http://{}",
					access_service(
						&project_ctx,
						&mut forward_configs,
						&run_context,
						"nomad-server",
						"nomad",
						4646,
					)
					.await?
				),
			));
		}

		if self.depends_on_clickhouse() {
			env.push((
				"CLICKHOUSE_URL".into(),
				access_service(
					&project_ctx,
					&mut forward_configs,
					&run_context,
					"clickhouse",
					"clickhouse",
					8123,
				)
				.await?,
			));
		}

		// TODO:
		// if self.depends_on_prometheus_api() {
		// 	// Add all prometheus regions to env
		// 	//
		// 	// We don't run Prometheus regionally at the moment, but we can add
		// 	// that later.
		// 	env.push((
		// 		format!("PROMETHEUS_URL"),
		// 		access_service(
		// 			&project_ctx,
		// 			&mut forward_configs,
		// 			&run_context,
		// 			"prometheus",
		// 			"prometheus",
		// 			9090,
		// 		)
		// 		.await?,
		// 	));
		// }

		// NATS
		env.push((
			"NATS_URL".into(),
			// TODO: Add back passing multiple NATS nodes for failover instead of using DNS resolution
			access_service(
				&project_ctx,
				&mut forward_configs,
				&run_context,
				"nats",
				"nats",
				4222,
			)
			.await?,
		));

		// Chirp config (used for both Chirp clients and Chirp workers)
		env.push(("CHIRP_SERVICE_NAME".into(), self.name()));
		env.push(("CHIRP_REGION".into(), region_id.clone()));

		// Chirp worker config
		if let (RunContext::Service, ServiceKind::Consumer { .. }) =
			(run_context, &self.config().kind)
		{
			env.push((
				"CHIRP_WORKER_INSTANCE".into(),
				format!("{}-$(KUBERNETES_POD_ID)", self.name()),
			));

			env.push(("CHIRP_WORKER_KIND".into(), "consumer".into()));
			env.push(("CHIRP_WORKER_CONSUMER_GROUP".into(), self.name()));
		}

		// Fly
		if let Some(fly) = &project_ctx.ns().fly {
			env.push(("FLY_ORGANIZATION_ID".into(), fly.organization_id.clone()));
			env.push(("FLY_REGION".into(), fly.region.clone()));
		}

		// CRDB
		let crdb_host = access_service(
			&project_ctx,
			&mut forward_configs,
			&run_context,
			"cockroachdb",
			"cockroachdb",
			26257,
		)
		.await?;
		for crdb_dep in self.crdb_dependencies(run_context).await {
			let username = "root"; // TODO:
			let sslmode = "disable"; // TODO:

			let uri = format!(
				"postgres://{username}@{crdb_host}/{db_name}?sslmode={sslmode}",
				db_name = crdb_dep.crdb_db_name(),
			);
			env.push((format!("CRDB_URL_{}", crdb_dep.name_screaming_snake()), uri));
		}

		// Expose all S3 endpoints to services that need them
		let s3_deps = if self.depends_on_s3() {
			project_ctx.all_services().await.to_vec()
		} else {
			self.s3_dependencies(run_context).await
		};
		for s3_dep in s3_deps {
			if !matches!(s3_dep.config().runtime, RuntimeKind::S3 { .. }) {
				continue;
			}

			// Add default provider
			let (default_provider, _) = project_ctx.default_s3_provider()?;
			env.push((
				"S3_DEFAULT_PROVIDER".to_string(),
				default_provider.as_str().to_uppercase(),
			));

			// Add all configured providers
			let providers = &project_ctx.ns().s3.providers;
			if providers.minio.is_some() {
				add_s3_env(&project_ctx, &mut env, &s3_dep, s3_util::Provider::Minio).await?;
			}
			if providers.backblaze.is_some() {
				add_s3_env(
					&project_ctx,
					&mut env,
					&s3_dep,
					s3_util::Provider::Backblaze,
				)
				.await?;
			}
			if providers.aws.is_some() {
				add_s3_env(&project_ctx, &mut env, &s3_dep, s3_util::Provider::Aws).await?;
			}
		}

		// S3 backfill
		if self.depends_on_s3_backfill() {
			if let Some(backfill) = &project_ctx.ns().s3.backfill {
				env.push((
					"S3_BACKFILL_PROVIDER".into(),
					heck::ShoutySnakeCase::to_shouty_snake_case(backfill.as_str()),
				));
			}
		}

		// Runtime-specific
		match &self.config().runtime {
			RuntimeKind::Rust { .. } => {
				env.push(("RUST_BACKTRACE".into(), "1".into()));
			}
			_ => {}
		}

		env.push((
			"RIVET_TELEMETRY_DISABLE".into(),
			if project_ctx.ns().rivet.telemetry.disable {
				"1"
			} else {
				"0"
			}
			.into(),
		));

		env.push((
			"RIVET_API_HUB_ORIGIN_REGEX".into(),
			project_ctx
				.ns()
				.rivet
				.api
				.hub_origin_regex
				.clone()
				.unwrap_or_else(|| {
					format!("^https://hub\\.{base}$", base = project_ctx.domain_main())
				}),
		));
		env.push((
			"RIVET_API_ERROR_VERBOSE".into(),
			if project_ctx.ns().rivet.api.error_verbose {
				"1"
			} else {
				"0"
			}
			.into(),
		));
		env.push((
			"RIVET_PROFANITY_FILTER_DISABLE".into(),
			if project_ctx.ns().rivet.profanity.filter_disable {
				"1"
			} else {
				"0"
			}
			.into(),
		));
		env.push((
			"RIVET_UPLOAD_NSFW_ERROR_VERBSOE".into(),
			if project_ctx.ns().rivet.upload.nsfw_error_verbose {
				"1"
			} else {
				"0"
			}
			.into(),
		));
		env.push((
			"RIVET_MM_LOBBY_DELIVERY_METHOD".into(),
			project_ctx
				.ns()
				.rivet
				.matchmaker
				.lobby_delivery_method
				.to_string(),
		));

		// Sort env by keys so it's always in the same order
		env.sort_by_cached_key(|x| x.0.clone());

		Ok((env, forward_configs))
	}

	pub async fn secret_env(
		&self,
		run_context: RunContext,
	) -> Result<(Vec<(String, String)>, Vec<utils::PortForwardConfig>)> {
		let project_ctx = self.project().await;

		let mut env = Vec::new();
		let mut forward_configs = Vec::new();

		// Write secrets
		for (secret_key, secret_config) in self.required_secrets(run_context).await? {
			let env_key = rivet_util::env::secret_env_var_key(&secret_key);
			if secret_config.optional {
				if let Some(value) = project_ctx.read_secret_opt(&secret_key).await? {
					env.push((env_key, value));
				}
			} else {
				env.push((env_key, project_ctx.read_secret(&secret_key).await?));
			}
		}

		// NATS
		env.push(("NATS_USERNAME".into(), "chirp".into()));
		env.push(("NATS_PASSWORD".into(), "password".into()));

		env.push((
			"RIVET_JWT_KEY_PUBLIC".into(),
			project_ctx
				.read_secret(&["jwt", "key", "public_pem"])
				.await?,
		));
		if self.depends_on_jwt_key_private() {
			env.push((
				"RIVET_JWT_KEY_PRIVATE".into(),
				project_ctx
					.read_secret(&["jwt", "key", "private_pem"])
					.await?,
			));
		}

		if run_context == RunContext::Service {
			if self.depends_on_sendgrid_key() {
				env.push((
					"SENDGRID_KEY".into(),
					project_ctx.read_secret(&["sendgrid", "key"]).await?,
				));
			}
		}

		if self.depends_on_cloudflare() {
			env.push((
				"CLOUDFLARE_AUTH_TOKEN".into(),
				project_ctx
					.read_secret(&["cloudflare", "terraform", "auth_token"])
					.await?,
			));
		}

		// CRDB
		let crdb_host = access_service(
			&project_ctx,
			&mut forward_configs,
			&run_context,
			"cockroachdb",
			"cockroachdb",
			26257,
		)
		.await?;
		for crdb_dep in self.crdb_dependencies(run_context).await {
			let username = "root"; // TODO:
			let sslmode = "disable"; // TODO:

			let uri = format!(
				"postgres://{username}@{crdb_host}/{db_name}?sslmode={sslmode}",
				db_name = crdb_dep.crdb_db_name(),
			);
			env.push((format!("CRDB_URL_{}", crdb_dep.name_screaming_snake()), uri));
		}

		// Redis
		for redis_dep in self.redis_dependencies(run_context).await {
			let name = redis_dep.name();
			let db_name = redis_dep.redis_db_name();

			// TODO: Use name and port to connect to different redis instances
			let host = access_service(
				&project_ctx,
				&mut forward_configs,
				&run_context,
				"redis-master",
				"redis",
				6379,
			)
			.await?;

			// Build URL with auth
			let username = project_ctx
				.read_secret(&["redis", &db_name, "username"])
				.await?;
			let password = project_ctx
				.read_secret_opt(&["redis", &db_name, "password"])
				.await?;
			let url = if let Some(password) = password {
				format!("redis://{}:{}@{host}", username, password)
			} else {
				format!("redis://{}@{host}", username)
			};

			env.push((
				format!("REDIS_URL_{}", db_name.to_uppercase().replace("-", "_")),
				url,
			));
		}

		// Expose S3 endpoints to services that need them
		let s3_deps = if self.depends_on_s3() {
			project_ctx.all_services().await.to_vec()
		} else {
			self.s3_dependencies(run_context).await
		};
		for s3_dep in s3_deps {
			if !matches!(s3_dep.config().runtime, RuntimeKind::S3 { .. }) {
				continue;
			}

			// Add default provider
			let default_provider_name = match project_ctx.default_s3_provider()? {
				(s3_util::Provider::Minio, _) => "MINIO",
				(s3_util::Provider::Backblaze, _) => "BACKBLAZE",
				(s3_util::Provider::Aws, _) => "AWS",
			};
			env.push((
				"S3_DEFAULT_PROVIDER".to_string(),
				default_provider_name.to_string(),
			));

			// Add all configured providers
			let providers = &project_ctx.ns().s3.providers;
			if providers.minio.is_some() {
				add_s3_env(&project_ctx, &mut env, &s3_dep, s3_util::Provider::Minio).await?;
			}
			if providers.backblaze.is_some() {
				add_s3_env(
					&project_ctx,
					&mut env,
					&s3_dep,
					s3_util::Provider::Backblaze,
				)
				.await?;
			}
			if providers.aws.is_some() {
				add_s3_env(&project_ctx, &mut env, &s3_dep, s3_util::Provider::Aws).await?;
			}
		}

		Ok((env, forward_configs))
	}
}

impl ServiceContextData {
	pub async fn docker_image_tag(&self) -> Result<String> {
		let project_ctx = self.project().await;

		let source_hash = project_ctx.source_hash();
		let repo = &project_ctx.ns().docker.repository;
		ensure!(repo.ends_with('/'), "docker repository must end with slash");

		Ok(format!(
			"{}{}:{}",
			repo,
			self.cargo_name().expect("no cargo name"),
			source_hash
		))
	}

	pub async fn upload_build(&self) -> Result<()> {
		let image_tag = self.docker_image_tag().await?;

		let mut cmd = Command::new("docker");
		cmd.arg("push");
		cmd.arg(image_tag);

		let status = cmd.status().await?;
		eprintln!();

		ensure!(status.success());

		Ok(())
	}
}

pub async fn access_service(
	_ctx: &ProjectContext,
	forward_configs: &mut Vec<utils::PortForwardConfig>,
	run_context: &RunContext,
	service_name: &'static str,
	namespace: &'static str,
	remote_port: u16,
) -> Result<String> {
	match run_context {
		RunContext::Test => {
			let local_port = utils::pick_port();
			forward_configs.push(utils::PortForwardConfig {
				service_name,
				namespace,
				local_port,
				remote_port,
			});

			Ok(format!("127.0.0.1:{local_port}"))
		}
		// Use the cluster address if running locally
		RunContext::Service => Ok(format!(
			"{service_name}.{namespace}.svc.cluster.local:{remote_port}"
		)),
	}
}

impl ServiceContextData {
	pub async fn ns_service_config(&self) -> config::ns::Service {
		let project_ctx = self.project().await;

		let is_singleton = matches!(
			&self.config().kind,
			ServiceKind::Headless {
				singleton: true,
				..
			} | ServiceKind::Api {
				singleton: true,
				..
			}
		);

		let mut service = project_ctx
			.ns()
			.services
			.get(&self.name())
			.cloned()
			.or_else(|| project_ctx.ns().services.get("default").cloned())
			.unwrap_or_else(|| match project_ctx.ns().cluster.kind {
				config::ns::ClusterKind::SingleNode { .. } => config::ns::Service {
					count: 1,
					resources: config::ns::ServiceResources {
						cpu: config::ns::CpuResources::Cpu(100),
						memory: 512,
						ephemeral_disk: 128,
					},
				},
				config::ns::ClusterKind::Distributed { .. } => config::ns::Service {
					count: 2,
					resources: config::ns::ServiceResources {
						cpu: config::ns::CpuResources::Cpu(250),
						memory: 256,
						ephemeral_disk: 128,
					},
				},
			});

		// Force single count if singleton
		if is_singleton {
			service.count = 1;
		}

		service
	}
}

async fn add_s3_env(
	project_ctx: &ProjectContext,
	env: &mut Vec<(String, String)>,
	s3_dep: &Arc<ServiceContextData>,
	provider: s3_util::Provider,
) -> Result<()> {
	let provider_upper = provider.as_str().to_uppercase();

	let s3_dep_name = s3_dep.name_screaming_snake();
	let s3_config = project_ctx.s3_config(provider).await?;
	let s3_creds = project_ctx.s3_credentials(provider).await?;

	env.push((
		format!("S3_{}_BUCKET_{}", provider_upper, s3_dep_name),
		s3_dep.s3_bucket_name().await,
	));
	env.push((
		format!("S3_{}_ENDPOINT_INTERNAL_{}", provider_upper, s3_dep_name),
		s3_config.endpoint_internal,
	));
	env.push((
		format!("S3_{}_ENDPOINT_EXTERNAL_{}", provider_upper, s3_dep_name),
		s3_config.endpoint_external,
	));
	env.push((
		format!("S3_{}_REGION_{}", provider_upper, s3_dep_name),
		s3_config.region,
	));
	env.push((
		format!("S3_{}_ACCESS_KEY_ID_{}", provider_upper, s3_dep_name),
		s3_creds.access_key_id,
	));
	env.push((
		format!("S3_{}_SECRET_ACCESS_KEY_{}", provider_upper, s3_dep_name),
		s3_creds.access_key_secret,
	));

	Ok(())
}
