use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};

use anyhow::{bail, ensure, Context, Result};
use s3_util::aws_sdk_s3;
use tempfile::NamedTempFile;
use tokio::{fs, process::Command, sync::RwLock};

use crate::{
	config::{
		self,
		service::{RuntimeKind, ServiceKind},
	},
	context::{self, BuildContext, ProjectContext, RunContext},
	dep::{self, cloudflare, s3},
	utils,
};

use super::BuildOptimization;

pub type ServiceContext = Arc<ServiceContextData>;

pub struct ServiceContextData {
	pub(in crate::context) project: RwLock<Weak<context::project::ProjectContextData>>,
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

		Some(Arc::new(ServiceContextData::new(
			project,
			config,
			workspace_path,
			path,
			cargo,
		)))
	}

	fn new(
		project: Weak<context::project::ProjectContextData>,
		config: config::service::ServiceConfig,
		workspace_path: &Path,
		path: &Path,
		cargo: Option<config::service::CargoConfig>,
	) -> ServiceContextData {
		// Build context
		let ctx = ServiceContextData {
			project: RwLock::new(project),
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
	pub async fn rust_bin_path(
		&self,
		optimization: &BuildOptimization,
		target: super::RustBuildTarget,
	) -> PathBuf {
		match target {
			super::RustBuildTarget::Native => self
				.project()
				.await
				.path()
				.join("target")
				.join(match optimization {
					BuildOptimization::Release => "release",
					BuildOptimization::Debug => "debug",
				})
				.join(self.cargo_name().expect("no cargo name")),
			super::RustBuildTarget::Musl => self
				.project()
				.await
				.path()
				.join("target")
				.join("x86_64-unknown-linux-musl")
				.join(match optimization {
					BuildOptimization::Release => "release",
					BuildOptimization::Debug => "debug",
				})
				.join(self.cargo_name().expect("no cargo name")),
		}
	}
}

// Deploy meta
impl ServiceContextData {
	pub fn has_migrations(&self) -> bool {
		matches!(
			self.config().runtime,
			RuntimeKind::CRDB { .. } | RuntimeKind::ClickHouse { .. }
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

	pub fn depends_on_region_config(&self) -> bool {
		true
		// self.name().starts_with("region-")
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
	/// Build exists locally.
	ExistingLocalBuild { output_path: PathBuf },

	/// Build exists on S3 server.
	ExistingUploadedBuild { build_key: String },

	/// Build the service locally.
	BuildLocally { output_path: PathBuf },

	/// Build the service and upload to S3.
	BuildAndUpload { build_key: String },

	/// Run a Docker container.
	Docker { image_tag: String },
}

impl ServiceContextData {
	/// Determines if this service needs to be recompiled.
	pub async fn build_plan(
		&self,
		build_context: &BuildContext,
		force_build: bool,
	) -> Result<ServiceBuildPlan> {
		let project_ctx = self.project().await;

		match &project_ctx.ns().deploy.kind {
			// Build locally
			config::ns::DeployKind::Local { .. } => {
				// Derive the build path
				let optimization = match &build_context {
					BuildContext::Bin { optimization } => optimization,
					BuildContext::Test => &BuildOptimization::Debug,
				};
				let output_path = self
					.rust_bin_path(optimization, project_ctx.rust_build_target())
					.await;

				if force_build {
					return Ok(ServiceBuildPlan::BuildLocally { output_path });
				}

				// Check if there's an existing build we can use
				let build_exists = tokio::fs::metadata(&output_path).await.is_ok();
				if build_exists {
					return Ok(ServiceBuildPlan::ExistingLocalBuild { output_path });
				}

				// Default to building
				Ok(ServiceBuildPlan::BuildLocally { output_path })
			}
			// Build and upload to S3
			config::ns::DeployKind::Cluster { .. } => {
				// Derive the build key
				let key = self.s3_build_key(build_context).await?;

				if force_build {
					return Ok(ServiceBuildPlan::BuildAndUpload { build_key: key });
				}

				// Check if there's an existing build we can use
				let s3_client = project_ctx.s3_client_service_builds().await?;
				let build_exists = s3::check_exists_cached(&project_ctx, &s3_client, &key).await?;
				if build_exists {
					return Ok(ServiceBuildPlan::ExistingUploadedBuild { build_key: key });
				}

				// Default to building
				Ok(ServiceBuildPlan::BuildAndUpload { build_key: key })
			}
		}
	}
}

// Dependencies
impl ServiceContextData {
	pub async fn dependencies(&self) -> Vec<ServiceContext> {
		let project = self.project().await;

		let all_svcs = project.all_services().await;

		let mut dep_ctxs = Vec::<ServiceContext>::new();

		// TODO: Find a cleaner way of specifying database dependencies
		// HACK: Mark all database & S3 dependencies as dependencies for all services in order to
		// expose the env
		for svc in all_svcs {
			if matches!(
				svc.config().kind,
				ServiceKind::Database { .. } | ServiceKind::Cache { .. }
			) {
				dep_ctxs.push(svc.clone());
			}
		}

		// TODO: Add dev dependencies if building for tests
		// Add operation dependencies from Cargo.toml
		if let Some(cargo) = &self.cargo {
			let svc_path = self.path();
			let svcs = cargo
				.dependencies
				.iter()
				.filter_map(|(_, x)| {
					if let config::service::CargoDependency::Path { path } = x {
						Some(path)
					} else {
						None
					}
				})
				.filter_map(|path| {
					let absolute_path = svc_path.join(path);
					all_svcs
						.iter()
						.filter(|x| x.path() == absolute_path)
						.next()
						.cloned()
				});

			dep_ctxs.extend(svcs);
		}

		// Check that these are services you can explicitly depend on
		for dep in &dep_ctxs {
			if !self.config().service.test_only && dep.config().service.test_only {
				panic!(
					"{} -> {}: cannot depend on a `service.test-only` service outside of `test-dependencies`",
					self.name(),
					dep.name()
				);
			}

			if !matches!(
				dep.config().kind,
				ServiceKind::Database { .. } | ServiceKind::Cache { .. }
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

	pub async fn crdb_dependencies(&self) -> Vec<ServiceContext> {
		self.dependencies()
			.await
			.iter()
			.filter(|svc| {
				if let RuntimeKind::CRDB { .. } = svc.config().runtime {
					true
				} else {
					false
				}
			})
			.cloned()
			.collect()
	}

	pub async fn redis_dependencies(&self) -> Vec<ServiceContext> {
		self.dependencies()
			.await
			.iter()
			.filter(|svc| {
				if let RuntimeKind::Redis { .. } = svc.config().runtime {
					true
				} else {
					false
				}
			})
			.cloned()
			.collect()
	}

	pub async fn s3_dependencies(&self) -> Vec<ServiceContext> {
		self.dependencies()
			.await
			.iter()
			.filter(|svc| {
				if let RuntimeKind::S3 { .. } = svc.config().runtime {
					true
				} else {
					false
				}
			})
			.cloned()
			.collect()
	}

	pub async fn nats_dependencies(&self) -> Vec<ServiceContext> {
		self.dependencies()
			.await
			.iter()
			.filter(|svc| {
				if let RuntimeKind::Nats { .. } = svc.config().runtime {
					true
				} else {
					false
				}
			})
			.cloned()
			.collect()
	}
}

impl ServiceContextData {
	pub async fn source_hash_dev(&self, build_optimization: &BuildOptimization) -> Result<String> {
		match self.config().runtime {
			// Use binary modified timestamp for rust runtimes
			RuntimeKind::Rust { .. } => {
				let bin_ts = if let Ok(metadata) = fs::metadata(
					self.rust_bin_path(
						&build_optimization,
						self.project().await.rust_build_target(),
					)
					.await,
				)
				.await
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

	async fn required_secrets(&self) -> Result<Vec<(Vec<String>, config::service::Secret)>> {
		let mut secrets = self
			.project()
			.await
			.recursive_dependencies(&[self.name()])
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
	) -> Result<(Vec<(String, String)>, Vec<cloudflare::TunnelConfig>)> {
		let project_ctx = self.project().await;

		let region_id = project_ctx.primary_region_or_local();
		let mut env = Vec::new();
		let mut tunnel_configs = Vec::<cloudflare::TunnelConfig>::new();

		// HACK: Link to dynamically linked libraries in /nix/store
		//
		// We build the binaries dynamically linked to dependencies from Nix, so
		// the output binaries are hardcoded to /nix/store.
		//
		// The `/nix/store` directory is mounted as a volume.
		if let config::ns::DeployKind::Local { .. } = project_ctx.ns().deploy.kind {
			env.push((
				"LD_LIBRARY_PATH".into(),
				std::env::var("LD_LIBRARY_PATH").context("missing LD_LIBRARY_PATH")?,
			));
		}

		// Write secrets
		for (secret_key, secret_config) in self.required_secrets().await? {
			let env_key = rivet_util::env::secret_env_var_key(&secret_key);
			if secret_config.optional {
				if let Some(value) = project_ctx.read_secret_opt(&secret_key).await? {
					env.push((env_key, value));
				}
			} else {
				env.push((env_key, project_ctx.read_secret(&secret_key).await?));
			}
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
				config::ns::CpuResources::CpuCores(cores) => cores.max(2),
				config::ns::CpuResources::Cpu(_) => 2,
			}
			.to_string(),
		));

		// Provide default Nomad variables if in test
		if matches!(run_context, RunContext::Test) {
			env.push(("NOMAD_REGION".into(), "global".into()));
			env.push(("NOMAD_DC".into(), region_id.clone()));
			env.push((
				"NOMAD_TASK_DIR".into(),
				project_ctx.gen_path().display().to_string(),
			));
		}

		// Generic context
		env.push(("RIVET_RUN_CONTEXT".into(), run_context.short().into()));
		env.push(("RIVET_NAMESPACE".into(), project_ctx.ns_id().into()));

		if self.enable_tokio_console() {
			env.push(("TOKIO_CONSOLE_ENABLE".into(), "1".into()));
			env.push((
				"TOKIO_CONSOLE_BIND".into(),
				r#"0.0.0.0:{{env "NOMAD_PORT_tokio_console"}}"#.into(),
			));
		}

		// Domains
		env.push(("RIVET_DOMAIN_MAIN".into(), project_ctx.domain_main()));
		env.push(("RIVET_DOMAIN_CDN".into(), project_ctx.domain_cdn()));
		env.push(("RIVET_DOMAIN_JOB".into(), project_ctx.domain_job()));
		env.push(("RIVET_ORIGIN_HUB".into(), project_ctx.origin_hub()));

		// Regions
		match run_context {
			RunContext::Service => {
				env.push(("RIVET_REGION".into(), "${NOMAD_DC}".into()));
			}
			RunContext::Test => {
				env.push(("RIVET_REGION".into(), region_id.clone()));
			}
		}
		env.push(("RIVET_PRIMARY_REGION".into(), project_ctx.primary_region()));

		// Networking
		if run_context == RunContext::Service {
			env.push(("HEALTH_PORT".into(), "${NOMAD_PORT_health}".into()));
			env.push(("METRICS_PORT".into(), "${NOMAD_PORT_metrics}".into()));
			if self.config().kind.has_server() {
				env.push(("PORT".into(), "${NOMAD_PORT_http}".into()));
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

		let config::ns::DnsProvider::Cloudflare { zones, .. } = &project_ctx.ns().dns.provider;
		if self.depends_on_cloudflare() {
			env.push((
				"CLOUDFLARE_AUTH_TOKEN".into(),
				project_ctx
					.read_secret(&["cloudflare", "terraform", "auth_token"])
					.await?,
			));
		}
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
				"NOMAD_ADDRESS".into(),
				format!(
					"http://{}",
					access_service(
						&project_ctx,
						&mut tunnel_configs,
						&run_context,
						"nomad.service.consul:4646",
						(cloudflare::TunnelProtocol::Http, "nomad"),
					)
					.await?,
				),
			));
			env.push((
				"CONSUL_ADDRESS".into(),
				format!(
					"http://{}",
					access_service(
						&project_ctx,
						&mut tunnel_configs,
						&run_context,
						"consul.service.consul:8500",
						(cloudflare::TunnelProtocol::Http, "consul"),
					)
					.await?,
				),
			));
		}

		if self.depends_on_prometheus_api() {
			// Add all prometheus regions to env
			//
			// We don't run Prometheus regionally at the moment, but we can add
			// that later.
			env.push((
				format!("PROMETHEUS_URL"),
				access_service(
					&project_ctx,
					&mut tunnel_configs,
					&run_context,
					"http.prometheus-job.service.consul:9090",
					(cloudflare::TunnelProtocol::Http, "prometheus"),
				)
				.await?,
			));
		}

		// NATS config
		env.push((
			"NATS_URL".into(),
			// TODO: Add back passsing multiple NATS nodes for failover instead of using DNS resolution
			access_service(
				&project_ctx,
				&mut tunnel_configs,
				&run_context,
				"client.nats-server.service.consul:4222",
				(cloudflare::TunnelProtocol::Tcp, "nats-client"),
			)
			.await?,
		));

		env.push(("NATS_USERNAME".into(), "chirp".into()));
		env.push(("NATS_PASSWORD".into(), "password".into()));

		// Chirp config (used for both Chirp clients and Chirp workers)
		env.push(("CHIRP_SERVICE_NAME".into(), self.name()));
		match run_context {
			RunContext::Service => {
				env.push(("CHIRP_REGION".into(), "${NOMAD_DC}".into()));
			}
			RunContext::Test => {
				env.push(("CHIRP_REGION".into(), region_id.clone()));
			}
		}

		// Chirp worker config
		if let (RunContext::Service, ServiceKind::Consumer { .. }) =
			(run_context, &self.config().kind)
		{
			env.push((
				"CHIRP_WORKER_INSTANCE".into(),
				format!("{}-${{NOMAD_DC}}-${{NOMAD_ALLOC_INDEX}}", self.name()),
			));

			env.push(("CHIRP_WORKER_KIND".into(), "consumer".into()));
			env.push(("CHIRP_WORKER_CONSUMER_GROUP".into(), self.name()));
		}

		// Redis
		for redis_dep in self.redis_dependencies().await {
			let name = redis_dep.name();
			let db_name = redis_dep.redis_db_name();
			let port = dep::redis::server_port(&redis_dep);

			let host = access_service(
				&project_ctx,
				&mut tunnel_configs,
				&run_context,
				&format!("listen.{name}.service.consul:{port}"),
				(cloudflare::TunnelProtocol::Tcp, &name),
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

		// CRDB
		let mut crdb_host = None;
		for crdb_dep in self.crdb_dependencies().await {
			let username = "root"; // TODO:
			let sslmode = "disable"; // TODO:

			// Resolve CRDB host
			if crdb_host.is_none() {
				crdb_host = Some(
					access_service(
						&project_ctx,
						&mut tunnel_configs,
						&run_context,
						"sql.cockroach.service.consul:26257",
						(cloudflare::TunnelProtocol::Tcp, "cockroach-sql"),
					)
					.await?,
				);
			}
			let host = crdb_host.as_ref().unwrap();

			let uri = format!(
				"postgres://{username}@{host}/{db_name}?sslmode={sslmode}",
				db_name = crdb_dep.crdb_db_name(),
			);
			env.push((format!("CRDB_URL_{}", crdb_dep.name_screaming_snake()), uri));
		}

		// Expose all S3 endpoints to services that need it
		let s3_deps = if self.depends_on_s3() {
			project_ctx.all_services().await.to_vec()
		} else {
			self.s3_dependencies().await
		};
		for s3_dep in s3_deps {
			if !matches!(s3_dep.config().runtime, RuntimeKind::S3 { .. }) {
				continue;
			}

			let s3_config = project_ctx
				.clone()
				.s3_config(project_ctx.clone().s3_credentials().await?)
				.await?;

			env.push((
				format!("S3_BUCKET_{}", s3_dep.name_screaming_snake()),
				s3_dep.s3_bucket_name().await,
			));
			env.push((
				format!("S3_ENDPOINT_INTERNAL_{}", s3_dep.name_screaming_snake()),
				s3_config.endpoint_internal,
			));
			env.push((
				format!("S3_ENDPOINT_EXTERNAL_{}", s3_dep.name_screaming_snake()),
				s3_config.endpoint_external,
			));
			env.push((
				format!("S3_REGION_{}", s3_dep.name_screaming_snake()),
				s3_config.region,
			));
			env.push((
				format!("S3_ACCESS_KEY_ID_{}", s3_dep.name_screaming_snake()),
				s3_config.credentials.access_key_id,
			));
			env.push((
				format!("S3_SECRET_ACCESS_KEY_{}", s3_dep.name_screaming_snake()),
				s3_config.credentials.access_key_secret,
			));
		}

		// Runtime-specific
		match &self.config().runtime {
			RuntimeKind::Rust { .. } => {
				env.push(("RUST_BACKTRACE".into(), "1".into()));
			}
			_ => {}
		}

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

		// Sort env by keys so it's always in the same order
		env.sort_by_cached_key(|x| x.0.clone());

		Ok((env, tunnel_configs))
	}
}

impl ServiceContextData {
	pub async fn s3_build_key(&self, build_context: &BuildContext) -> Result<String> {
		let source_hash = self
			.source_hash_dev(match &build_context {
				BuildContext::Bin { optimization } => optimization,
				BuildContext::Test => &BuildOptimization::Debug,
			})
			.await?;
		Ok(format!(
			"{}/{}/{source_hash}.tar.bz2",
			self.name(),
			build_context.path()
		))
	}

	pub async fn package_build(&self, build_context: &BuildContext) -> Result<NamedTempFile> {
		let tar_bz2 = NamedTempFile::new()?;

		// Compress with bzip2. It has similar performance as gzip. xz is way
		// too slow.
		let mut cmd = Command::new("tar");
		cmd.arg("cfj").arg(tar_bz2.path());

		match build_context {
			BuildContext::Bin { optimization } => match &self.config.runtime {
				RuntimeKind::Rust {} => {
					// Write binary
					let bin_path = self
						.rust_bin_path(optimization, self.project().await.rust_build_target())
						.await;
					cmd.arg("-C")
						.arg(bin_path.parent().context("bin_path.parent()")?)
						.arg(bin_path.file_name().context("bin_path.file_name()")?);
				}
				_ => bail!("can't get build binary for this runtime type"),
			},
			BuildContext::Test => {
				bail!("can't get build path for test")
			}
		}

		let output = cmd.output().await?;
		ensure!(
			output.status.success(),
			"tar failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);

		Ok(tar_bz2)
	}

	pub async fn upload_build(&self, build_context: &BuildContext) -> Result<()> {
		let package_file = self.package_build(build_context).await?;

		let body = aws_sdk_s3::types::ByteStream::from_path(package_file.path()).await?;

		let s3_client = self.project().await.s3_client_service_builds().await?;
		let key = self.s3_build_key(build_context).await?;
		s3_client
			.put_object()
			.bucket(s3_client.bucket())
			.key(key)
			.body(body)
			.send()
			.await?;

		Ok(())
	}
}

pub async fn access_service(
	ctx: &ProjectContext,
	tunnel_configs: &mut Vec<cloudflare::TunnelConfig>,
	run_context: &RunContext,
	service_hostname: &str,
	(tunnel_protocol, tunnel_name): (cloudflare::TunnelProtocol, &str),
) -> Result<String> {
	match (run_context, &ctx.ns().deploy.kind) {
		// Create tunnels to remote services
		(RunContext::Test, config::ns::DeployKind::Cluster { .. }) => {
			// Save the tunnel config
			let local_port = utils::pick_port();
			tunnel_configs.push(cloudflare::TunnelConfig::new_with_port(
				tunnel_protocol,
				tunnel_name,
				local_port,
			));

			// Hardcode to the forwarded port
			Ok(format!("127.0.0.1:{local_port}"))
		}
		// Use the Consul address if either in production or running locally
		(RunContext::Service, _) | (RunContext::Test, config::ns::DeployKind::Local { .. }) => {
			Ok(service_hostname.into())
		}
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

		let service = project_ctx
			.ns()
			.services
			.get(&self.name())
			.cloned()
			.or_else(|| project_ctx.ns().services.get("default").cloned())
			.unwrap_or_else(|| match project_ctx.ns().deploy.kind {
				config::ns::DeployKind::Local { .. } => config::ns::Service {
					count: 1,
					resources: config::ns::ServiceResources {
						cpu: config::ns::CpuResources::Cpu(50),
						memory: 64,
						ephemeral_disk: 128,
					},
				},
				config::ns::DeployKind::Cluster { .. } => config::ns::Service {
					count: if is_singleton { 1 } else { 2 },
					resources: config::ns::ServiceResources {
						cpu: config::ns::CpuResources::Cpu(250),
						memory: 256,
						ephemeral_disk: 128,
					},
				},
			});

		if is_singleton {
			assert_eq!(service.count, 1)
		}

		service
	}
}
