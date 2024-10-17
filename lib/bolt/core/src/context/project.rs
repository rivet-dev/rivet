use anyhow::*;
use bolt_config::ns::S3Provider;
use sha2::{Digest, Sha256};
use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
	process::Command,
	str::FromStr,
	sync::{Arc, Weak},
};
use tokio::{fs, sync::Mutex};

use super::{ServiceContext, ServiceContextData};
use crate::{config, context, dep::terraform, utils::command_helper::CommandHelper};

pub type ProjectContext = Arc<ProjectContextData>;

pub struct ProjectContextData {
	path: PathBuf,
	ns_id: String,
	config: config::project::Project,
	config_local: config::local::Local,
	ns_config: config::ns::Namespace,
	cache: Mutex<config::cache::Cache>,
	secrets: serde_json::Value,
	svc_ctxs_sorted: Vec<context::service::ServiceContext>,
	svc_ctxs_map: HashMap<String, context::service::ServiceContext>,

	source_hash: String,
}

impl ProjectContextData {
	#[allow(unused)]
	pub fn config(&self) -> &config::project::Project {
		&self.config
	}

	pub fn config_local(&self) -> &config::local::Local {
		&self.config_local
	}

	pub async fn openapi_config_cloud(
		self: &Arc<Self>,
	) -> Result<rivet_api::apis::configuration::Configuration> {
		let api_admin_token = self.read_secret(&["rivet", "api_admin", "token"]).await?;

		Ok(rivet_api::apis::configuration::Configuration {
			base_path: self.origin_api().await,
			bearer_access_token: Some(api_admin_token),
			..Default::default()
		})
	}

	pub fn ns(&self) -> &config::ns::Namespace {
		&self.ns_config
	}

	pub async fn cache<T: Sized>(
		&self,
		reader: impl FnOnce(&config::cache::Cache) -> T + Sized,
	) -> T {
		let cache = self.cache.lock().await;
		reader(&cache)
	}

	pub async fn cache_mut<T: Sized>(
		&self,
		reader: impl FnOnce(&mut config::cache::Cache) -> T + Sized,
	) -> T {
		let res = {
			let mut cache = self.cache.lock().await;
			reader(&mut cache)
		};

		self.write_cache().await;

		res
	}

	pub fn path(&self) -> &Path {
		self.path.as_path()
	}

	pub fn cargo_target_dir(&self) -> PathBuf {
		std::env::var("CARGO_TARGET_DIR").map_or_else(
			|_| self.path().join("target"),
			|x| PathBuf::from_str(&x).unwrap(),
		)
	}
}

impl ProjectContextData {
	pub async fn new(ns_id: Option<String>) -> ProjectContext {
		// Read the config
		let project_root = ProjectContextData::seek_project_root().await;
		let config = ProjectContextData::read_config(project_root.as_path()).await;
		let config_local = ProjectContextData::read_config_local(project_root.as_path()).await;
		let cache = ProjectContextData::read_cache(project_root.as_path()).await;

		let ns_id = ns_id
			.or_else(|| config_local.namespace.clone())
			.expect("no namespace specified, either use the `BOLT_NAMESPACE` environment variable or specify in `Bolt.local.toml`");
		let ns_config = ProjectContextData::read_ns(project_root.as_path(), &ns_id).await;

		// Load secrets
		let secrets = ProjectContextData::read_secrets(
			Some(&ns_config.secrets),
			project_root.as_path(),
			&ns_id,
		)
		.await;

		// If there is no diff, use the git commit hash
		let source_diff = get_source_diff(&project_root).await.unwrap();
		let source_hash = if source_diff.is_empty() {
			let mut cmd = Command::new("git");
			cmd.current_dir(&project_root).arg("rev-parse").arg("HEAD");

			cmd.exec_string_with_err("Command failed", true)
				.await
				.unwrap()
				// No idea why a new line appears at the end of this in CI
				.trim()
				.to_string()
		} else {
			// Get hash of diff
			hex::encode(Sha256::digest(source_diff))
		};

		// Build services
		let mut svc_ctxs_map = HashMap::new();
		for (name, service) in &config.services {
			let data = ServiceContextData::new(Weak::new(), service.clone());
			svc_ctxs_map.insert(name.clone(), Arc::new(data));
		}

		// Build sorted list of services for consistent execution order
		let mut svc_ctxs_sorted = svc_ctxs_map.values().cloned().collect::<Vec<_>>();
		svc_ctxs_sorted.sort_by_key(|v| v.name());

		// Create project
		let ctx = Arc::new(ProjectContextData {
			path: project_root.clone(),
			ns_id,
			config,
			config_local,
			ns_config,
			secrets,
			cache: Mutex::new(cache),
			svc_ctxs_sorted,
			svc_ctxs_map,

			source_hash,
		});

		ctx.validate_ns();

		// Assign references to all services after we're done
		for (_, svc) in &ctx.svc_ctxs_map {
			svc.set_project(Arc::downgrade(&ctx)).await;
		}

		ctx
	}

