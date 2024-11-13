use global_error::prelude::*;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::IpAddr, path::PathBuf};
use url::Url;
use uuid::Uuid;

use crate::secret::Secret;

pub mod default_dev_cluster {
	use uuid::{uuid, Uuid};

	// These are intentionally hardcoded in order to simplify default dev configuration.

	pub const CLUSTER_ID: Uuid = uuid!("11ca8960-acab-4963-909c-99d72af3e1cb");
	pub const DATACENTER_ID: Uuid = uuid!("f288913c-735d-4188-bf9b-2fcf6eac7b9c");
}

pub mod default_hosts {
	use std::net::{IpAddr, Ipv4Addr};

	// Public services using public interface
	pub const API_PUBLIC: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
	pub const API_EDGE: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
	pub const PEGBOARD: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
	pub const TUNNEL: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

	// Private services using loopback interface
	pub const HEALTH: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
	pub const METRICS: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
}

pub mod default_ports {
	pub const API_PUBLIC: u16 = 8080;
	pub const API_EDGE: u16 = 8081;
	pub const PEGBOARD: u16 = 8082;
	pub const TUNNEL: u16 = 8003;

	pub const HEALTH: u16 = 9000;
	pub const METRICS: u16 = 9001;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Rivet {
	/// IMPORTANT: Do not change this value after the first time starting a cluster with this
	/// namespace.
	#[serde(default = "Rivet::default_namespace")]
	pub namespace: String,

	/// If specified, will use this as the default cluster ID.
	///
	/// This will have no effect if applied after the cluster has first ran.
	#[serde(default)]
	pub default_cluster_id: Option<Uuid>,

	#[serde(default)]
	pub clusters: Option<HashMap<String, Cluster>>,

	/// Configures how servers are provisioned.
	///
	/// Enterprise only.
	#[serde(default)]
	pub provision: Option<ClusterProvision>,

	#[serde(default, rename = "orchestrator")]
	pub pegboard: Pegboard,

	#[serde(default)]
	pub auth: Auth,

	#[serde(default)]
	pub token: Tokens,

	#[serde(default)]
	pub api_public: ApiPublic,

	#[serde(default)]
	pub api_edge: ApiEdge,

	#[serde(default)]
	pub metrics: Metrics,

	#[serde(default)]
	pub health: Health,

	#[serde(default)]
	pub tunnel: Tunnel,

	#[serde(default)]
	pub ui: Ui,

	#[serde(default)]
	pub dns: Option<Dns>,

	#[serde(default)]
	pub telemetry: Telemetry,

	#[serde(default)]
	pub billing: Option<Billing>,

	#[serde(default)]
	pub backend: Option<Backend>,

	/// Configuration for test builds.
	#[serde(default)]
	pub test_builds: HashMap<String, TestBuild>,

	#[serde(default)]
	pub job_run: Option<JobRun>,

	#[serde(default)]
	pub cdn: Option<Cdn>,
}

impl Default for Rivet {
	fn default() -> Rivet {
		Self {
			namespace: Self::default_namespace(),
			default_cluster_id: None,
			clusters: None,
			provision: None,
			tunnel: Default::default(),
			ui: Default::default(),
			pegboard: Pegboard::default(),
			job_run: None,
			auth: Auth::default(),
			token: Tokens::default(),
			api_public: ApiPublic::default(),
			api_edge: ApiEdge::default(),
			metrics: Metrics::default(),
			health: Health::default(),
			telemetry: Telemetry::default(),
			cdn: None,
			dns: None,
			billing: None,
			backend: None,
			test_builds: Default::default(),
		}
	}
}

impl Rivet {
	fn default_namespace() -> String {
		"rivet".to_string()
	}
}

impl Rivet {
	pub fn default_cluster_id(&self) -> GlobalResult<Uuid> {
		if let Some(default_cluster_id) = self.default_cluster_id {
			ensure!(
				self.clusters().values().any(|x| x.id == default_cluster_id),
				"default cluster id does not specify a configured cluster"
			);
			Ok(default_cluster_id)
		} else {
			match self.auth.access_kind {
				// Return default development clusters
				AccessKind::Development => Ok(default_dev_cluster::CLUSTER_ID),
				// No cluster configured
				AccessKind::Public | AccessKind::Private => {
					bail!("`default_cluster_id` not configured")
				}
			}
		}
	}

	pub fn clusters(&self) -> HashMap<String, Cluster> {
		if let Some(clusters) = self.clusters.clone() {
			// Return configured clusters
			clusters
		} else {
			match self.auth.access_kind {
				// Return default development clusters
				AccessKind::Development => {
					// TODO: pull this from util::dev_defaults::CLUSTER_SLUG
					maplit::hashmap! {
						"default".into() => Cluster::build_dev_cluster(),
					}
				}
				// No cluster configured
				AccessKind::Public | AccessKind::Private => HashMap::new(),
			}
		}
	}

