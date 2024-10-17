use std::{
	hash::{Hash, Hasher},
	net::Ipv4Addr,
	path::PathBuf,
	sync::{Arc, Weak},
};

use anyhow::{ensure, Context, Result};
use async_recursion::async_recursion;
use indexmap::IndexMap;
use tokio::{fs, process::Command, sync::RwLock};

use super::BuildOptimization;
use crate::{
	config::{self, project::service::ServiceKind},
	context::{self, BuildContext, ProjectContext, RunContext},
	dep::{docker, k8s, terraform},
};

pub type ServiceContext = Arc<ServiceContextData>;

pub struct ServiceContextData {
	project: RwLock<Weak<context::project::ProjectContextData>>,
	config: config::project::service::ServiceConfig,
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
	}

	pub async fn project(&self) -> ProjectContext {
		self.project
			.read()
			.await
			.upgrade()
			.expect("missing project")
	}

	pub fn config(&self) -> &config::project::service::ServiceConfig {
		&self.config
	}
}

impl ServiceContextData {
	pub fn new(
		project: Weak<context::project::ProjectContextData>,
		config: config::project::service::ServiceConfig,
	) -> ServiceContextData {
		// Build context
		let ctx = ServiceContextData {
			project: RwLock::new(project),
			config,
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

	pub fn cargo_name(&self) -> &str {
		&self.config.build.package
	}
}

impl ServiceContextData {
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

	/// Path to the executable binary.
	pub async fn rust_bin_path(&self, optimization: &BuildOptimization) -> PathBuf {
		self.project()
			.await
			.cargo_target_dir()
			.join(match optimization {
				BuildOptimization::Release => "release",
				BuildOptimization::Debug => "debug",
			})
			.join(self.cargo_name())
	}
}

// Deploy meta
impl ServiceContextData {
	pub fn is_provision_service(&self) -> bool {
		self.config().service.name == "provision"
	}

	pub fn is_monolith_worker(&self) -> bool {
		self.config().service.name == "monolith-worker"
			|| self.config().service.name == "monolith-workflow-worker"
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

	pub fn depends_on_sendgrid_key(&self, project_ctx: &ProjectContext) -> bool {
		// self.name() == "email-send"
		project_ctx.ns().email.as_ref().map_or(false, |x| {
			matches!(x.provider, config::ns::EmailProvider::SendGrid { .. })
		})
	}

	pub fn depends_on_cloudflare(&self) -> bool {
		true
		// 	|| self.name() == "cf-custom-hostname-create"
		// 	|| self.name() == "cf-custom-hostname-delete"
		// 	|| self.name() == "api-cloud"
	}

	pub fn depends_on_captcha(&self) -> bool {
		// TODO:
		true
	}

	pub fn depends_on_infra(&self) -> bool {
		self.name() == "cluster-worker"
			|| self.name() == "monolith-worker"
			|| self.name() == "monolith-workflow-worker"
	}

	pub fn depends_on_cluster_config(&self) -> bool {
		self.name() == "cluster-default-update" || self.name() == "pegboard-dc-init"
	}

	pub fn depends_on_provision_margin(&self) -> bool {
		self.name() == "cluster-autoscale"
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
	BuildAndUpload {
		/// Push location (local repo)
		push_image_tag: String,
		/// Pull location (inside of k8s)
		pull_image_tag: String,
	},
}

impl ServiceContextData {
	/// Determines if this service needs to be recompiled.
	pub async fn build_plan(&self, build_context: &BuildContext) -> Result<ServiceBuildPlan> {
		let project_ctx = self.project().await;

		// Check if build exists on docker.io
		let pub_image_tag = self.docker_image_tag(&project_ctx, "docker.io/rivetgg/")?;
		if docker::cli::container_exists(&pub_image_tag).await {
			return Ok(ServiceBuildPlan::ExistingUploadedBuild {
				image_tag: pub_image_tag,
			});
		}

		// Check if build exists in config repo
		let image_tag = self.docker_image_tag(&project_ctx, &project_ctx.ns().docker.repository)?;
		if docker::cli::container_exists(&image_tag).await {
			return Ok(ServiceBuildPlan::ExistingUploadedBuild { image_tag });
		}

		if project_ctx.build_svcs_locally() {
			// Derive the build path
			let optimization = match &build_context {
				BuildContext::Bin { optimization } => optimization,
				BuildContext::Test { .. } => &BuildOptimization::Debug,
			};
			let output_path = self.rust_bin_path(optimization).await;

			// Rust libs always attempt to rebuild (handled by cargo)
			Ok(ServiceBuildPlan::BuildLocally {
				exec_path: output_path,
			})
		} else {
			let (push_repo, pull_repo) = project_ctx.docker_repos().await;

			Ok(ServiceBuildPlan::BuildAndUpload {
				push_image_tag: self.docker_image_tag(&project_ctx, &push_repo)?,
				pull_image_tag: self.docker_image_tag(&project_ctx, &pull_repo)?,
			})
		}
	}
}

impl ServiceContextData {
	pub async fn source_hash_dev(&self, build_optimization: &BuildOptimization) -> Result<String> {
		// Use binary modified timestamp for rust runtimes
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

	pub async fn env(&self, run_context: &RunContext) -> Result<IndexMap<String, String>> {
		let project_ctx = self.project().await;

		let mut env = IndexMap::new();

		// HACK: Link to dynamically linked libraries in /nix/store
		//
		// We build the binaries dynamically linked to dependencies from Nix, so
		// the output binaries are hardcoded to /nix/store.
		//
		// The `/nix/store` directory is mounted as a volume.
		if let config::ns::ClusterKind::SingleNode { .. } = project_ctx.ns().cluster.kind {
			env.insert(
				"LD_LIBRARY_PATH".into(),
				std::env::var("LD_LIBRARY_PATH").context("missing LD_LIBRARY_PATH")?,
			);
		}

		// TODO: This is re-running the hashing function for every service when we already did this in the planning step
		// Provide source hash to purge the cache when the service is updated
		let source_hash = self.source_hash_dev(&BuildOptimization::Debug).await?;
		env.insert("RIVET_SOURCE_HASH".into(), source_hash.clone());

		env.insert(
			"RIVET_PROJECT_SOURCE_HASH".into(),
			project_ctx.source_hash(),
		);

		let ns_service_config = self.ns_service_config().await;
		env.insert(
			"TOKIO_WORKER_THREADS".into(),
			if ns_service_config.resources.cpu >= 1000 {
				(ns_service_config.resources.cpu / 1000).max(1)
			} else {
				1
			}
			.to_string(),
		);

		// Generic context
		env.insert("RIVET_RUN_CONTEXT".into(), run_context.short().into());
		env.insert("RIVET_NAMESPACE".into(), project_ctx.ns_id().into());
		env.insert(
			"RIVET_CLUSTER_ID".into(),
			project_ctx.ns().cluster.id.to_string(),
		);

		if self.enable_tokio_console() {
			env.insert("TOKIO_CONSOLE_ENABLE".into(), "1".into());
			env.insert(
				"TOKIO_CONSOLE_BIND".into(),
				format!("0.0.0.0:{}", k8s::gen::TOKIO_CONSOLE_PORT),
			);
		}

		env.insert(
			"RIVET_ACCESS_KIND".into(),
			match project_ctx.ns().rivet.access {
				config::ns::RivetAccess::Private {} => "private".into(),
				config::ns::RivetAccess::Public {} => "public".into(),
			},
		);

		if project_ctx.ns().rivet.login.enable_admin {
			env.insert("RIVET_ACCESS_TOKEN_LOGIN".into(), "1".into());
		}

		// Domains
		if let Some(x) = project_ctx.domain_main() {
			env.insert("RIVET_DOMAIN_MAIN".into(), x);
		}
		if let Some(x) = project_ctx.domain_cdn() {
			env.insert("RIVET_DOMAIN_CDN".into(), x);
		}
		if let Some(x) = project_ctx.domain_job() {
			env.insert("RIVET_DOMAIN_JOB".into(), x);
		}
		if let Some(x) = project_ctx.domain_main_api() {
			env.insert("RIVET_DOMAIN_MAIN_API".into(), x);
		}
		if let Some(true) = project_ctx
			.ns()
			.dns
			.as_ref()
			.map(|x| x.deprecated_subdomains)
		{
			env.insert("RIVET_SUPPORT_DEPRECATED_SUBDOMAINS".into(), "1".into());
		}
		env.insert("RIVET_HOST_API".into(), project_ctx.host_api().await);
		env.insert("RIVET_HOST_TUNNEL".into(), project_ctx.host_tunnel().await);
		env.insert("RIVET_ORIGIN_API".into(), project_ctx.origin_api().await);
		env.insert("RIVET_ORIGIN_HUB".into(), project_ctx.origin_hub());

		// DNS
		if let Some(dns) = &project_ctx.ns().dns {
			if let Some(provider) = &dns.provider {
				env.insert(
					"RIVET_DNS_PROVIDER".into(),
					match provider {
						config::ns::DnsProvider::Cloudflare { .. } => "cloudflare".into(),
					},
				);
			}
		}

		// Networking
		if matches!(run_context, RunContext::Service { .. }) {
			env.insert("HEALTH_PORT".into(), k8s::gen::HEALTH_PORT.to_string());
			env.insert("METRICS_PORT".into(), k8s::gen::METRICS_PORT.to_string());
			if self.config().kind.has_server() {
				env.insert("PORT".into(), k8s::gen::HTTP_SERVER_PORT.to_string());
			}
		}

		// Add billing flag
		if let Some(billing) = &project_ctx.ns().rivet.billing {
			env.insert(
				"RIVET_BILLING".to_owned(),
				serde_json::to_string(&billing).unwrap(),
			);
		}

		// DNS
		if let Some(dns) = &project_ctx.ns().dns {
			match &dns.provider {
				Some(config::ns::DnsProvider::Cloudflare { account_id, .. }) => {
					env.insert("RIVET_DNS_PROVIDER".into(), "cloudflare".into());
					env.insert("CLOUDFLARE_ACCOUNT_ID".into(), account_id.clone());
				}
				None => {}
			}

			let dns_output = terraform::output::read_dns(&project_ctx).await;
			env.insert(
				"CLOUDFLARE_ZONE_ID_MAIN".into(),
				(*dns_output.cloudflare_zone_ids).main.clone(),
			);
			env.insert(
				"CLOUDFLARE_ZONE_ID_GAME".into(),
				(*dns_output.cloudflare_zone_ids).cdn.clone(),
			);
			env.insert(
				"CLOUDFLARE_ZONE_ID_JOB".into(),
				(*dns_output.cloudflare_zone_ids).job.clone(),
			);
		}

		// Infra Artifacts
		let infra_artifacts_output = terraform::output::read_infra_artifacts(&project_ctx).await;
		env.insert(
			"JOB_RUNNER_BINARY_KEY".into(),
			(*infra_artifacts_output.job_runner_binary_key).clone(),
		);
		env.insert(
			"CONTAINER_RUNNER_BINARY_KEY".into(),
			(*infra_artifacts_output.container_runner_binary_key).clone(),
		);
		env.insert(
			"PEGBOARD_MANAGER_BINARY_KEY".into(),
			(*infra_artifacts_output.pegboard_manager_binary_key).clone(),
		);

		// Backend
		if project_ctx.ns().rivet.backend.is_some() {
			let backend_output = terraform::output::read_backend(&project_ctx).await;

			env.insert(
				"CLOUDFLARE_BACKEND_DISPATCHER_NAMESPACE".into(),
				backend_output.dispatcher_namespace_name.to_string(),
			);
		}

		if self.depends_on_captcha() {
			if let Some(hcaptcha) = &project_ctx.ns().captcha.hcaptcha {
				env.insert(
					"HCAPTCHA_SITE_KEY_FALLBACK".into(),
					hcaptcha.site_key_fallback.clone(),
				);
			}

			if let Some(turnstile) = &project_ctx.ns().captcha.turnstile {
				env.insert(
					"TURNSTILE_SITE_KEY_MAIN".into(),
					turnstile.site_key_main.clone(),
				);
				env.insert(
					"TURNSTILE_SITE_KEY_CDN".into(),
					turnstile.site_key_cdn.clone(),
				);
			}
		}

		if self.depends_on_nomad_api() {
			// TODO: Read host url from terraform
			env.insert(
				"NOMAD_URL".into(),
				"http://nomad-server.nomad.svc.cluster.local:4646".into(),
			);
		}

		if let Some(x) = ns_service_config.crdb_min_connections {
			env.insert("CRDB_MIN_CONNECTIONS".into(), x.to_string());
		}
		if let Some(x) = ns_service_config.crdb_max_connections {
			env.insert("CRDB_MAX_CONNECTIONS".into(), x.to_string());
		}

		if project_ctx.ns().prometheus.is_some() && self.depends_on_prometheus_api() {
			env.insert(
				format!("PROMETHEUS_URL"),
				"http://prometheus-operated.prometheus.svc.cluster.local:9090".into(),
			);
		}

		// NATS
		env.insert(
			"NATS_URL".into(),
			// TODO: Add back passing multiple NATS nodes for failover instead of using DNS resolution
			"nats.nats.svc.cluster.local:4222".into(),
		);

		// Chirp config (used for both Chirp clients and Chirp workers)
		env.insert("CHIRP_SERVICE_NAME".into(), self.name());

		// Chirp worker config
		env.insert(
			"CHIRP_WORKER_INSTANCE".into(),
			format!("{}-$(KUBERNETES_POD_ID)", self.name()),
		);

		env.insert("CHIRP_WORKER_KIND".into(), "consumer".into());
		env.insert("CHIRP_WORKER_CONSUMER_GROUP".into(), self.name());

		// Fly
		if let Some(fly) = &project_ctx.ns().fly {
			env.insert("FLY_ORGANIZATION_ID".into(), fly.organization_id.clone());
			env.insert("FLY_REGION".into(), fly.region.clone());
		}

		add_s3_env(&project_ctx, &mut env).await?;

		// Runtime-specific
		env.insert("RUST_BACKTRACE".into(), "1".into());

		env.insert(
			"RIVET_API_HUB_ORIGIN_REGEX".into(),
			project_ctx.origin_hub_regex(),
		);
		if project_ctx.ns().rivet.api.error_verbose {
			env.insert("RIVET_API_ERROR_VERBOSE".into(), "1".into());
		}
		if project_ctx.ns().rivet.profanity.filter_disable {
			env.insert("RIVET_PROFANITY_FILTER_DISABLE".into(), "1".into());
		}

		// Nomad
		env.insert(
			"NOMAD_SERVER_COUNT".into(),
			project_ctx.nomad_server_count().to_string(),
		);

		if let Some(provisioning) = &project_ctx.ns().rivet.provisioning {
			if self.depends_on_cluster_config() || matches!(run_context, RunContext::Test { .. }) {
				env.insert(
					"RIVET_DEFAULT_CLUSTER_CONFIG".into(),
					serde_json::to_string(&provisioning.cluster)?,
				);
			}

			if self.depends_on_provision_margin() {
				env.insert(
					"RIVET_JOB_SERVER_PROVISION_MARGIN".to_string(),
					provisioning.job_server_provision_margin.to_string(),
				);
				env.insert(
					"RIVET_PB_SERVER_PROVISION_MARGIN".to_string(),
					provisioning.pb_server_provision_margin.to_string(),
				);
			}

			env.insert(
				"TLS_ACME_DIRECTORY".to_string(),
				serde_json::to_value(&provisioning.acme_directory)?
					.as_str()
					.unwrap()
					.to_string(),
			);
		}

		if let Some(nsfw_check) = &project_ctx.ns().rivet.upload.nsfw_check {
			env.insert("RIVET_UPLOAD_NSFW_CHECK_ENABLED".into(), "1".into());

			if nsfw_check.error_verbose {
				env.insert("RIVET_UPLOAD_NSFW_ERROR_VERBOSE".into(), "1".into());
			}
		}

		Ok(env)
	}

	pub async fn secret_env(&self, _run_context: &RunContext) -> Result<IndexMap<String, String>> {
		let project_ctx = self.project().await;

		let mut env = IndexMap::new();

		// Write secrets
		for (secret_key_str, secret_config) in &self.config().secrets {
			let secret_key = secret_key_str.split("/").collect::<Vec<_>>();
			let env_key = secret_env_var_key(&secret_key);
			if secret_config.optional {
				if let Some(value) = project_ctx.read_secret_opt(&secret_key).await? {
					env.insert(env_key, value);
				}
			} else {
				env.insert(env_key, project_ctx.read_secret(&secret_key).await?);
			}
		}

		// NATS
		env.insert("NATS_USERNAME".into(), "chirp".into());
		env.insert("NATS_PASSWORD".into(), "password".into());

		env.insert(
			"RIVET_JWT_KEY_PUBLIC".into(),
			project_ctx
				.read_secret(&["jwt", "key", "public_pem"])
				.await?,
		);
		if self.depends_on_jwt_key_private() {
			env.insert(
				"RIVET_JWT_KEY_PRIVATE".into(),
				project_ctx
					.read_secret(&["jwt", "key", "private_pem"])
					.await?,
			);
		}

		if self.depends_on_sendgrid_key(&project_ctx) {
			env.insert(
				"SENDGRID_KEY".into(),
				project_ctx.read_secret(&["sendgrid", "key"]).await?,
			);
		}

		// CRDB
		// TODO: Function is expensive
		{
			let crdb_data = terraform::output::read_crdb(&project_ctx).await;
			let crdb_host = format!("{}:{}", *crdb_data.host, *crdb_data.port);
			let username = project_ctx.read_secret(&["crdb", "username"]).await?;
			let password = project_ctx.read_secret(&["crdb", "password"]).await?;
			let sslmode = "verify-ca";

			let url = format!(
				"postgres://{}:{}@{crdb_host}/postgres?sslmode={sslmode}",
				username, password,
			);
			env.insert("CRDB_URL".into(), url);

			// TODO:
			let workflow_url = format!(
				"postgres://{}:{}@{crdb_host}/db_workflow?sslmode={sslmode}",
				username, password,
			);
			env.insert("CRDB_WORKFLOW_URL".into(), workflow_url);
		}

		// Redis
		let redis_data = terraform::output::read_redis(&project_ctx).await;
		for db_name in ["persistent", "ephemeral"] {
			// Read host and port from terraform
			let hostname = redis_data
				.host
				.get(db_name)
				.expect("terraform output for redis db not found");
			let port = redis_data
				.port
				.get(db_name)
				.expect("terraform output for redis db not found");
			let host = format!("{}:{}", *hostname, *port);

			// Read auth secrets
			let (username, password) = match project_ctx.ns().redis.provider {
				config::ns::RedisProvider::Kubernetes {}
				| config::ns::RedisProvider::Aiven { .. } => (
					project_ctx
						.read_secret(&["redis", &db_name, "username"])
						.await?,
					project_ctx
						.read_secret_opt(&["redis", &db_name, "password"])
						.await?,
				),
			};

			// Build URL with auth
			let url = if let Some(password) = password {
				format!("rediss://{}:{}@{host}", username, password)
			} else {
				format!("rediss://{}@{host}", username)
			};

			env.insert(
				format!("REDIS_URL_{}", db_name.to_uppercase().replace('-', "_")),
				url,
			);
		}

		// ClickHouse
		if self.depends_on_clickhouse() {
			if project_ctx.ns().clickhouse.is_some() {
				let clickhouse_data = terraform::output::read_clickhouse(&project_ctx).await;
				let username = if self.is_provision_service() {
					// Needs root user in order to be able to create other users
					"default"
				} else {
					// Use regular user
					"chirp"
				};
				let password = project_ctx
					.read_secret(&["clickhouse", "users", username, "password"])
					.await?;
				let uri = format!(
					"https://{}:{}@{}:{}",
					username, password, *clickhouse_data.host, *clickhouse_data.port_https
				);

				env.insert("CLICKHOUSE_URL".into(), uri);
			} else {
				env.insert("CLICKHOUSE_DISABLED".into(), "1".into());
			}
		}

		add_s3_secret_env(&project_ctx, &mut env).await?;

		if let Some(dns) = &project_ctx.ns().dns {
			match &dns.provider {
				Some(config::ns::DnsProvider::Cloudflare { .. }) => {
					if self.depends_on_cloudflare() {
						env.insert(
							"CLOUDFLARE_AUTH_TOKEN".into(),
							project_ctx
								.read_secret(&["cloudflare", "terraform", "auth_token"])
								.await?,
						);
					}
				}
				None => {}
			}
		}

		// if self.depends_on_infra() && project_ctx.ns().rivet.provisioning.is_some() {
		let tls = terraform::output::read_tls(&project_ctx).await;

		env.insert(
			"TLS_CERT_LOCALLY_SIGNED_JOB_CERT_PEM".into(),
			tls.tls_cert_locally_signed_job.cert_pem.clone(),
		);
		env.insert(
			"TLS_CERT_LOCALLY_SIGNED_JOB_KEY_PEM".into(),
			tls.tls_cert_locally_signed_job.key_pem.clone(),
		);
		env.insert(
			"TLS_ACME_ACCOUNT_PRIVATE_KEY_PEM".into(),
			(*tls.acme_account_private_key_pem).clone(),
		);
		env.insert(
			"TLS_ROOT_CA_CERT_PEM".into(),
			(*tls.root_ca_cert_pem).clone(),
		);
		// }

		Ok(env)
	}
}

impl ServiceContextData {
	pub fn docker_image_tag(&self, project_ctx: &ProjectContext, repo: &str) -> Result<String> {
		ensure!(repo.ends_with('/'), "docker repository must end with slash");

		let source_hash = project_ctx.source_hash();

		Ok(format!("{}{}:{}", repo, self.cargo_name(), source_hash))
	}

	pub async fn upload_build(&self) -> Result<()> {
		let project_ctx = self.project().await;
		let (push_repo, _) = project_ctx.docker_repos().await;
		let image_tag = self.docker_image_tag(&project_ctx, &push_repo)?;

		let mut cmd = Command::new("docker");
		cmd.arg("push");
		cmd.arg(image_tag);

		let status = cmd.status().await?;
		eprintln!();

		ensure!(status.success());

		Ok(())
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
					resources: self.config().resources.single_node.clone(),
					crdb_min_connections: None,
					crdb_max_connections: None,
				},
				config::ns::ClusterKind::Distributed { .. } => config::ns::Service {
					count: 2,
					resources: self.config().resources.distributed.clone(),
					crdb_min_connections: None,
					crdb_max_connections: None,
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
	env: &mut IndexMap<String, String>,
) -> Result<()> {
	let s3_config = project_ctx.s3_config().await?;

	env.insert("S3_ENDPOINT_INTERNAL".into(), s3_config.endpoint_internal);
	// External endpoint
	{
		let mut external_endpoint = s3_config.endpoint_external;

		// Switch to internal k8s url if public ip is loopback
		if let config::ns::ClusterKind::SingleNode {
			public_ip: Some(public_ip),
			minio_port,
			..
		} = &project_ctx.ns().cluster.kind
		{
			let is_loopback = public_ip
				.parse::<Ipv4Addr>()
				.ok()
				.map(|ip| ip.is_loopback())
				.unwrap_or_default();

			if is_loopback {
				external_endpoint = format!("http://minio.minio.svc.cluster.local:{minio_port}");
			}
		}

		env.insert("S3_ENDPOINT_EXTERNAL".into(), external_endpoint);
	}
	env.insert("S3_REGION".into(), s3_config.region);

	Ok(())
}

async fn add_s3_secret_env(
	project_ctx: &ProjectContext,
	env: &mut IndexMap<String, String>,
) -> Result<()> {
	let s3_creds = project_ctx.s3_credentials().await?;

	env.insert("S3_ACCESS_KEY_ID".into(), s3_creds.access_key_id);
	env.insert("S3_SECRET_ACCESS_KEY".into(), s3_creds.access_key_secret);

	Ok(())
}

/// TODO: Reuse code with lib/util/env/src/lib.r
pub fn secret_env_var_key(key: &[impl AsRef<str>]) -> String {
	key.iter()
		.map(|x| x.as_ref().to_uppercase())
		.collect::<Vec<_>>()
		.join("_")
}