	/// Validates the namespace config.
	fn validate_ns(&self) {
		match &self.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode {
				api_http_port,
				api_https_port,
				minio_port,
				..
			} => {
				if self.ns().dns.is_some() {
					assert_eq!(
						80, *api_http_port,
						"api_http_port must be 80 if DNS is configured"
					);
					assert_eq!(
						Some(443),
						*api_https_port,
						"api_https_port must be 443 if DNS is configured"
					);
					assert_eq!(
						9000, *minio_port,
						"minio_port must not be changed if DNS is configured"
					);
				}
			}
			config::ns::ClusterKind::Distributed { .. } => {
				assert!(
					std::env::consts::OS == "linux"
						&& (std::env::consts::ARCH == "x86_64"
							|| std::env::consts::ARCH == "x86"
							|| std::env::consts::ARCH == "i686"),
					"distributed clusters on platforms other than linux-x86 are not supported"
				);

				assert!(
					self.dns_enabled(),
					"must have DNS configured with a distributed cluster"
				);
			}
		}

		// MARK: Api
		if let Some(hub_origin_regex) = &self.ns().rivet.api.hub_origin_regex {
			if let Err(err) = regex::Regex::new(&hub_origin_regex) {
				panic!("invalid hub origin regex: {err}");
			}
		}

		// MARK: Backend
		if self.ns().rivet.backend.is_some() {
			assert!(
				self.ns().dns.is_some(),
				"must have DNS configured with for backend"
			);
		}

