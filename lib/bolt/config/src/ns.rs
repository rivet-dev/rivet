use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

pub fn decode(s: &str) -> Result<Namespace, toml::de::Error> {
	toml::from_str(s)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Namespace {
	pub cluster: Cluster,
	#[serde(default)]
	pub secrets: Secrets,
	#[serde(default = "default_regions")]
	pub regions: HashMap<String, Region>,
	#[serde(default)]
	pub pools: Vec<Pool>,
	#[serde(default)]
	pub terraform: Terraform,
	pub dns: Option<Dns>,
	#[serde(default)]
	pub s3: S3,
	pub fly: Option<Fly>,
	pub email: Option<Email>,
	#[serde(default)]
	pub captcha: Captcha,
	#[serde(default)]
	pub services: HashMap<String, Service>,
	#[serde(default)]
	pub docker: Docker,
	#[serde(default)]
	pub grafana: Option<Grafana>,
	#[serde(default)]
	pub nomad: Nomad,
	#[serde(default)]
	pub kubernetes: Kubernetes,
	#[serde(default)]
	pub redis: Redis,
	#[serde(default)]
	pub cockroachdb: CockroachDB,
	#[serde(default)]
	pub clickhouse: ClickHouse,
	#[serde(default)]
	pub traefik: Traefik,
	#[serde(default)]
	pub rust: Rust,
	#[serde(default)]
	pub rivet: Rivet,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
	/// Unique identifier for this cluster.
	///
	/// Should not be changed.
	pub id: Uuid,
	#[serde(flatten)]
	pub kind: ClusterKind,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum ClusterKind {
	#[serde(rename = "single_node")]
	SingleNode {
		public_ip: String,
		/// Port to expose API HTTP interface. Exposed on public IP.
		#[serde(default = "default_api_http_port")]
		api_http_port: u16,
		/// Port to expose API HTTPS interface. Exposed on public IP.
		#[serde(default = "default_api_https_port")]
		api_https_port: Option<u16>,
		/// Port to expose Minio on. Exposed to localhost. Not used if DNS is enabled.
		#[serde(default = "default_minio_port")]
		minio_port: u16,
		/// Port to expose the tunnel on. Exposed to localhost.
		#[serde(default = "default_tunnel_port")]
		tunnel_port: u16,

		/// Enable restricting the resources for Kubernetes services.
		///
		/// Disabled by default since this doesn't play well with development machines.
		#[serde(default)]
		limit_resources: bool,
	},
	#[serde(rename = "distributed")]
	Distributed {},
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Pool {
	pub pool: String,
	pub version: String,
	pub region: String,
	pub count: usize,
	pub size: String,
	#[serde(default)]
	pub volumes: HashMap<String, Volume>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Volume {
	pub size: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum ProviderKind {
	#[serde(rename = "linode")]
	Linode {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Terraform {
	#[serde(default)]
	pub backend: TerraformBackend,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Dns {
	/// If we should enable endpoints like `matchmaker.api.rivet.gg/v1`, which were replaced with
	/// `api.rivet.gg/matchmaker`.
	#[serde(default)]
	pub deprecated_subdomains: bool,
	pub domain: DnsDomains,
	/// Auto-provision DNS records.
	#[serde(flatten)]
	pub provider: Option<DnsProvider>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DnsDomains {
	/// Will create DNS records for:
	/// - api.{domain.main}
	pub main: String,
	/// Will create DNS records for:
	/// - *.lobby.{region}.{domain.job}
	///
	/// Can be the identical to `domain.main`.
	pub job: String,
	/// Will create DNS records for:
	/// - **.{domain.cdn}
	///
	/// Cannot be the same as `domain.main` or `domain.job`.
	pub cdn: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum DnsProvider {
	#[serde(rename = "cloudflare")]
	Cloudflare {
		account_id: String,
		// zones: CloudflareZones,
		access: Option<CloudflareAccess>,
	},
}

// #[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(deny_unknown_fields)]
// pub struct CloudflareZones {
// 	pub root: String,
// 	pub game: String,
// 	pub job: String,
// }

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct S3 {
	#[serde(default)]
	pub cors: S3Cors,
	#[serde(flatten, default)]
	pub providers: S3Providers,
	/// Used to migrate data from an old S3 provider to the new one.
	pub backfill: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct S3Cors {
	pub allowed_origins: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct S3Providers {
	pub minio: Option<S3Provider>,
	pub backblaze: Option<S3Provider>,
	pub aws: Option<S3Provider>,
}

impl Default for S3Providers {
	fn default() -> Self {
		Self {
			minio: Some(S3Provider { default: true }),
			backblaze: None,
			aws: None,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct S3Provider {
	#[serde(default)]
	pub default: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Fly {
	pub organization_id: String,
	pub region: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Email {
	#[serde(flatten, default)]
	pub provider: EmailProvider,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum EmailProvider {
	#[serde(rename = "sendgrid")]
	SendGrid {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Captcha {
	#[serde(default)]
	pub hcaptcha: Option<Hcaptcha>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Hcaptcha {
	pub site_keys: HcaptchaSiteKeys,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct HcaptchaSiteKeys {
	pub easy: String,
	pub moderate: String,
	pub difficult: String,
	pub always_on: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Service {
	pub count: usize,
	pub resources: ServiceResources,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ServiceResources {
	#[serde(flatten)]
	pub cpu: CpuResources,
	pub memory: usize,
	pub ephemeral_disk: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum CpuResources {
	#[serde(rename = "cpu_cores")]
	CpuCores(usize),
	/// MHz
	#[serde(rename = "cpu")]
	Cpu(usize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Docker {
	/// Provides authentication for Docker when pulling public images.
	///
	/// This is useful to prevent hitting rate limits when pulling Docker images.
	///
	/// See [here](https://docs.docker.com/docker-hub/download-rate-limit) for
	/// more information on Docker Hub's rate limits.
	pub authenticate_all_docker_hub_pulls: bool,
	/// Docker repository to upload builds to. Must end in a slash.
	#[serde(default = "default_docker_repo")]
	pub repository: String,
}

impl Default for Docker {
	fn default() -> Self {
		Docker {
			authenticate_all_docker_hub_pulls: false,
			repository: default_docker_repo(),
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Grafana {}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Kubernetes {
	#[serde(flatten, default)]
	pub provider: KubernetesProvider,
	#[serde(default)]
	pub health_checks: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum KubernetesProvider {
	#[serde(rename = "k3d")]
	K3d {},
	#[serde(rename = "aws_eks")]
	AwsEks {},
}

impl Default for KubernetesProvider {
	fn default() -> Self {
		Self::K3d {}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Redis {
	#[serde(default)]
	pub replicas: usize,
	#[serde(flatten, default)]
	pub provider: RedisProvider,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RedisProvider {
	#[serde(rename = "kubernetes")]
	Kubernetes {},
	#[serde(rename = "aws")]
	Aws {},
}

impl Default for RedisProvider {
	fn default() -> Self {
		Self::Kubernetes {}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct CockroachDB {
	#[serde(flatten, default)]
	pub provider: CockroachDBProvider,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CockroachDBProvider {
	#[serde(rename = "kubernetes")]
	Kubernetes {},
	#[serde(rename = "managed")]
	Managed {},
}

impl Default for CockroachDBProvider {
	fn default() -> Self {
		Self::Kubernetes {}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct ClickHouse {
	#[serde(flatten, default)]
	pub provider: ClickHouseProvider,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClickHouseProvider {
	#[serde(rename = "kubernetes")]
	Kubernetes {},
	#[serde(rename = "managed")]
	Managed {},
}

impl Default for ClickHouseProvider {
	fn default() -> Self {
		Self::Kubernetes {}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Rust {
	#[serde(default)]
	pub build_opt: RustBuildOpt,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RustBuildOpt {
	Release,
	#[default]
	Debug,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
	#[serde(default)]
	pub dynamic_servers: DynamicServers,
	#[serde(default)]
	pub cdn: Cdn,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Telemetry {
	/// Disables sending telemetry to Rivet.
	#[serde(default)]
	pub disable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct RivetTest {}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Api {
	#[serde(default)]
	pub error_verbose: bool,
	/// The origin used to build URLs for the hub in the API server.
	#[serde(default)]
	pub hub_origin: Option<String>,
	/// Regexp used to validate requests from the hub.
	pub hub_origin_regex: Option<String>,
}

impl Default for Api {
	fn default() -> Self {
		Self {
			error_verbose: false,
			hub_origin: None,
			hub_origin_regex: None,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Profanity {
	pub filter_disable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Upload {
	pub nsfw_error_verbose: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct DynamicServers {
	#[serde(default)]
	pub build_delivery_method: DynamicServersBuildDeliveryMethod,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, strum_macros::Display)]
pub enum DynamicServersBuildDeliveryMethod {
	#[serde(rename = "traffic_server")]
	#[strum(serialize = "traffic_server")]
	#[default]
	TrafficServer,
	#[serde(rename = "s3_direct")]
	#[strum(serialize = "s3_direct")]
	S3Direct,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Cdn {
	pub cache_size_gb: usize,
}

impl Default for Cdn {
	fn default() -> Self {
		Cdn { cache_size_gb: 10 }
	}
}

fn default_regions() -> HashMap<String, Region> {
	toml::from_str(include_str!("../default_regions.toml"))
		.expect("failed to parse default_regions.toml")
}

fn default_docker_repo() -> String {
	"ghcr.io/rivet-gg/".to_string()
}

fn default_api_http_port() -> u16 {
	80
}

fn default_api_https_port() -> Option<u16> {
	Some(443)
}

fn default_minio_port() -> u16 {
	9000
}

fn default_tunnel_port() -> u16 {
	5000
}
