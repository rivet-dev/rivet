use anyhow::*;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tokio::{fs, sync::Mutex};

use crate::{
	config::{self, service::ServiceDomain},
	context,
	dep::{self, s3, terraform},
};

use super::ServiceContext;

pub type ProjectContext = Arc<ProjectContextData>;

pub struct ProjectContextData {
	ns_id: String,
	config: config::project::Project,
	config_local: config::local::Local,
	ns_config: config::ns::Namespace,
	cache: Mutex<config::cache::Cache>,
	path: PathBuf,
	svc_ctxs: Vec<context::service::ServiceContext>,
	svc_ctxs_map: HashMap<String, context::service::ServiceContext>,
}

impl ProjectContextData {
	#[allow(unused)]
	pub fn config(&self) -> &config::project::Project {
		&self.config
	}

	pub fn config_local(&self) -> &config::local::Local {
		&self.config_local
	}

	pub fn ns(&self) -> &config::ns::Namespace {
		&self.ns_config
	}

	pub async fn cache<T: Sized>(
		&self,
		reader: impl FnOnce(&config::cache::Cache) -> T + Sized,
	) -> T {
		let cache = self.cache.lock().await;
		reader(&*cache)
	}

	pub async fn cache_mut<T: Sized>(
		&self,
		reader: impl FnOnce(&mut config::cache::Cache) -> T + Sized,
	) -> T {
		let res = {
			let mut cache = self.cache.lock().await;
			reader(&mut *cache)
		};

		self.write_cache().await;

		res
	}

	pub fn path(&self) -> &Path {
		self.path.as_path()
	}
}

impl ProjectContextData {
	pub async fn new(ns_id: Option<String>) -> ProjectContext {
		// Read the config
		let project_root = Self::seek_project_root().await;
		let config = ProjectContextData::read_config(project_root.as_path()).await;
		let config_local = ProjectContextData::read_config_local(project_root.as_path()).await;
		let cache = ProjectContextData::read_cache(project_root.as_path()).await;

		let ns_id = ns_id
			.or_else(|| config_local.namespace.clone())
			.expect("no namespace specified, either use the `BOLT_NAMESPACE` environment variable or specify in `Bolt.local.toml`");
		let ns_config = ProjectContextData::read_ns(project_root.as_path(), &ns_id).await;

		let mut svc_ctxs_map = HashMap::new();

		// Load sub projects
		for (_, additional_root) in &config_local.additional_roots {
			let path = project_root.join(&additional_root.path);
			Self::load_root_dir(&mut svc_ctxs_map, path).await;
		}

		// Load main project after sub projects so it overrides sub project services
		// eprintln!("> Loading root");
		Self::load_root_dir(&mut svc_ctxs_map, project_root.clone()).await;

		let mut svc_ctxs = svc_ctxs_map.values().cloned().collect::<Vec<_>>();
		svc_ctxs.sort_by_key(|v| v.name());

		// Create project
		let ctx = Arc::new(ProjectContextData {
			ns_id,
			config,
			config_local,
			ns_config,
			cache: Mutex::new(cache),
			path: project_root,
			svc_ctxs,
			svc_ctxs_map,
		});

		ctx.validate_ns();

		// Assign references to all services after we're done
		for svc in &ctx.svc_ctxs {
			*svc.project.write().await = Arc::downgrade(&ctx);
		}

		ctx
	}

	/// Validates the namespace config.
	fn validate_ns(&self) {
		if self.ns().grafana.is_some() {
			assert!(
				matches!(
					self.ns().dns.provider,
					config::ns::DnsProvider::Cloudflare {
						access: Some(_),
						..
					}
				),
				"cloudflare access must be enabled to use grafana"
			);
		}
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

		panic!("Could not find project root.");
	}
}