		// MARK: Cluster Provisioning
		if let Some(cluster) = self
			.ns()
			.rivet
			.provisioning
			.as_ref()
			.and_then(|p| p.cluster.as_ref())
		{
			assert!(
				self.dns_enabled(),
				"must have dns configured to provision servers"
			);
			let mut unique_datacenter_ids = HashSet::new();

			for (name_id, datacenter) in &cluster.datacenters {
				assert!(
					!unique_datacenter_ids.contains(&datacenter.datacenter_id),
					"invalid datacenter ({}): datacenter_id not unique",
					name_id,
				);
				unique_datacenter_ids.insert(datacenter.datacenter_id);

				let Some(ats_pool) = datacenter
					.pools
					.get(&config::ns::ProvisioningDatacenterPoolType::Ats)
				else {
					panic!("invalid datacenter ({}): Missing ATS pool", name_id);
				};

				assert!(
					ats_pool.desired_count <= ats_pool.max_count,
					"invalid datacenter ({}): ATS desired > max",
					name_id,
				);
				assert!(
					ats_pool.min_count <= ats_pool.desired_count,
					"invalid datacenter ({}): ATS min > desired",
					name_id,
				);

				// Validate the build delivery method
				match datacenter.build_delivery_method {
					config::ns::ProvisioningBuildDeliveryMethod::TrafficServer => {
						assert_ne!(
							0, ats_pool.desired_count,
							"invalid datacenter ({}): TrafficServer delivery method will not work without ats servers. Either set datacenter.build_delivery_method = \"s3_direct\" to download builds directly from S3 or increase the ATS pool count.",
							name_id,
						);
					}
					config::ns::ProvisioningBuildDeliveryMethod::S3Direct => {
						assert_eq!(
							0, ats_pool.desired_count,
							"invalid datacenter ({}): S3Direct delivery method should not be used if ats servers are available",
							name_id,
						);
					}
				}

				// Validate all required pools exist
				let gg_pool = datacenter
					.pools
					.get(&config::ns::ProvisioningDatacenterPoolType::Gg);
				let gg_count = gg_pool.map(|pool| pool.desired_count).unwrap_or_default();
				assert_ne!(
					gg_count, 0,
					"invalid datacenter ({}): Missing GG servers",
					name_id,
				);
				assert!(
					gg_count <= gg_pool.unwrap().max_count,
					"invalid datacenter ({}): GG desired > max",
					name_id,
				);
				assert!(
					gg_pool.unwrap().min_count <= gg_pool.unwrap().desired_count,
					"invalid datacenter ({}): GG min > desired",
					name_id,
				);

				let job_pool = datacenter
					.pools
					.get(&config::ns::ProvisioningDatacenterPoolType::Job);
				let job_count = job_pool.map(|pool| pool.desired_count).unwrap_or_default();

				let pb_pool = datacenter
					.pools
					.get(&config::ns::ProvisioningDatacenterPoolType::Pegboard);
				let pb_count = pb_pool.map(|pool| pool.desired_count).unwrap_or_default();

				assert_ne!(
					job_count + pb_count,
					0,
					"invalid datacenter ({}): Must have at least 1 job or pegboard server",
					name_id,
				);

				if let Some(job_pool) = &job_pool {
					assert!(
						job_count <= job_pool.max_count,
						"invalid datacenter ({}): Job desired > max",
						name_id,
					);
					assert!(
						job_pool.min_count <= job_pool.desired_count,
						"invalid datacenter ({}): Job min > desired",
						name_id,
					);
				}
				if let Some(pb_pool) = &pb_pool {
					assert!(
						pb_count <= pb_pool.max_count,
						"invalid datacenter ({}): Pegboard desired > max",
						name_id,
					);
					assert!(
						pb_pool.min_count <= pb_pool.desired_count,
						"invalid datacenter ({}): Pegboard min > desired",
						name_id,
					);
				}

				// Validate Linode
				#[allow(irrefutable_let_patterns)]
				if let config::ns::ProvisioningProvider::Linode = datacenter.provider {
					assert!(
						ats_pool.drain_timeout >= 55 * 60 * 1000,
						"invalid datacenter ({}): ATS drain timeout < 55 min (Linode bills hourly, drain timeout should be close to hour intervals)",
						name_id,
					);

					if let Some(gg_pool) = &gg_pool {
						assert!(
							gg_pool.drain_timeout >= 55 * 60 * 1000,
							"invalid datacenter ({}): GG drain timeout < 55 min (Linode bills hourly, drain timeout should be close to hour intervals)",
							name_id,
						);
					}

					if let Some(job_pool) = &job_pool {
						assert!(
							job_pool.drain_timeout >= 55 * 60 * 1000,
							"invalid datacenter ({}): Job drain timeout < 55 min (Linode bills hourly, drain timeout should be close to hour intervals)",
							name_id,
						);
					}

					if let Some(pb_pool) = &pb_pool {
						assert!(
							pb_pool.drain_timeout >= 55 * 60 * 1000,
							"invalid datacenter ({}): Pegboard drain timeout < 55 min (Linode bills hourly, drain timeout should be close to hour intervals)",
							name_id,
						);
					}
				}
			}
		}

		// MARK: Billing emails
		if self.ns().rivet.billing.is_some() {
			assert!(
				self.ns().email.is_some(),
				"cannot enable billing without emailing"
			);
		}

