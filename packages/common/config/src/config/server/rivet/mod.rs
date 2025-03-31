use global_error::prelude::*;
use maplit::hashmap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	net::{IpAddr, SocketAddr},
	path::PathBuf,
};
use url::Url;
use uuid::Uuid;

use crate::secret::Secret;

pub mod cluster_provision;

pub use cluster_provision::*;

pub mod default_dev_cluster {
	use uuid::{uuid, Uuid};

	// These are intentionally hardcoded in order to simplify default dev configuration.
	pub const CLUSTER_ID: Uuid = uuid!("11ca8960-acab-4963-909c-99d72af3e1cb");
	pub const DATACENTER_ID: Uuid = uuid!("f288913c-735d-4188-bf9b-2fcf6eac7b9c");
	pub const SERVER_ID: Uuid = uuid!("174aca2a-98b7-462c-9ad9-3835094a9a10");

	// Routes via Rivet Guard instead of directly to edge server
	pub const EDGE_API_URL: &str = "http://rivet-guard:7080";

	// In dev, there are no servers to pull the addresses from. We need to have a fallback address.
	pub const DEV_EDGE_API_FALLBACK_ADDR_LAN_HOST: &str = "rivet-edge-server";
pub const DEV_EDGE_API_FALLBACK_ADDR_LAN_PORT: u16 = 8080;
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

pub(crate) mod default_ports {
	pub const API_PUBLIC: u16 = 8080;
	pub const API_EDGE: u16 = 8081;
	pub const PEGBOARD: u16 = 8082;
	pub const TUNNEL: u16 = 8003;

	pub const HEALTH: u16 = 8090;
	pub const METRICS: u16 = 8091;
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
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
	pub provision: Option<cluster_provision::ClusterProvision>,

	#[serde(default, rename = "orchestrator")]
	pub pegboard: Pegboard,

	#[serde(default)]
	pub guard: Guard,

	#[serde(default)]
	pub auth: Auth,

	#[serde(default)]
	pub api_public: ApiPublic,

	#[serde(default)]
	pub api_edge: ApiEdge,

	#[serde(default)]
	pub metrics: Metrics,

	#[serde(default)]
	pub health: Health,

	#[serde(default)]
	pub status: Option<Status>,

	#[serde(default)]
	pub cache: Cache,

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

