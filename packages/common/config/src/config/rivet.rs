use std::{collections::HashMap, path::PathBuf};

use global_error::prelude::*;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::secret::Secret;

pub mod default_ports {
	pub const API: u16 = 8080;
	pub const API_INTERNAL: u16 = 8081;
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

	/// Manages the automatic provisioning of servers that Rivet runs on.
	#[serde(default)]
	pub cluster: Option<Cluster>,

	/// The service that manages Rivet Actors.
	#[serde(default, rename = "orchestrator")]
	pub pegboard: Pegboard,

	/// Configuration for authentication and access control.
	#[serde(default)]
	pub auth: Auth,

	/// Configuration for various tokens used in the system.
	#[serde(default)]
	pub token: Tokens,

	/// Configuration for the public API service.
	#[serde(default)]
	pub api: Api,

	/// Configuration for the internal API service.
	#[serde(default)]
	pub api_internal: ApiInternal,

	/// Configuration for the metrics service.
	#[serde(default)]
	pub metrics: Metrics,

	/// Configuration for the health check service.
	#[serde(default)]
	pub health: Metrics,

	/// Configuration for the tunnel service.
	#[serde(default)]
	pub tunnel: Tunnel,

	/// Configuration for the hub service.
	#[serde(default)]
	pub hub: Hub,

	/// Configuration for DNS management.
	#[serde(default)]
	pub dns: Option<Dns>,

	/// Configuration for telemetry collection.
	#[serde(default)]
	pub telemetry: Telemetry,

	/// Configuration for billing features (Enterprise Edition).
	#[serde(default)]
	pub billing: Option<Billing>,

	/// Configuration for backend features (Enterprise Edition).
	#[serde(default)]
	pub backend: Option<Backend>,

	/// Configuration for test builds.
	#[serde(default)]
	pub test_builds: HashMap<String, TestBuild>,

	/// Deprecated: Configuration for job running.
	#[serde(default)]
	pub job_run: Option<JobRun>,

	/// Deprecated: Configuration for CDN.
	#[serde(default)]
	pub cdn: Option<Cdn>,

	// TODO: Remove these
	#[serde(default)]
	pub profanity_filter_disable: bool,
	#[serde(default)]
	pub upload_nsfw_check_enabled: bool,
	#[serde(default)]
	pub upload_nsfw_error_verbose: bool,
}

impl Default for Rivet {
	fn default() -> Rivet {
		Self {
			namespace: Self::default_namespace(),
			default_cluster_id: None,
			cluster: None,
			tunnel: Default::default(),
			hub: Default::default(),
			pegboard: Pegboard::default(),
			job_run: None,
			auth: Auth::default(),
			token: Tokens::default(),
			api: Api::default(),
			api_internal: ApiInternal::default(),
			metrics: Metrics::default(),
			health: Metrics::default(),
			telemetry: Telemetry::default(),
			cdn: None,
			dns: None,
			billing: None,
			backend: None,
			test_builds: Default::default(),
			profanity_filter_disable: false,
			upload_nsfw_check_enabled: false,
			upload_nsfw_error_verbose: false,
		}
	}
}

impl Rivet {
	fn default_namespace() -> String {
		"rivet".to_string()
	}
}

impl Rivet {
	pub fn cluster(&self) -> GlobalResult<&Cluster> {
		Ok(unwrap_ref!(self.cluster, "cluster disabled"))
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
		let host_str = unwrap!(self.api.public_origin.host_str(), "api origin missing host");
		Ok(host_str.to_string())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RivetAccessKind {
	Public,
	Private,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Billing {
	/// Price ID for the indie tier.
	pub indie_price_id: String,
	/// Price ID for the studio tier.
	pub studio_price_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Backend {
	/// Base domain serving the backend endpoints.
	pub base_domain: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TestBuild {
	/// Image tag.
	pub tag: String,
	/// S3 key.
	pub key: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Api {
	/// The public origin URL for the API.
	pub public_origin: Url,
	/// The port on which the API service listens.
	pub port: u16,
	/// Flag to enable verbose error reporting.
	pub verbose_errors: bool,
}

impl Default for Api {
	fn default() -> Self {
		Self {
			public_origin: url::Url::parse(&format!("http://127.0.0.1:{}", default_ports::API))
				.unwrap(),
			port: default_ports::API,
			verbose_errors: false,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApiInternal {
	pub port: u16,
}

impl Default for ApiInternal {
	fn default() -> Self {
		Self {
			port: default_ports::API_INTERNAL,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cdn {}

impl Default for Cdn {
	fn default() -> Self {
		Self {}
	}
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Cluster {
	/// Configuration for different server pools.
	pub pools: ClusterPools,
	/// Default configuration for new clusters.
	pub default_cluster_config: Option<serde_json::Value>,
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
pub struct Pegboard {
	/// The port on which the Pegboard service listens.
	#[serde(default = "Pegboard::default_port")]
	pub port: u16,
	/// The url for the manager binary.
	#[serde(default)]
	pub manager_binary_url: Option<Url>,
	/// The url for the container runner binary.
	#[serde(default)]
	pub container_runner_binary_url: Option<Url>,
	/// The url for the isolate runner binary.
	#[serde(default)]
	pub isolate_runner_binary_url: Option<Url>,
}

impl Default for Pegboard {
	fn default() -> Self {
		Self {
			port: Self::default_port(),
			manager_binary_url: None,
			container_runner_binary_url: None,
			isolate_runner_binary_url: None,
		}
	}
}

impl Pegboard {
	fn default_port() -> u16 {
		default_ports::PEGBOARD
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct JobRun {
	pub job_runner_binary_url: Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Auth {
	/// Determines whether the Rivet instance is public or private.
	pub access_kind: RivetAccessKind,
	/// Flag to enable access token login.
	pub access_token_login: bool,
}

impl Default for Auth {
	fn default() -> Self {
		Self {
			access_kind: RivetAccessKind::Private,
			access_token_login: true,
		}
	}
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Hub {
	/// The origin URL for the Hub service.
	pub public_origin: Url,
	/// Regular expression to match valid Hub origins.
	pub public_origin_regex: String,
}

impl Default for Hub {
	fn default() -> Hub {
		Self {
			public_origin: url::Url::parse(&format!("http://127.0.0.1:{}", default_ports::API))
				.unwrap(),
			public_origin_regex: format!("^http:\\/\\/:127\\.0\\.0\\.1:{}", default_ports::API),
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Tokens {
	/// Token for the API Traefik provider.
	pub api_traefik_provider: Option<Secret<String>>,
	/// Token for API status checks.
	pub api_status: Option<Secret<String>>,
	/// Token for API admin access.
	pub api_admin: Option<Secret<String>>,
}

impl Default for Tokens {
	fn default() -> Tokens {
		Self {
			api_traefik_provider: None,
			api_status: None,
			api_admin: None,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Health {
	pub port: u16,
}

impl Default for Health {
	fn default() -> Self {
		Self {
			port: default_ports::HEALTH,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Metrics {
	pub port: u16,
}

impl Default for Metrics {
	fn default() -> Self {
		Self {
			port: default_ports::METRICS,
		}
	}
}

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