		assert!(
			self.ns().docker.repository.ends_with('/'),
			"docker repository must end with slash"
		);
	}

	// Traverses from FS root to CWD, returns first directory with Bolt.toml
	pub async fn seek_project_root() -> PathBuf {
		let path = std::env::current_dir().unwrap();
		for i in 1..=path.iter().count() {
			let partial_path = path.iter().take(i).collect::<PathBuf>();

			if partial_path.join("Bolt.toml").is_file() {
				return partial_path;
			}
		}

		eprintln!("Could not find project root.");
		std::process::exit(1);
	}

	pub fn get_secrets_path(
		ns_secrets: Option<&config::ns::Secrets>,
		project_path: &Path,
		ns_id: &str,
	) -> PathBuf {
		if let Some(path) = ns_secrets.and_then(|s| s.path.as_ref()) {
			path.clone()
		} else {
			project_path.join("secrets").join(format!("{}.toml", ns_id))
		}
	}
}

impl ProjectContextData {
	async fn read_config(project_path: &Path) -> config::project::Project {
		let config_path = project_path.join("Bolt.toml");
		let config_str = fs::read_to_string(config_path).await.unwrap();

		match toml::from_str::<config::project::Project>(&config_str) {
			Result::Ok(x) => x,
			Result::Err(err) => {
				if let Some(span) = err.span().filter(|span| span.start != span.end) {
					panic!(
						"failed to parse project config ({:?}): {}\n\n{}\n",
						&span,
						err.message(),
						&config_str[span.clone()]
					);
				} else {
					panic!("failed to parse project config: {}", err.message());
				}
			}
		}
	}

	pub async fn read_config_local(project_path: &Path) -> config::local::Local {
		let config_path = project_path.join("Bolt.local.toml");
		match fs::read_to_string(config_path).await {
			Result::Ok(config) => toml::from_str::<config::local::Local>(&config).unwrap(),
			Result::Err(_) => Default::default(),
		}
	}

	async fn read_ns(project_path: &Path, ns_id: &str) -> config::ns::Namespace {
		let path = project_path
			.join("namespaces")
			.join(format!("{ns_id}.toml"));
		let config_str = fs::read_to_string(&path)
			.await
			.unwrap_or_else(|_| panic!("failed to read namespace config: {}", path.display()));
		let config = match toml::from_str::<config::ns::Namespace>(&config_str) {
			Result::Ok(x) => x,
			Result::Err(err) => {
				if let Some(span) = err.span().filter(|span| span.start != span.end) {
					panic!(
						"failed to parse namespace config ({:?}): {}\n\n{}\n",
						&span,
						err.message(),
						&config_str[span.clone()],
					);
				} else {
					panic!("failed to parse namespace config: {}", err.message());
				}
			}
		};

		config
	}

	pub async fn read_partial_ns(project_path: &Path, ns_id: &str) -> config::ns::PartialNamespace {
		let path = project_path
			.join("namespaces")
			.join(format!("{ns_id}.toml"));
		let config_str = fs::read_to_string(&path)
			.await
			.unwrap_or_else(|_| panic!("failed to read namespace config: {}", path.display()));

		match toml::from_str::<config::ns::PartialNamespace>(&config_str) {
			Result::Ok(x) => x,
			Result::Err(err) => {
				if let Some(span) = err.span().filter(|span| span.start != span.end) {
					panic!(
						"failed to partially parse namespace config ({:?}): {}\n\n{}\n",
						&span,
						err.message(),
						&config_str[span.clone()],
					);
				} else {
					panic!(
						"failed to partially parse namespace config: {}",
						err.message()
					);
				}
			}
		}
	}

	pub async fn read_secrets(
		ns_secrets: Option<&config::ns::Secrets>,
		project_path: &Path,
		ns_id: &str,
	) -> serde_json::Value {
		let secrets_path = ProjectContextData::get_secrets_path(ns_secrets, project_path, ns_id);
		// Read the config
		let config_str = fs::read_to_string(&secrets_path)
			.await
			.context(format!(
				"failed to read secrets file: {}",
				secrets_path.display()
			))
			.unwrap();

		toml::from_str::<serde_json::Value>(&config_str)
			.context("failed to read secrets")
			.unwrap()
	}

	async fn read_cache(project_path: &Path) -> config::cache::Cache {
		let config_path = project_path.join(".bolt-cache.json");
		match fs::read(config_path).await {
			Result::Ok(config_buffer) => {
				serde_json::from_slice::<config::cache::Cache>(config_buffer.as_slice()).unwrap()
			}
			Result::Err(_) => Default::default(),
		}
	}