impl ProjectContextData {
	async fn load_root_dir(
		svc_ctxs_map: &mut HashMap<String, context::service::ServiceContext>,
		path: PathBuf,
	) {
		let workspace_path = path.join("svc");

		// APIs
		Self::load_services_dir(svc_ctxs_map, &workspace_path, workspace_path.join("api")).await;

		// Packages
		let pkgs_path = workspace_path.join("pkg");
		let mut pkg_dir = fs::read_dir(&pkgs_path).await.unwrap();
		while let Some(pkg) = pkg_dir.next_entry().await.unwrap() {
			// Read worker
			let worker_path = pkg.path().join("worker");
			if fs::metadata(&worker_path.join("Service.toml"))
				.await
				.is_ok()
			{
				// Load the service
				let svc_ctx = context::service::ServiceContextData::from_path(
					Weak::new(),
					&workspace_path,
					&worker_path,
				)
				.await
				.unwrap();
				svc_ctxs_map.insert(svc_ctx.name(), svc_ctx.clone());
			}

			// Read standalone
			Self::load_services_dir(svc_ctxs_map, &workspace_path, pkg.path().join("standalone"))
				.await;

			// Read ops
			Self::load_services_dir(svc_ctxs_map, &workspace_path, pkg.path().join("ops")).await;

			// Read dbs
			Self::load_services_dir(svc_ctxs_map, &workspace_path, pkg.path().join("db")).await;

			// Read buckets
			Self::load_services_dir(svc_ctxs_map, &workspace_path, pkg.path().join("buckets"))
				.await;
		}
	}

	async fn load_services_dir(
		svc_ctxs_map: &mut HashMap<String, context::service::ServiceContext>,
		workspace_path: &Path,
		path: PathBuf,
	) {
		if !path.exists() {
			return;
		}

		let mut dir = fs::read_dir(path).await.unwrap();
		while let Some(entry) = dir.next_entry().await.unwrap() {
			// Check if service config exists
			if fs::metadata(entry.path().join("Service.toml"))
				.await
				.is_err()
			{
				continue;
			}

			// Load the service
			let svc_ctx = context::service::ServiceContextData::from_path(
				Weak::new(),
				workspace_path,
				&entry.path(),
			)
			.await
			.unwrap();

			svc_ctxs_map.insert(svc_ctx.name(), svc_ctx.clone());
		}
	}
}

impl ProjectContextData {
	async fn read_config(project_path: &Path) -> config::project::Project {
		let config_path = project_path.join("Bolt.toml");
		let config_str = fs::read_to_string(config_path).await.unwrap();
		toml::from_str::<config::project::Project>(&config_str).unwrap()
	}

	async fn read_config_local(project_path: &Path) -> config::local::Local {
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
		let config_str = tokio::fs::read_to_string(&path).await.expect(&format!(
			"failed to read namespace config: {}",
			path.display()
		));
		toml::from_str::<config::ns::Namespace>(&config_str)
			.expect("failed to parse namespace config")
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
		self.svc_ctxs.as_slice()
	}