	/// Configuration for edge clusters. Should be null on the core cluster.
	#[serde(default)]
	pub edge: Option<Edge>,
}

impl Default for Rivet {
	fn default() -> Rivet {
		Self {
			namespace: Self::default_namespace(),
			default_cluster_id: None,
			clusters: None,
			provision: None,
			tunnel: Tunnel::default(),
			cache: Cache::default(),
			ui: Ui::default(),
			pegboard: Pegboard::default(),
			guard: Guard::default(),
			job_run: None,
			auth: Auth::default(),
			api_public: ApiPublic::default(),
			api_edge: ApiEdge::default(),
			metrics: Metrics::default(),
			health: Health::default(),
			status: None,
			telemetry: Telemetry::default(),
			cdn: None,
			dns: None,
			billing: None,
			backend: None,
			test_builds: Default::default(),
			edge: None,
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

	pub fn provision(&self) -> GlobalResult<&cluster_provision::ClusterProvision> {
		Ok(unwrap_ref!(self.provision, "provision disabled"))
	}

	pub fn job_run(&self) -> GlobalResult<&JobRun> {
		Ok(unwrap_ref!(self.job_run, "job run disabled"))
	}

	pub fn dns(&self) -> GlobalResult<&Dns> {
		Ok(unwrap_ref!(self.dns, "dns disabled"))
	}

	pub fn domain_job(&self) -> GlobalResult<&str> {
		let domain_job = unwrap!(
			self.dns().ok().and_then(|dns| dns.domain_job.as_ref()),
			"unable to get dns.domain_job to build actor hostname. configure dns or switch to host networking."
		);

		Ok(domain_job)
	}

	pub fn domain_main(&self) -> GlobalResult<&str> {
		let domain_main = unwrap!(
			self.dns().ok().and_then(|dns| dns.domain_main.as_ref()),
			"unable to get dns.domain_main"
		);

		Ok(domain_main)
	}

	pub fn edge_api_url(&self, dc_name_id: &str) -> GlobalResult<Url> {
		match self.auth.access_kind {
			AccessKind::Development => {
				Url::parse(default_dev_cluster::EDGE_API_URL).map_err(Into::into)
			}
			AccessKind::Public | AccessKind::Private => {
				let domain_main = self.domain_main()?;

				Url::parse(&format!("https://api.{dc_name_id}.{domain_main}")).map_err(Into::into)
			}
		}
	}

	// Does what `rivet_util::url::to_string_without_slash` does
	pub fn edge_api_url_str(&self, dc_name_id: &str) -> GlobalResult<String> {
		Ok(self
			.edge_api_url(dc_name_id)?
			.to_string()
			.trim_end_matches('/')
			.to_string())
	}

	/// Host to use for routing traffic from Rivet Guard to the edge.
	///
	/// If `None`, accepts all host.
	///
	/// If `Some`, then only accepts that host.
	pub fn edge_api_routing_host(&self, dc_name_id: &str) -> GlobalResult<Option<String>> {
		match self.auth.access_kind {
			AccessKind::Development => Ok(None),
			AccessKind::Public | AccessKind::Private => {
				let domain_main = self.domain_main()?;
				Ok(Some(format!("api.{dc_name_id}.{domain_main}")))
			}
		}
	}

	pub fn edge_api_fallback_addr_lan(&self) -> Option<(String, u16)> {
		if let Some(edge) = &self.edge {
			if let Some(lan_addr) = &edge.api_lan_address {
				Some(lan_addr.clone())
			} else {
				match self.auth.access_kind {
					AccessKind::Development => {
						Some((
							default_dev_cluster::DEV_EDGE_API_FALLBACK_ADDR_LAN_HOST.to_string(),
							default_dev_cluster::DEV_EDGE_API_FALLBACK_ADDR_LAN_PORT
						))
					}
					AccessKind::Public | AccessKind::Private => None,
				}
			}
		} else {
			None
		}
	}

	pub fn billing(&self) -> GlobalResult<&Billing> {
		Ok(unwrap_ref!(self.billing, "billing disabled"))
	}

	pub fn api_host(&self) -> GlobalResult<String> {
		let public_origin = self.api_public.public_origin();
		let host_str = unwrap!(public_origin.host_str(), "api origin missing host");
		Ok(host_str.to_string())
	}

	pub fn status(&self) -> GlobalResult<&Status> {
		Ok(unwrap_ref!(self.status, "status api disabled"))
	}

	pub fn edge(&self) -> GlobalResult<&Edge> {
		Ok(unwrap_ref!(self.edge, "edge disabled"))
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AccessKind {
	/// Anyone can sign up for an account.
	Public,
	/// Only admin users can create teams & projects.
	Private,
	/// Anyone can access the cluster without authorization.
	///
	/// If enabled:
	/// - A default project with slug "default" & environment with slug "default" will be created
	/// automatically
	///	  - This allows using Rivet without manually creating a new project/environment
	/// - Project & environment fields will fallback to "default" if not provided
	///   - This allows using Rivet with simplified requests, like `POST /actors` without a query
	/// - If no bearer token is provided, authentication will always succeed
	///	  - This allows setting up development environments without manually creating tokens
	Development,
}

/// Configuration for billing features (Enterprise Edition).
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Billing {
	/// Price ID for the indie tier.
	pub indie_price_id: String,
	/// Price ID for the studio tier.
	pub studio_price_id: String,
}

/// Configuration for backend features (Enterprise Edition).
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Backend {
	/// Base domain serving the backend endpoints.
	pub base_domain: String,
}

/// Configuration for a default test build.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestBuild {
	/// Image tag.
	pub tag: String,
	/// S3 key.
	pub key: PathBuf,
}

/// Configuration for the public API service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
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
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApiEdge {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,

	/// Token for the API Traefik provider.
	pub traefik_provider_token: Option<Secret<String>>,
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
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[derive(Default)]
pub struct Cdn {}

/// Configuration for DNS management.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Dns {
	/// The DNS provider used for managing domains.
	pub provider: DnsProvider,
	/// The domain used for backend services.
	pub domain_main: Option<String>,
	/// The domain used for job-related services.
	pub domain_job: Option<String>,
	/// The domain used for CDN-related services.
	pub domain_cdn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DnsProvider {
	Cloudflare,
}

/// Manages the automatic provisioning of servers that Rivet runs on.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cluster {
	/// This ID must not change.
	pub id: Uuid,
	/// Datacenters to automatically be created on cluster boot.
	///
	/// This should only be used for manual cluster creation. Do not use for enterprise
	/// distributions.
	pub bootstrap_datacenters: HashMap<String, Datacenter>,
}

impl Cluster {
	fn build_dev_cluster() -> Self {
		Cluster {
			id: default_dev_cluster::CLUSTER_ID,
			bootstrap_datacenters: hashmap! {
				"local".into() => Datacenter {
					// Intentionally hardcoded in order to simplify default dev configuration
					id: default_dev_cluster::DATACENTER_ID,
					// TODO: Pull from dev_defaults
					name: "Local".into(),
					// ATS is not running in local dev
					build_delivery_method: BuildDeliveryMethod::S3Direct,
					guard: DatacenterGuard {
						public_hostname: Some(GuardPublicHostname::Static("127.0.0.1".into())),
					},
					// Placeholder values for the dev node
					hardware: Some(DatacenterHardware { cpu_cores: 4, cpu: 3_000 * 4, memory: 8_192, disk: 32_768, bandwidth: 1_000_000 }),
				},
			},
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GuardPublicHostname {
	/// Rivet Guard has the appropriate wildcard DNS addresses set up for host-based routing. This
	/// is the preferred option.
	///
	/// This is the parent address that all subdomains will be build from.
	///
	/// If SSL certs are provided, this should also be the name of the TLS parent host.
	///
	/// This will default to hostname endpoint types (see `pegboard::types::EndpointType`).
	DnsParent(String),
	/// Rivet Guard is only accessible by IP or DNS address without a wildcard.
	///
	/// This is usually used for routing to a static IP address on simple architectures.
	///
	/// If the developer cannot set up wildcard DNS addresses for whatever reason, this method can
	/// be used.
	///
	/// This will default to path endpoint types (see `pegboard::types::EndpointType`).
	Static(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Datacenter {
	/// This ID must not change.
	pub id: Uuid,

	pub name: String,

	pub build_delivery_method: BuildDeliveryMethod,

	#[serde(default)]
	pub guard: DatacenterGuard,

	/// Hardware specs used to orchestrate jobs.
	///
	/// This is only used if `provision` is not provided.
	///
	/// This will be automatically determined in development mode.
	#[serde(default)]
	pub hardware: Option<DatacenterHardware>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DatacenterGuard {
	/// If not specified, will attempt to fallback to auto-generated wildcard hosts based on
	/// domain_job or error if domain_job is not provided.
	pub public_hostname: Option<GuardPublicHostname>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DatacenterHardware {
	pub cpu_cores: u32,
	/// Mhz.
	pub cpu: u32,
	/// MiB.
	pub memory: u32,
	/// MiB.
	pub disk: u32,
	/// Kibps.
	pub bandwidth: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BuildDeliveryMethod {
	TrafficServer,
	S3Direct,
}

/// The service that manages Rivet Actors.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum EndpointType {
	Hostname,
	Path,
}

/// The port ranges define what ports Guard will allocate ports on. If using cluster
/// provisioning, these are also used for firewall rules.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Guard {
	pub tls_enabled: Option<bool>,
	pub http_port: Option<u16>,
	pub https_port: Option<u16>,
	pub min_ingress_port_tcp: Option<u16>,
	pub max_ingress_port_tcp: Option<u16>,
	pub min_ingress_port_udp: Option<u16>,
	pub max_ingress_port_udp: Option<u16>,
}

impl Guard {
	pub fn tls_enabled(&self) -> bool {
		self.tls_enabled.unwrap_or(true)
	}

	pub fn http_port(&self) -> u16 {
		self.http_port.unwrap_or(80)
	}

	pub fn https_port(&self) -> u16 {
		self.http_port.unwrap_or(443)
	}

	pub fn min_ingress_port_tcp(&self) -> u16 {
		self.min_ingress_port_tcp.unwrap_or(20000)
	}

	pub fn max_ingress_port_tcp(&self) -> u16 {
		self.max_ingress_port_tcp.unwrap_or(31999)
	}

	pub fn min_ingress_port_udp(&self) -> u16 {
		self.min_ingress_port_udp.unwrap_or(20000)
	}

	pub fn max_ingress_port_udp(&self) -> u16 {
		self.max_ingress_port_udp.unwrap_or(31999)
	}
}

/// Deprecated: Configuration for job running.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct JobRun {
	pub job_runner_binary_url: Url,
}

/// Configuration for authentication and access control.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
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
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
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

/// Configuration for the cache layer.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cache {
	pub driver: CacheDriver,
}

impl Default for Cache {
	fn default() -> Cache {
		Self {
			driver: CacheDriver::Redis,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CacheDriver {
	Redis,
	InMemory,
}

/// Configuration for the UI service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
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
			Url::parse(&format!(
				"http://127.0.0.1:{}/ui",
				default_ports::API_PUBLIC
			))
			.unwrap()
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

/// Configuration for the health check service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
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

/// Configure the status check API.
///
/// These are different than the health check API since they check the internals of the Rivet
/// system.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Status {
	pub token: Secret<String>,
	pub system_test_project: Option<String>,
	pub system_test_environment: Option<String>,
}

/// Configuration for the metrics service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
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
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
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

/// Configuration for edge clusters.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Edge {
	pub cluster_id: Uuid,
	pub datacenter_id: Uuid,
	pub server_id: Uuid,
	pub intercom_endpoint: Url,
	/// This API address will be used if there are no worker servers listed in the cluster package
	#[serde(default)]
	pub api_lan_address: Option<(String, u16)>,
	#[serde(default)]
	pub redirect_logs: Option<bool>,
}