	pub async fn write_cache(&self) {
		let cache = self.cache.lock().await;
		let data = serde_json::to_vec(&*cache).unwrap();
		fs::write(self.path().join(".bolt-cache.json"), data)
			.await
			.unwrap();
	}
}

impl ProjectContextData {
	pub fn ns_id(&self) -> &str {
		&self.ns_id
	}

	pub async fn all_services(&self) -> &[ServiceContext] {
		self.svc_ctxs_sorted.as_slice()
	}

	pub async fn service_with_name(
		self: &Arc<Self>,
		name: &str,
	) -> context::service::ServiceContext {
		if let Some(ctx) = self.svc_ctxs_map.get(name) {
			ctx.clone()
		} else {
			panic!("Could not find service with name {}", name);
		}
	}

	pub async fn service_with_name_opt(
		self: &Arc<Self>,
		name: &str,
	) -> Option<context::service::ServiceContext> {
		self.svc_ctxs_map.get(name).cloned()
	}

	pub async fn services_with_pattern<T: AsRef<str>>(
		self: &Arc<Self>,
		pattern: T,
	) -> Vec<context::service::ServiceContext> {
		self.all_services()
			.await
			.iter()
			.cloned()
			.filter(|svc| svc.name_matches_pattern(pattern.as_ref()))
			.collect::<Vec<_>>()
	}

	pub async fn services_with_names<T: AsRef<str>>(
		self: &Arc<Self>,
		names: &[T],
	) -> Vec<context::service::ServiceContext> {
		let mut svc_ctxs = Vec::new();
		for name in names {
			svc_ctxs.push(self.service_with_name(name.as_ref()).await);
		}
		svc_ctxs
	}

	pub async fn services_with_patterns<T: AsRef<str>>(
		self: &Arc<Self>,
		patterns: &[T],
	) -> Vec<context::service::ServiceContext> {
		let mut svc_ctxs = Vec::new();
		for pattern in patterns {
			for svc_ctx in self.services_with_pattern(pattern).await {
				if !svc_ctxs.contains(&svc_ctx) {
					svc_ctxs.push(svc_ctx);
				}
			}
		}
		svc_ctxs
	}
}

impl ProjectContextData {
	pub fn ns_path(&self) -> PathBuf {
		self.path.join("namespaces")
	}

	pub fn secrets_path(&self) -> PathBuf {
		ProjectContextData::get_secrets_path(Some(&self.ns().secrets), self.path(), self.ns_id())
	}

	pub fn gen_path(&self) -> PathBuf {
		self.path.join("gen")
	}

	pub fn volumes_path(&self) -> PathBuf {
		self.path.join("volumes")
	}

	pub fn tf_path(&self) -> PathBuf {
		self.path.join("infra").join("tf")
	}
}

impl ProjectContextData {
	pub fn k8s_cluster_name(&self) -> String {
		format!("rivet-{}", self.ns_id())
	}

	pub fn gen_kubeconfig_path(&self) -> PathBuf {
		self.gen_path()
			.join("k8s")
			.join("kubeconfig")
			.join(format!("{}.yml", self.ns_id()))
	}

	/// If the Kubernetes pods have resource limits imposed.
	pub fn limit_resources(&self) -> bool {
		match self.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode {
				limit_resources, ..
			} => limit_resources,
			config::ns::ClusterKind::Distributed { .. } => true,
		}
	}
}

impl ProjectContextData {
	pub fn domain_main(&self) -> Option<String> {
		self.ns().dns.as_ref().map(|x| x.domain.main.clone())
	}

	pub fn domain_cdn(&self) -> Option<String> {
		self.ns().dns.as_ref().map(|x| x.domain.cdn.clone())
	}

	pub fn domain_job(&self) -> Option<String> {
		self.ns().dns.as_ref().map(|x| x.domain.job.clone())
	}

	/// Domain to host the API endpoint on. This is the default domain for all endpoints without a
	/// specific subdomain.
	pub fn domain_main_api(&self) -> Option<String> {
		self.domain_main().map(|x| format!("api.{x}"))
	}