	pub async fn service_with_name(
		self: &Arc<Self>,
		name: &str,
	) -> context::service::ServiceContext {
		if let Some(ctx) = self.svc_ctxs_map.get(name) {
			return ctx.clone();
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

	pub async fn essential_services(self: &Arc<Self>) -> Vec<context::service::ServiceContext> {
		self.all_services()
			.await
			.iter()
			.filter(|x| x.config().service.essential)
			.cloned()
			.collect::<Vec<_>>()
	}

	pub async fn recursive_dependencies_with_pattern(
		self: &Arc<Self>,
		svc_names: &[impl AsRef<str>],
	) -> Vec<ServiceContext> {
		let svc_names = self
			.services_with_patterns(svc_names)
			.await
			.iter()
			.map(|x| x.name())
			.collect::<Vec<String>>();
		self.recursive_dependencies(svc_names.as_slice()).await
	}

	pub async fn recursive_dependencies(
		self: &Arc<Self>,
		svc_names: &[impl AsRef<str>],
	) -> Vec<ServiceContext> {
		// Fetch core services
		let mut all_svc = self.services_with_names(&svc_names).await;

		// Add all dependencies
		let mut pending_deps = all_svc.clone(); // Services whose dependencies still need to be processed
		while !pending_deps.is_empty() {
			// Find all new dependencies
			let mut new_deps = Vec::<ServiceContext>::new();
			for svc_ctx in &pending_deps {
				let dependencies = svc_ctx.dependencies().await;
				for dep_ctx in dependencies {
					// Check if dependency is already registered
					if !all_svc.iter().any(|d| d.name() == dep_ctx.name()) {
						all_svc.push(dep_ctx.clone());
						new_deps.push(dep_ctx.clone());
					}
				}
			}

			// Save new pending dep list
			pending_deps = new_deps;
		}

		all_svc
	}

	pub async fn services_with_migrations(&self) -> Vec<ServiceContext> {
		self.all_services()
			.await
			.iter()
			.filter(|x| x.has_migrations())
			.cloned()
			.collect::<Vec<_>>()
	}
}

impl ProjectContextData {
	pub fn ns_path(&self) -> PathBuf {
		self.path.join("namespaces")
	}

	pub fn secrets_path(&self) -> PathBuf {
		self.path
			.join("secrets")
			.join(format!("{}.toml", self.ns_id()))
	}

	pub fn gen_path(&self) -> PathBuf {
		self.path.join("gen")
	}

	pub fn tf_path(&self) -> PathBuf {
		self.path.join("infra").join("tf")
	}

	pub fn salt_path(&self) -> PathBuf {
		self.path.join("infra").join("salt")
	}
}

impl ProjectContextData {
	/// Origin used for building links to the Hub.
	pub fn origin_hub(&self) -> String {
		self.ns().dns.hub_origin.clone().map_or_else(
			|| self.ns().dns.domain.main.clone(),
			|x| format!("https://hub.{x}"),
		)
	}
	pub fn domain_main(&self) -> String {
		self.ns().dns.domain.main.clone()
	}

	pub fn domain_cdn(&self) -> String {
		self.ns().dns.domain.cdn.clone()
	}

	pub fn domain_job(&self) -> String {
		self.ns().dns.domain.job.clone()
	}

	pub async fn all_router_url_env(self: &Arc<Self>) -> Vec<(String, String)> {
		self.all_services()
			.await
			.iter()
			.filter_map(|svc| {
				svc.config()
					.kind
					.router()
					.and_then(|x| x.mounts.first())
					.map(|x| (svc.clone(), x))
			})
			.map(|(svc, mount)| {
				let domain = match mount.domain {
					ServiceDomain::Base => self.domain_main(),
					ServiceDomain::BaseGame => self.domain_cdn(),
					ServiceDomain::BaseJob => self.domain_job(),
				};

				(
					format!("RIVET_{}_URL", svc.name_screaming_snake()),
					format!(
						"https://{domain}{path}",
						domain = if let Some(subdomain) = &mount.subdomain {
							format!("{}.{}", subdomain, domain)
						} else {
							domain
						},
						path = mount.path.clone().unwrap_or_default(),
					),
				)
			})
			.collect::<Vec<(String, String)>>()
	}
}

impl ProjectContextData {
	pub async fn resolve_image_tag(&self, tag: &str) -> Option<String> {
		if let Some(resolved_tag) = self
			.cache(|x| x.resolved_image_tags.get(tag).cloned())
			.await
		{
			// Use cached tag
			Some(resolved_tag.clone())
		} else {
			// Attempt to fetch the resolved image tag
			if let Some(resolved_tag) = dep::docker::cli::inspect_sha_tag_pull(&tag).await {
				// Cache the image tag if it exists
				{
					let resolved_tag = resolved_tag.clone();
					self.cache_mut(|cache| {
						cache
							.resolved_image_tags
							.insert(tag.to_owned(), resolved_tag);
					})
					.await;
				}

				Some(resolved_tag)
			} else {
				None
			}
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
	pub credentials: S3Credentials,
}

impl ProjectContextData {
	/// Returns the appropriate S3 connection configuration for the provided S3 provider.
	pub async fn s3_credentials(self: &Arc<Self>) -> Result<S3Credentials> {
		// TODO: Add multiple credentials for different services

		match self.ns().s3.provider {
			config::ns::S3Provider::Minio {} => Ok(S3Credentials {
				access_key_id: "admin".into(),
				access_key_secret: self
					.read_secret(&["minio", "users", "root", "password"])
					.await?,
			}),
			config::ns::S3Provider::Backblaze {} => {
				let service_key = s3::fetch_service_key(&self, &["b2", "persistent"]).await?;
				Ok(S3Credentials {
					access_key_id: service_key.key_id,
					access_key_secret: service_key.key,
				})
			}
		}
	}

	/// Returns the appropriate S3 connection configuration for the provided S3 provider.
	pub async fn s3_config(self: &Arc<Self>, credentials: S3Credentials) -> Result<S3Config> {
		// TODO: Add multiple credentials for different services

		match self.ns().s3.provider {
			config::ns::S3Provider::Minio {} => {
				let s3 = terraform::output::read_s3_minio(&*self).await;
				Ok(S3Config {
					endpoint_internal: (*s3.s3_endpoint_internal).clone(),
					endpoint_external: (*s3.s3_endpoint_external).clone(),
					region: (*s3.s3_region).clone(),
					credentials,
				})
			}
			config::ns::S3Provider::Backblaze {} => {
				let s3 = terraform::output::read_s3_backblaze(&*self).await;
				Ok(S3Config {
					endpoint_internal: (*s3.s3_endpoint_internal).clone(),
					endpoint_external: (*s3.s3_endpoint_external).clone(),
					region: (*s3.s3_region).clone(),
					credentials,
				})
			}
		}
	}

	pub async fn s3_client_service_builds(self: &Arc<Self>) -> Result<s3_util::Client> {
		let service_key =
			s3::fetch_service_key(self, &["b2", "bolt_service_builds_upload"]).await?;
		let client = s3_util::Client::new(
			s3::service_builds::BUCKET,
			s3::service_builds::ENDPOINT,
			s3::service_builds::REGION,
			&service_key.key_id,
			&service_key.key,
		)?;
		Ok(client)
	}

	pub fn s3_cors_allowed_origins(&self) -> Vec<String> {
		self.ns()
			.s3
			.cors
			.allowed_origins
			.clone()
			.unwrap_or_else(|| vec![format!("https://{}", self.domain_main())])
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
	pub fn leader_count(&self) -> usize {
		match &self.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => 1,
			config::ns::ClusterKind::Distributed { .. } => 3,
		}
	}

	/// Returns the region which contains the core cluster.
	///
	/// Seldom used in services. Only used to specify the CDN region at the
	/// moment, but that will be deprecated later.
	pub fn primary_region(&self) -> String {
		self.ns()
			.regions
			.iter()
			.find(|(_, x)| x.primary)
			.map(|(x, _)| x.clone())
			.expect("missing primary region")
	}

	/// Species the region or returns "local" for local development.
	///
	/// This is useful for deploying Nomad services from Bolt to know which
	/// region to connect to.
	pub fn primary_region_or_local(&self) -> String {
		match &self.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => "local".to_string(),
			config::ns::ClusterKind::Distributed { .. } => self.primary_region(),
		}
	}
}

impl ProjectContextData {
	/// Which target to build for with the current configuration.
	pub fn rust_build_target(&self) -> context::RustBuildTarget {
		match self.ns().cluster.kind {
			// Build natively, since deploying to the same machine locally.
			config::ns::ClusterKind::SingleNode { .. } => context::RustBuildTarget::Native,
			// Build for MUSL since this will be deployed to a different machine.
			config::ns::ClusterKind::Distributed { .. } => context::RustBuildTarget::Musl,
		}
	}
}

impl ProjectContextData {
	/// Reads a secret from the configured secret datasource.
	pub async fn read_secret(&self, key_path: &[impl AsRef<str>]) -> Result<String> {
		match &self.ns().secrets {
			config::ns::Secrets::File { path } => {
				let path = path.clone().unwrap_or_else(|| self.secrets_path());

				// TODO: RIV-2289
				// Read the config
				let config_str = tokio::fs::read_to_string(&path)
					.await
					.context(format!("failed to read secrets: {}", path.display()))?;
				let config_json = toml::from_str::<serde_json::Value>(&config_str)
					.context("failed to read secrets")?;

				// Extract the value
				let mut current_value = &config_json;
				for component in key_path {
					let component: &str = component.as_ref();

					current_value = current_value.get(component).with_context(|| {
						let path_joined = key_path
							.iter()
							.map(|x| x.as_ref())
							.collect::<Vec<_>>()
							.join("/");
						format!(
							"secret '{path_joined}' does not exist in '{}'",
							path.display()
						)
					})?;
				}

				// Serialize to string
				let value_str = match current_value {
					serde_json::Value::Bool(x) => x.to_string(),
					serde_json::Value::Number(x) => x.to_string(),
					serde_json::Value::String(x) => x.clone(),
					_ => bail!("cannot convert to string: {current_value}"),
				};

				Ok(value_str)
			}
		}
	}

	/// Reads a secret from the configured data source, returning None if not available.
	pub async fn read_secret_opt(&self, key_path: &[impl AsRef<str>]) -> Result<Option<String>> {
		match &self.ns().secrets {
			config::ns::Secrets::File { path } => {
				let path = path.clone().unwrap_or_else(|| self.secrets_path());

				// TODO: RIV-2289
				// Read the config
				let config_str = tokio::fs::read_to_string(&path)
					.await
					.context(format!("failed to read secrets: {}", path.display()))?;
				let config_json = toml::from_str::<serde_json::Value>(&config_str)
					.context("failed to read secrets")?;

				// Extract the value
				let mut current_value = &config_json;
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
	}
}
