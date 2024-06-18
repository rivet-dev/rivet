use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
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
	pub nomad: Nomad,
	#[serde(default)]
	pub kubernetes: Kubernetes,
	#[serde(default)]
	pub redis: Redis,
	#[serde(default)]
	pub cockroachdb: CockroachDB,
	#[serde(default)]
	pub clickhouse: Option<ClickHouse>,
	#[serde(default)]
	pub traefik: Traefik,
	#[serde(default)]
	pub prometheus: Option<Prometheus>,
	#[serde(default)]
	pub rust: Rust,
	#[serde(default)]
	pub rivet: Rivet,
	#[serde(default)]
	pub bolt: Bolt,
	#[serde(default)]
	pub better_uptime: Option<BetterUptime>,
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

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Secrets {
	pub path: Option<PathBuf>,
	#[serde(rename = "1password")]
	pub _1password: Option<_1Password>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct _1Password {
	pub namespace_path: String,
	pub secrets_path: String,
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
	/// - *.lobby.{region_id}.{domain.job}
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
pub struct CloudflareAccessServices {
	pub grafana: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CloudflareAccessGroups {
	pub engineering: String,
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
	#[serde(default)]
	pub turnstile: Option<Turnstile>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Hcaptcha {
	pub site_key_fallback: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Turnstile {
	pub site_key_main: String,
	pub site_key_cdn: String,
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
	pub cpu: usize,
	pub memory: usize,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Nomad {
	pub health_checks: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Kubernetes {
	#[serde(flatten, default)]
	pub provider: KubernetesProvider,
	#[serde(default)]
	pub health_checks: Option<bool>,
	#[serde(default)]
	pub dashboard_enabled: bool,
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
	#[serde(rename = "aiven")]
	Aiven {
		project: String,
		cloud: String,
		plan_ephemeral: String,
		plan_persistent: String,
	},
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
	Managed {
		/// USD cents.
		spend_limit: u32,
		/// CRDB Request Units.
		/// https://www.cockroachlabs.com/docs/cockroachcloud/architecture#request-unit-ru
		request_unit_limit: u32,
		/// Storage limit in MiB.
		storage_limit: u32,
	},
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
	Managed { tier: ClickHouseManagedTier },
}

impl Default for ClickHouseProvider {
	fn default() -> Self {
		Self::Kubernetes {}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ClickHouseManagedTier {
	#[serde(rename = "development")]
	Development {},
	#[serde(rename = "production")]
	Production {
		min_total_memory_gb: usize,
		max_total_memory_gb: usize,
	},
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Prometheus {}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Rust {
	#[serde(default)]
	pub build_opt: RustBuildOpt,

	/// Enables using sccache to speed up Docker builds since we cannot use a shared Docker file.
	///
	/// Does not support `bolt check`. This will cause `bolt up` to error on single node
	/// installations if sccache is not installed.
	#[serde(default)]
	pub sccache: Option<RustSccache>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct RustSccache {
	pub bucket: String,
	pub endpoint: String,
	pub region: String,
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
	pub access: RivetAccess,
	#[serde(default)]
	pub login: RivetLogin,
	#[serde(default)]
	pub test: Option<RivetTest>,
	#[serde(default)]
	pub api: Api,
	#[serde(default)]
	pub profanity: Profanity,
	#[serde(default)]
	pub upload: Upload,
	#[serde(default)]
	pub provisioning: Option<Provisioning>,
	#[serde(default)]
	pub cdn: Cdn,
	#[serde(default)]
	pub billing: Option<RivetBilling>,
	#[serde(default)]
	pub opengb: Option<RivetOpenGb>,
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
pub struct RivetOpenGb {}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub enum RivetAccess {
	#[serde(rename = "private")]
	Private {},
	#[serde(rename = "public")]
	Public {},
}

impl Default for RivetAccess {
	fn default() -> Self {
		RivetAccess::Private {}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RivetLogin {
	pub enable_admin: bool,
}

impl Default for RivetLogin {
	fn default() -> Self {
		RivetLogin {
			// Admin login enabled by default
			enable_admin: true,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct RivetTest {
	#[serde(default)]
	pub load_tests: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Profanity {
	pub filter_disable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Upload {
	pub nsfw_check: Option<NsfwCheck>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct NsfwCheck {
	#[serde(default)]
	pub error_verbose: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Provisioning {
	/// Default cluster.
	pub cluster: Option<ProvisioningCluster>,
	/// Whether or not to send a taint message in the next cluster update.
	#[serde(default)]
	pub taint: bool,
	/// How many empty job servers to have at all times. Used in the simple provisioning algorithm on Rivet
	/// Enterprise.
	#[serde(default = "default_job_server_provision_margin")]
	pub job_server_provision_margin: u32,
	#[serde(default)]
	pub acme_directory: ProvisioningAcmeDirectory,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ProvisioningAcmeDirectory {
	#[serde(rename = "lets_encrypt")]
	#[default]
	LetsEncrypt,
	#[serde(rename = "lets_encrypt_staging")]
	LetsEncryptStaging,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ProvisioningCluster {
	/// Identifier name of the default cluster.
	name_id: String,
	#[serde(default)]
	pub datacenters: HashMap<String, ProvisioningDatacenter>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ProvisioningDatacenter {
	/// Randomly generated ID for the given datacenter.
	pub datacenter_id: Uuid,
	pub display_name: String,
	pub provider: ProvisioningProvider,
	pub provider_datacenter_name: String,
	#[serde(default)]
	pub build_delivery_method: ProvisioningBuildDeliveryMethod,
	#[serde(default)]
	pub pools: HashMap<ProvisioningDatacenterPoolType, ProvisioningDatacenterPool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProvisioningProvider {
	#[serde(rename = "linode")]
	Linode,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, strum_macros::Display)]
pub enum ProvisioningBuildDeliveryMethod {
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
pub struct ProvisioningDatacenterPool {
	pub hardware: Vec<ProvisioningDatacenterHardware>,
	pub desired_count: u32,
	#[serde(default)]
	pub min_count: u32,
	pub max_count: u32,
	/// Server drain time in ms.
	pub drain_timeout: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ProvisioningDatacenterHardware {
	pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProvisioningDatacenterPoolType {
	#[serde(rename = "job")]
	Job,
	#[serde(rename = "gg")]
	Gg,
	#[serde(rename = "ats")]
	Ats,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Cdn {
	pub cache_size_gb: usize,
	pub image_resizing: Option<ImageResizing>,
}

impl Default for Cdn {
	fn default() -> Self {
		Cdn {
			cache_size_gb: 10,
			image_resizing: None,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImageResizing {}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RivetBilling {
	pub indie_price_id: String,
	pub studio_price_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Bolt {
	#[serde(default)]
	pub confirm_commands: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct BetterUptime {
	/// The name of your company. This will be displayed on your status page
	/// in the top left. This is required by Better Uptime.
	pub company_name: String,
	/// The URL of your company. This will be used on the status page to link
	/// to your company's website. This is required by Better Uptime.
	pub company_url: String,
	/// The subdomain is the part of the public URL of your status page uses.
	///
	/// Eg. <company_subdomain>.betteruptime.com.
	///
	/// It needs to be unique across all of Better Uptime. This is required
	/// by Better Uptime.
	pub company_subdomain: String,
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

fn default_job_server_provision_margin() -> u32 {
	2
}

// Used for parsing from 1password
#[derive(Deserialize, Debug)]
pub struct PartialNamespace {
	#[serde(default)]
	pub secrets: Secrets,
}