	pub fn has_dev_tunnel(&self) -> bool {
		matches!(
			self.ns().cluster.kind,
			config::ns::ClusterKind::SingleNode {
				dev_tunnel: Some(_),
				..
			}
		)
	}

	pub async fn get_dev_public_ip(self: &Arc<Self>) -> String {
		match &self.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode {
				public_ip: Some(_),
				dev_tunnel: Some(_),
				..
			} => {
				panic!("cannot have both public_ip and dev_tunnel")
			}
			config::ns::ClusterKind::SingleNode {
				public_ip: Some(public_ip),
				..
			} => public_ip.clone(),
			config::ns::ClusterKind::SingleNode {
				dev_tunnel: Some(_),
				..
			} => {
				if terraform::cli::has_applied(self, "dev_tunnel").await {
					let dev_tunnel = terraform::output::read_dev_tunnel(self).await;
					(*dev_tunnel.tunnel_public_ip).clone()
				} else {
					// HACK: If the dev tunnel has not yet been applied, we don't know the host
					// yet.
					"UNREACHABLE".into()
				}
			}
			config::ns::ClusterKind::SingleNode { .. } => {
				panic!("public ip not configured")
			}
			config::ns::ClusterKind::Distributed { .. } => {
				panic!("does not have dev public ip")
			}
		}
	}

	/// Host used for building links to the API endpoint.
	pub async fn host_api(self: &Arc<Self>) -> String {
		if let Some(domain_main_api) = self.domain_main_api() {
			domain_main_api
		} else if let config::ns::ClusterKind::SingleNode { api_http_port, .. } =
			&self.ns().cluster.kind
		{
			format!("{}:{api_http_port}", self.get_dev_public_ip().await)
		} else {
			unreachable!()
		}
	}

	pub async fn host_tunnel(self: &Arc<Self>) -> String {
		let k8s_infra = terraform::output::read_k8s_infra(self).await;

		let tunnel_port = if let config::ns::ClusterKind::SingleNode { tunnel_port, .. } =
			&self.ns().cluster.kind
		{
			*tunnel_port
		} else {
			5000
		};

		format!("{}:{tunnel_port}", *k8s_infra.traefik_tunnel_external_ip)
	}

	/// Origin used for building links to the API endpoint.
	pub async fn origin_api(self: &Arc<Self>) -> String {
		if let Some(domain_main_api) = self.domain_main_api() {
			format!("https://{domain_main_api}")
		} else if let config::ns::ClusterKind::SingleNode { api_http_port, .. } =
			&self.ns().cluster.kind
		{
			format!("http://{}:{api_http_port}", self.get_dev_public_ip().await)
		} else {
			unreachable!()
		}
	}

	/// Origin used to access Minio. Only applicable if Minio provider is specified.
	pub async fn origin_minio(self: &Arc<Self>) -> String {
		if let Some(domain_main) = self.domain_main() {
			format!("https://minio.{domain_main}")
		} else if let config::ns::ClusterKind::SingleNode { minio_port, .. } =
			&self.ns().cluster.kind
		{
			format!("http://{}:{minio_port}", self.get_dev_public_ip().await)
		} else {
			unreachable!()
		}
	}

	/// Origin used for building links to the Hub.
	pub fn origin_hub(&self) -> String {
		self.ns().rivet.api.hub_origin.clone().unwrap_or_else(|| {
			if let Some(domain_main) = self.domain_main() {
				format!("https://hub.{domain_main}")
			} else {
				// We assume the dev is hosting the hub locally
				"http://127.0.0.1:5080".into()
			}
		})
	}

	pub fn origin_hub_regex(&self) -> String {
		self.ns()
			.rivet
			.api
			.hub_origin_regex
			.clone()
			.unwrap_or_else(|| {
				// Create regex pattern from the default hub origin
				format!("^{}$", self.origin_hub().replace('.', "\\."))
			})
	}

	pub fn tls_enabled(&self) -> bool {
		self.ns().dns.is_some()
	}

	pub fn dns_enabled(&self) -> bool {
		self.ns()
			.dns
			.as_ref()
			.and_then(|dns| dns.provider.as_ref())
			.is_some()
	}

	pub fn nomad_server_count(&self) -> usize {
		// !!! DO NOT CHANGE !!!
		//
		// This value must be 1, 3, 5, or 7. More = better redundancy, but does not make things faster.
		//
		// See https://developer.hashicorp.com/nomad/tutorials/enterprise/production-reference-architecture-vm-with-consul
		match self.ns().cluster.kind {
			config::ns::ClusterKind::Distributed { .. } => 3,
			config::ns::ClusterKind::SingleNode { .. } => 1,
		}
	}
}

