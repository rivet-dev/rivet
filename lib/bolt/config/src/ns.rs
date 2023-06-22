use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

pub fn decode(s: &str) -> Result<Namespace, toml::de::Error> {
	toml::from_str(s)
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
	pub cluster: Cluster,
	#[serde(default)]
	pub secrets: Secrets,
	#[serde(default = "default_regions")]
	pub regions: HashMap<String, Region>,
	pub pools: Vec<Pool>,
	#[serde(default)]
	pub terraform: Terraform,
	pub dns: Dns,
	pub s3: S3,
	pub email: Option<Email>,
	#[serde(default)]
	pub captcha: Captcha,
	pub logging: Option<Logging>,
	#[serde(default)]
	pub services: HashMap<String, Service>,
	#[serde(default)]
	pub docker: Docker,
	#[serde(default)]
	pub grafana: Option<Grafana>,
	#[serde(default)]
	pub nomad: Nomad,
	#[serde(default)]
	pub traefik: Traefik,
	#[serde(default)]
	pub rust: Rust,
	#[serde(default)]
	pub rivet: Rivet,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
	/// Unique identifier for this cluster.
	///
	/// Should not be changed.
	pub id: Uuid,
	#[serde(flatten)]
	pub kind: ClusterKind,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum ClusterKind {
	#[serde(rename = "single_node")]
	SingleNode {
		public_ip: String,
		#[serde(default)]
		preferred_subnets: Vec<String>,
	},
	#[serde(rename = "distributed")]
	Distributed {
		salt_master_size: String,
		nebula_lighthouse_size: String,
	},
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum Secrets {
	#[serde(rename = "file")]
	File { path: Option<PathBuf> },
}

impl Default for Secrets {
	fn default() -> Self {
		Self::File { path: None }
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Region {
	#[serde(default)]
	pub primary: bool,
	pub id: String,
	pub provider: String,
	pub provider_region: String,
	pub netnum: usize,
	#[serde(default)]
	pub supports_vlan: bool,
	#[serde(default)]
	pub preferred_subnets: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Pool {
	pub pool: String,
	pub version: String,
	pub region: String,
	pub count: usize,
	pub size: String,
	pub netnum: usize,
	#[serde(default)]
	pub volumes: HashMap<String, Volume>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Volume {
	pub size: usize,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum ProviderKind {
	#[serde(rename = "linode")]
	Linode {},
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Terraform {
	#[serde(default)]
	pub backend: TerraformBackend,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum TerraformBackend {
	#[serde(rename = "local")]
	Local {},
	#[serde(rename = "postgres")]
	Postgres {},
}

impl Default for TerraformBackend {
	fn default() -> Self {
		TerraformBackend::Local {}
	}
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Dns {
	pub domain: DnsDomains,
	#[serde(default)]
	pub hub_origin: Option<String>,
	#[serde(flatten)]
	pub provider: DnsProvider,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DnsDomains {
	pub main: String,
	pub job: String,
	pub cdn: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum DnsProvider {
	#[serde(rename = "cloudflare")]
	Cloudflare {
		account_id: String,
		zones: CloudflareZones,
		access: Option<CloudflareAccess>,
	},
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CloudflareZones {
	pub root: String,
	pub game: String,
	pub job: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CloudflareAccess {
	pub groups: CloudflareAccessGroups,
	pub services: CloudflareAccessServices,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CloudflareAccessGroups {
	pub engineering: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CloudflareAccessServices {
	pub terraform_nomad: String,
	pub bolt: String,
	pub grafana_cloud: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct S3 {
	#[serde(default)]
	pub cors: S3Cors,
	#[serde(flatten)]
	pub provider: S3Provider,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct S3Cors {
	#[serde(default)]
	pub allowed_origins: Option<Vec<String>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum S3Provider {
	#[serde(rename = "minio")]
	Minio {},
	#[serde(rename = "backblaze")]
	Backblaze {},
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Email {
	#[serde(flatten)]
	pub provider: EmailProvider,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum EmailProvider {
	#[serde(rename = "sendgrid")]
	SendGrid {},
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Captcha {
	#[serde(default)]
	pub hcaptcha: Option<Hcaptcha>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Hcaptcha {
	pub site_keys: HcaptchaSiteKeys,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HcaptchaSiteKeys {
	pub easy: String,
	pub moderate: String,
	pub difficult: String,
	pub always_on: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Logging {
	#[serde(flatten)]
	pub provider: LoggingProvider,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum LoggingProvider {
	#[serde(rename = "loki")]
	Loki { endpoint: String },
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Service {
	pub count: usize,
	pub resources: ServiceResources,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ServiceResources {
	#[serde(flatten)]
	pub cpu: CpuResources,
	pub memory: usize,
	pub ephemeral_disk: usize,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum CpuResources {
	#[serde(rename = "cpu_cores")]
	CpuCores(usize),
	/// MHz
	#[serde(rename = "cpu")]
	Cpu(usize),
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Docker {
	/// Provides authentication for Docker when pulling public images.
	///
	/// This is useful to prevent hitting rate limits when pulling Docker images.
	///
	/// See [here](https://docs.docker.com/docker-hub/download-rate-limit) for
	/// more information on Docker Hub's rate limits.
	pub authenticate_all_docker_hub_pulls: bool,
}

impl Default for Docker {
	fn default() -> Self {
		Docker {
			authenticate_all_docker_hub_pulls: false,
		}
	}
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Grafana {}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Nomad {
	pub health_checks: Option<bool>,
}

impl Default for Nomad {
	fn default() -> Self {
		Self {
			health_checks: None,
		}
	}
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Traefik {
	pub log_level: String,
	pub access_logs: bool,
}

impl Default for Traefik {
	fn default() -> Self {
		Self {
			log_level: "ERROR".into(),
			access_logs: false,
		}
	}
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Rust {
	#[serde(default)]
	pub build_opt: RustBuildOpt,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub enum RustBuildOpt {
	Release,
	#[default]
	Debug,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Rivet {
	#[serde(default)]
	pub telemetry: Telemetry,
	#[serde(default)]
	pub test: Option<RivetTest>,
	#[serde(default)]
	pub api: Api,
	#[serde(default)]
	pub profanity: Profanity,
	#[serde(default)]
	pub upload: Upload,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Telemetry {
	/// Disables sending telemetry to Rivet.
	#[serde(default)]
	pub disable: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct RivetTest {}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Api {
	pub error_verbose: bool,
	pub hub_origin_regex: Option<String>,
}

impl Default for Api {
	fn default() -> Self {
		Self {
			error_verbose: false,
			hub_origin_regex: None,
		}
	}
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Profanity {
	pub filter_disable: bool,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Upload {
	pub nsfw_error_verbose: bool,
}

fn default_regions() -> HashMap<String, Region> {
	toml::from_str(include_str!("../default_regions.toml"))
		.expect("failed to parse default_regions.toml")
}