	pub fn provision(&self) -> GlobalResult<&ClusterProvision> {
		Ok(unwrap_ref!(self.provision, "provision disabled"))
	}

	pub fn job_run(&self) -> GlobalResult<&JobRun> {
		Ok(unwrap_ref!(self.job_run, "job run disabled"))
	}

	pub fn dns(&self) -> GlobalResult<&Dns> {
		Ok(unwrap_ref!(self.dns, "dns disabled"))
	}

	pub fn billing(&self) -> GlobalResult<&Billing> {
		Ok(unwrap_ref!(self.billing, "billing disabled"))
	}

	pub fn api_host(&self) -> GlobalResult<String> {
		let public_origin = self.api_public.public_origin();
		let host_str = unwrap!(public_origin.host_str(), "api origin missing host");
		Ok(host_str.to_string())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AccessKind {
	/// Anyone can sign up for an account.
	Public,
	/// Only admin users can crate teams & projects.
	Private,
	/// Anyone can access the cluster without authorization.
	///
	/// If enabled:
	/// - A default project with slug "default" & environment with slug "default" will be created
	/// automatically
	///	  - This allows using Rivet without manually creating a new project/environment
	/// - Project & environment fields will fallback to "default" if not provided
	///   - This allows using Rivet with simplfied requests, like `POST /actors` without a query
	/// - If no bearer token is provided, authentication will always succeed
	///	  - This allows setting up development environments without manually creating tokens
	Development,
}

/// Configuration for billing features (Enterprise Edition).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Billing {
	/// Price ID for the indie tier.
	pub indie_price_id: String,
	/// Price ID for the studio tier.
	pub studio_price_id: String,
}

/// Configuration for backend features (Enterprise Edition).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Backend {
	/// Base domain serving the backend endpoints.
	pub base_domain: String,
}

/// Configuration for a default test build.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestBuild {
	/// Image tag.
	pub tag: String,
	/// S3 key.
	pub key: PathBuf,
}

/// Configuration for the public API service.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApiPublic {
	/// The public origin URL for the API.
	pub public_origin: Option<Url>,
	/// The host on which the API service listens.
	pub host: Option<IpAddr>,
	/// The port on which the API service listens.
	pub port: Option<u16>,
	/// Flag to enable verbose error reporting.
	pub verbose_errors: Option<bool>,
	/// Flag to respect the X-Forwarded-For header for client IP addresses.
	///
	/// Will be ignored in favor of CF-Connecting-IP if DNS provider is
	/// configured as Cloudflare.
	pub respect_forwarded_for: Option<bool>,
}

impl ApiPublic {
	pub fn public_origin(&self) -> Url {
		self.public_origin.clone().unwrap_or_else(|| {
			url::Url::parse(&format!("http://127.0.0.1:{}", self.port())).unwrap()
		})
	}

	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(default_hosts::API_PUBLIC)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(default_ports::API_PUBLIC)
	}

	pub fn verbose_errors(&self) -> bool {
		self.verbose_errors.unwrap_or(true)
	}

	pub fn respect_forwarded_for(&self) -> bool {
		self.respect_forwarded_for.unwrap_or(false)
	}
}

/// Configuration for the edge API service.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApiEdge {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
}

impl ApiEdge {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(default_hosts::API_EDGE)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(default_ports::API_EDGE)
	}
}

/// Deprecated: Configuration for CDN.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cdn {}

impl Default for Cdn {
	fn default() -> Self {
		Self {}
	}
}

/// Configuration for DNS management.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Dns {
	/// The DNS provider used for managing domains.
	pub provider: DnsProvider,
	// TODO: Remove this once we can remove the use of `gg_cert`
	pub domain_main: Option<String>,
	/// The domain used for job-related services.
	pub domain_job: Option<String>,
	/// The domain used for CDN-related services.
	pub domain_cdn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DnsProvider {
	Cloudflare,
}

/// Manages the automatic provisioning of servers that Rivet runs on.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cluster {
	/// This ID must not change.
	pub id: Uuid,
	pub datacenters: HashMap<String, Datacenter>,
}