pub struct S3Credentials {
	pub access_key_id: String,
	pub access_key_secret: String,
}

pub struct S3Config {
	pub endpoint_internal: String,
	pub endpoint_external: String,
	pub region: String,
}

impl ProjectContextData {
	/// Returns the appropriate S3 connection configuration for the provided S3 provider.
	pub async fn s3_credentials(self: &Arc<Self>) -> Result<S3Credentials> {
		Ok(S3Credentials {
			access_key_id: self.read_secret(&["s3", "terraform", "key_id"]).await?,
			access_key_secret: self.read_secret(&["s3", "terraform", "key"]).await?,
		})
	}

	/// Returns the appropriate S3 connection configuration for the provided S3 provider.
	pub async fn s3_config(self: &Arc<Self>) -> Result<S3Config> {
		match &self.ns().s3.provider {
			S3Provider::Minio => {
				Ok(S3Config {
					endpoint_internal: "http://minio.minio.svc.cluster.local:9000".into(),
					// Use localhost if DNS is not configured
					endpoint_external: self.origin_minio().await,
					// Minio defaults to us-east-1 region
					// https://github.com/minio/minio/blob/0ec722bc5430ad768a263b8464675da67330ad7c/cmd/server-main.go#L739
					region: "us-east-1".into(),
				})
			}
			S3Provider::Aws => {
				let endpoint = "https://s3.us-east-1.amazonaws.com".to_string();
				Ok(S3Config {
					endpoint_internal: endpoint.clone(),
					endpoint_external: endpoint,
					region: "us-east-1".into(),
				})
			}
		}
	}

	pub fn s3_cors_allowed_origins(&self) -> Vec<String> {
		self.ns()
			.s3
			.cors
			.allowed_origins
			.clone()
			.unwrap_or_else(|| {
				let mut origins = Vec::new();
				origins.push(self.origin_hub());
				origins
			})
	}

	pub fn imagor_cors_allowed_origins(&self) -> Vec<String> {
		// Mirror S3 CORS for now
		self.s3_cors_allowed_origins()
	}
}

impl ProjectContextData {
	pub fn gen_tf_env_path(&self) -> PathBuf {
		self.gen_path()
			.join("tf")
			.join("env")
			.join(format!("{}.tfvars.json", self.ns_id()))
	}
}

impl ProjectContextData {
	pub fn build_optimization(&self) -> context::BuildOptimization {
		match self.ns().rust.build_opt {
			config::ns::RustBuildOpt::Debug => context::BuildOptimization::Debug,
			config::ns::RustBuildOpt::Release => context::BuildOptimization::Release,
		}
	}
}

impl ProjectContextData {
	/// Gets the correct repo to push svc builds to/pull from.
	pub async fn docker_repos(self: &Arc<Self>) -> (String, String) {
		match self.ns().kubernetes.provider {
			config::ns::KubernetesProvider::K3d { use_local_repo } if use_local_repo => {
				let output = terraform::output::read_k8s_cluster_k3d(self).await;
				let local_repo = format!("localhost:{}/", *output.repo_port);
				let internal_repo = format!("{}:{}/", *output.repo_host, *output.repo_port);

				(local_repo, internal_repo)
			}
			_ => (
				self.ns().docker.repository.clone(),
				self.ns().docker.repository.clone(),
			),
		}
	}