impl Cluster {
	fn build_dev_cluster() -> Self {
		Cluster {
			id: default_dev_cluster::CLUSTER_ID,
			datacenters: hashmap! {
				"local".into() => Datacenter {
					// Intentionally hardcoded in order to simplify default dev configuration
					id: default_dev_cluster::DATACENTER_ID,
					// TODO: Pull from dev_defaults
					name: "Local".into(),
					// ATS is not running in local dev
					build_delivery_method: BuildDeliveryMethod::S3Direct,
					// Placeholder values for the dev node
					hardware: Some(DatacenterHardware { cpu_cores: 4, cpu: 3_000 * 4, memory: 8_192, disk: 32_768, bandwidth: 1_000_000 }),
					provision: None,
				},
			},
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterProvision {
	/// Configuration for server pools that use a margin for scaling.
	pub pools: ClusterPools,

	/// The URL for the manager binary.
	pub manager_binary_url: Url,

	/// The URL for the container runner binary.
	pub container_runner_binary_url: Url,

	/// The URL for the isolate runner binary.
	pub isolate_runner_binary_url: Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPools {
	pub job: ClusterPool,
	pub pegboard: ClusterPool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClusterPool {
	pub provision_margin: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Datacenter {
	/// This ID must not change.
	pub id: Uuid,

	pub name: String,

	pub build_delivery_method: BuildDeliveryMethod,

	/// Hardware specs used to orchestrate jobs.
	///
	/// This is only used if `provision` is not provided.
	///
	/// This will be automatically determined in development mode.
	#[serde(default)]
	pub hardware: Option<DatacenterHardware>,

	// #[serde(default)]
	// pub reserve_resources: Option<ReserveResources>,
	/// Configures how servers are provisioned.
	///
	/// Enterprise only.
	#[serde(default)]
	pub provision: Option<DatacenterProvision>,
}

impl Datacenter {
	pub fn pools(&self) -> HashMap<PoolType, Pool> {
		self.provision
			.as_ref()
			.map_or_else(|| HashMap::new(), |x| x.pools.clone())
	}

	pub fn prebakes_enabled(&self) -> bool {
		self.provision
			.as_ref()
			.map_or(false, |x| x.prebakes_enabled)
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DatacenterHardware {
	pub cpu_cores: u32,
	/// Mhz
	pub cpu: u32,
	/// MiB
	pub memory: u32,
	/// MiB
	pub disk: u32,
	/// Kibps
	pub bandwidth: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DatacenterProvision {
	pub provider: Provider,
	pub provider_datacenter_id: String,
	pub pools: HashMap<PoolType, Pool>,
	pub prebakes_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Provider {
	Linode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Pool {
	pub hardware: Vec<Hardware>,
	pub desired_count: u32,
	pub min_count: u32,
	pub max_count: u32,
	pub drain_timeout: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PoolType {
	Job,
	Gg,
	Ats,
	Pegboard,
	PegboardIsolate,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Hardware {
	pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BuildDeliveryMethod {
	TrafficServer,
	S3Direct,
}

/// The service that manages Rivet Actors.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Pegboard {
	/// The host on which the Pegboard service listens.
	pub host: Option<IpAddr>,
	/// The port on which the Pegboard service listens.
	pub port: Option<u16>,
}

impl Pegboard {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(default_hosts::PEGBOARD)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(default_ports::PEGBOARD)
	}
}

/// Deprecated: Configuration for job running.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct JobRun {
	pub job_runner_binary_url: Url,
}

/// Configuration for authentication and access control.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Auth {
	pub access_kind: AccessKind,
}

impl Default for Auth {
	fn default() -> Self {
		Self {
			access_kind: AccessKind::Private,
		}
	}
}

/// Configuration for the tunnel service.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Tunnel {
	pub public_host: String,
}

impl Default for Tunnel {
	fn default() -> Tunnel {
		Self {
			public_host: format!("127.0.0.1:{}", default_ports::TUNNEL),
		}
	}
}

/// Configuration for the UI service.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Ui {
	/// Enables serving the UI automatically.
	///
	/// If disabled, the UI can be hosted separately.
	pub enable: Option<bool>,
	/// The origin URL for the UI.
	pub public_origin: Option<Url>,
	/// Regular expression to match valid UI origins.
	pub public_origin_regex: Option<String>,
}

impl Ui {
	pub fn enable(&self) -> bool {
		self.enable.unwrap_or(true)
	}

	pub fn public_origin(&self) -> Url {
		self.public_origin.clone().unwrap_or_else(|| {
			Url::parse(&format!("http://127.0.0.1:{}", default_ports::API_PUBLIC)).unwrap()
		})
	}

	pub fn public_origin_regex(&self) -> String {
		self.public_origin_regex.clone().unwrap_or_else(|| {
			format!(
				"^https?://(?:localhost|127\\.0\\.0\\.1|\\[::1\\]|\\[::\\]|0\\.0\\.0\\.0):{}",
				default_ports::API_PUBLIC
			)
		})
	}
}

/// Configuration for various tokens used in the system.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Tokens {
	/// Token for the API Traefik provider.
	pub traefik_provider: Option<Secret<String>>,
	/// Token for API status checks.
	pub status: Option<Secret<String>>,
}

impl Default for Tokens {
	fn default() -> Tokens {
		Self {
			traefik_provider: None,
			status: None,
		}
	}
}

/// Configuration for the health check service.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Health {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
}

impl Health {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(default_hosts::HEALTH)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(default_ports::HEALTH)
	}
}

/// Configuration for the metrics service.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Metrics {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
}

impl Metrics {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(default_hosts::METRICS)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(default_ports::METRICS)
	}
}

/// Configuration for telemetry collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Telemetry {
	/// Flag to enable or disable telemetry collection.
	pub enable: bool,
}

impl Default for Telemetry {
	fn default() -> Self {
		Telemetry { enable: true }
	}
}