	/// Whether or not to build svc images locally vs inside of docker.
	pub fn build_svcs_locally(&self) -> bool {
		match self.ns().kubernetes.provider {
			config::ns::KubernetesProvider::K3d { use_local_repo } if use_local_repo => false,
			_ => matches!(
				&self.ns().cluster.kind,
				config::ns::ClusterKind::SingleNode { .. }
			),
		}
	}
}

impl ProjectContextData {
	pub fn leader_count(&self) -> usize {
		match &self.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => 1,
			config::ns::ClusterKind::Distributed { .. } => 3,
		}
	}
}

impl ProjectContextData {
	pub fn source_hash(&self) -> String {
		self.source_hash.clone()
	}
}

impl ProjectContextData {
	/// Reads a secret from the configured secret datasource.
	pub async fn read_secret(&self, key_path: &[impl AsRef<str>]) -> Result<String> {
		self.read_secret_opt(key_path).await?.with_context(|| {
			let path_joined = key_path
				.iter()
				.map(|x| x.as_ref())
				.collect::<Vec<_>>()
				.join("/");
			format!(
				"secret '{path_joined}' does not exist in '{}'",
				self.secrets_path().display(),
			)
		})
	}

	/// Reads a secret from the configured data source, returning None if not available.
	pub async fn read_secret_opt(&self, key_path: &[impl AsRef<str>]) -> Result<Option<String>> {
		ProjectContextData::read_secret_inner(&self.secrets, key_path).await
	}

	async fn read_secret_inner(
		secrets: &serde_json::Value,
		key_path: &[impl AsRef<str>],
	) -> Result<Option<String>> {
		// Extract the value
		let mut current_value = secrets;
		for component in key_path {
			let component: &str = component.as_ref();

			if let Some(x) = current_value.get(component) {
				current_value = x;
			} else {
				return Ok(None);
			}
		}

		// Serialize to string
		let value_str = match current_value {
			serde_json::Value::Null => None,
			serde_json::Value::Bool(x) => Some(x.to_string()),
			serde_json::Value::Number(x) => Some(x.to_string()),
			serde_json::Value::String(x) => Some(x.clone()),
			_ => bail!("cannot convert to string: {current_value}"),
		};

		Ok(value_str)
	}
}

async fn get_source_diff(path: &Path) -> Result<Vec<u8>> {
	// Get diff for this repo
	let mut result = get_diff_for_path(path).await?;

	// Also get diff for all submodules
	let submodules = get_submodules(path).await?;
	for submodule in submodules {
		result.extend(get_diff_for_path(&submodule).await?);
	}

	Ok(result)
}

async fn get_diff_for_path(path: &Path) -> Result<Vec<u8>> {
	use tokio::io::AsyncReadExt;
	use tokio::process::Command;

	// Get git diff
	let diff_output = Command::new("git")
		.current_dir(path)
		.args(&["--no-pager", "diff", "HEAD", "--minimal"])
		.output()
		.await?;
	let mut result = diff_output.stdout;

	// Add diff for untracked files
	let ls_files_output = Command::new("git")
		.current_dir(path)
		.args(&[
			"--no-pager",
			"ls-files",
			"-z",
			"--others",
			"--exclude-standard",
		])
		.output()
		.await?;
	for file in String::from_utf8(ls_files_output.stdout)?.split('\0') {
		if !file.is_empty() {
			let mut file_content = Vec::new();
			tokio::fs::File::open(path.join(file))
				.await?
				.read_to_end(&mut file_content)
				.await?;
			result.extend(file_content);
		}
	}

	Ok(result)
}

async fn get_submodules(path: &Path) -> Result<Vec<PathBuf>> {
	let mut submodule_cmd = Command::new("git");
	submodule_cmd
		.current_dir(path)
		.args(&["submodule", "status", "--recursive"]);

	let submodule_output = submodule_cmd
		.exec_string_with_err("Command failed", true)
		.await?
		.trim()
		.to_string();

	let paths = submodule_output
		.lines()
		.filter_map(|line| {
			line.split_whitespace()
				.nth(1)
				.map(|submodule_path| path.join(submodule_path))
		})
		.collect();

	Ok(paths)
}
