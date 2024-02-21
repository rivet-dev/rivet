use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum EnvVarError {
	#[error("missing env var: {0}")]
	Missing(String),

	#[error("{0}")]
	Invalid(std::env::VarError),
}

lazy_static::lazy_static! {
	static ref RUN_CONTEXT: Option<RunContext> = std::env::var("RIVET_RUN_CONTEXT")
		.ok()
		.and_then(|ctx| RunContext::from_str(&ctx));
	static ref PRIMARY_REGION: Option<String> = std::env::var("RIVET_PRIMARY_REGION").ok();
	static ref NAMESPACE: Option<String> = std::env::var("RIVET_NAMESPACE").ok();
	static ref CLUSTER_ID: Option<String> = std::env::var("RIVET_CLUSTER_ID").ok();
	static ref SOURCE_HASH: Option<String> = std::env::var("RIVET_SOURCE_HASH").ok();
	static ref DOMAIN_MAIN: Option<String> = std::env::var("RIVET_DOMAIN_MAIN").ok();
	static ref DOMAIN_CDN: Option<String> = std::env::var("RIVET_DOMAIN_CDN").ok();
	static ref DOMAIN_JOB: Option<String> = std::env::var("RIVET_DOMAIN_JOB").ok();
	static ref DOMAIN_MAIN_API: Option<String> = std::env::var("RIVET_DOMAIN_MAIN_API").ok();
	static ref SUPPORT_DEPRECATED_SUBDOMAINS: bool = std::env::var("RIVET_SUPPORT_DEPRECATED_SUBDOMAINS")
		.ok()
		.map(|s| s == "1")
		.unwrap_or_default();
	static ref HOST_API: Option<String> = std::env::var("RIVET_HOST_API").ok();
	static ref ORIGIN_API: Option<String> = std::env::var("RIVET_ORIGIN_API").ok();
	static ref ORIGIN_HUB: Option<String> = std::env::var("RIVET_ORIGIN_HUB").ok();
	static ref DNS_PROVIDER: Option<String> = std::env::var("RIVET_DNS_PROVIDER").ok();
	static ref CHIRP_SERVICE_NAME: Option<String> = std::env::var("CHIRP_SERVICE_NAME").ok();
	static ref BILLING: Option<RivetBilling> = std::env::var("RIVET_BILLING")
		.ok()
		.map(|x| serde_json::from_str(&x).expect("failed to parse billing"));
	static ref CLUSTER_TYPE: Option<String> = std::env::var("RIVET_CLUSTER_ID").ok();
}

/// Where this code is being written from. This is derived from the `RIVET_RUN_CONTEXT` environment
/// variable.
///
/// The production run context is not the same as the production namespace.
#[derive(Clone, Debug, PartialEq)]
pub enum RunContext {
	Service,
	Test,
}

impl RunContext {
	fn from_str(ctx: &str) -> Option<RunContext> {
		match ctx {
			"service" => Some(RunContext::Service),
			"test" => Some(RunContext::Test),
			_ => None,
		}
	}
}

pub fn run_context() -> RunContext {
	RUN_CONTEXT.clone().expect("RIVET_RUN_CONTEXT")
}

/// The namespace this service is running in. This is derived from the `NAMESPACE` environment
/// variable.
pub fn namespace() -> &'static str {
	match &*NAMESPACE {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("RIVET_NAMESPACE".to_string())),
	}
}

pub fn cluster_id() -> &'static str {
	match &*CLUSTER_ID {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("RIVET_CLUSTER_ID".to_string())),
	}
}

/// See `ServiceContextData::source_hash`.
pub fn source_hash() -> &'static str {
	match &*NAMESPACE {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("RIVET_SOURCE_HASH".to_string())),
	}
}

/// The base domain in which all subdomains are mounted.
pub fn domain_main() -> Option<&'static str> {
	DOMAIN_MAIN.as_ref().map(|x| x.as_str())
}

/// The base domain in which all game subdomains are mounted.
pub fn domain_cdn() -> Option<&'static str> {
	DOMAIN_CDN.as_ref().map(|x| x.as_str())
}

/// The base domain in which all job subdomains are mounted.
pub fn domain_job() -> Option<&'static str> {
	DOMAIN_JOB.as_ref().map(|x| x.as_str())
}

/// Domain to host the API endpoint on. This is the default domain for all endpoints without a
/// specific subdomain.
pub fn domain_main_api() -> Option<&'static str> {
	DOMAIN_MAIN_API.as_ref().map(|x| x.as_str())
}

pub fn support_deprecated_subdomains() -> bool {
	*SUPPORT_DEPRECATED_SUBDOMAINS
}

/// The host for the API.
pub fn host_api() -> &'static str {
	match &*HOST_API {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("RIVET_HOST_API".to_string())),
	}
}

/// The base domain for the API.
pub fn origin_api() -> &'static str {
	match &*ORIGIN_API {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("RIVET_ORIGIN_API".to_string())),
	}
}

/// The base domain for the hub.
pub fn origin_hub() -> &'static str {
	match &*ORIGIN_HUB {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("RIVET_ORIGIN_HUB".to_string())),
	}
}

pub fn dns_provider() -> Option<&'static str> {
	DNS_PROVIDER.as_ref().map(|x| x.as_str())
}

pub fn primary_region() -> &'static str {
	match &*PRIMARY_REGION {
		Some(x) => x.as_str(),
		None => panic!("RIVET_PRIMARY_REGION"),
	}
}

pub fn chirp_service_name() -> &'static str {
	match &*CHIRP_SERVICE_NAME {
		Some(x) => x.as_str(),
		None => panic!("{}", EnvVarError::Missing("CHIRP_SERVICE_NAME".to_string())),
	}
}

#[derive(Deserialize)]
pub struct RivetBilling {
	pub dynamic_servers_capacity_price_id: String,
}

pub fn billing() -> Option<&'static RivetBilling> {
	BILLING.as_ref()
}

/// The current stripe API token.
pub async fn stripe_secret_key() -> Result<String, EnvVarError> {
	read_secret(&["stripe", "secret_key"]).await
}

/// The current stripe webhook secret.
///
/// Secrets can be added at: https://dashboard.stripe.com/webhooks
///
/// Add the following events to the WebHook:
/// - invoice.payment_succeeded
/// - checkout.session.completed
/// - payment_intent.succeeded
pub async fn stripe_webhook_secret() -> Result<String, EnvVarError> {
	read_secret(&["stripe", "webhook_secret"]).await
}

pub mod cloudflare {
	use super::EnvVarError;

	lazy_static::lazy_static! {
		static ref CLOUDFLARE_AUTH_TOKEN: Option<String> = std::env::var("CLOUDFLARE_AUTH_TOKEN").ok();
	}

	pub fn auth_token() -> &'static str {
		match &*CLOUDFLARE_AUTH_TOKEN {
			Some(x) => x.as_str(),
			None => panic!(
				"{}",
				EnvVarError::Missing("CLOUDFLARE_AUTH_TOKEN".to_string())
			),
		}
	}

	pub mod zone {
		pub mod base {
			lazy_static::lazy_static! {
				static ref ID: Option<String> = std::env::var("CLOUDFLARE_ZONE_ID_BASE").ok();
			}

			pub fn id() -> Option<&'static str> {
				ID.as_ref().map(|x| x.as_str())
			}
		}

		pub mod game {
			lazy_static::lazy_static! {
				static ref ID: Option<String> = std::env::var("CLOUDFLARE_ZONE_ID_GAME").ok();
			}

			pub fn id() -> Option<&'static str> {
				ID.as_ref().map(|x| x.as_str())
			}
		}

		pub mod job {
			lazy_static::lazy_static! {
				static ref ID: Option<String> = std::env::var("CLOUDFLARE_ZONE_ID_JOB").ok();
			}

			pub fn id() -> Option<&'static str> {
				ID.as_ref().map(|x| x.as_str())
			}
		}
	}
}

/// Reads a secret from the env.
///
/// This is marked as async so we have the flexibility to pull the secret from remote datasources.
pub async fn read_secret(key: &[impl AsRef<str>]) -> Result<String, EnvVarError> {
	var(secret_env_var_key(key))
}

pub async fn read_secret_opt(key: &[impl AsRef<str>]) -> Result<Option<String>, EnvVarError> {
	let env_var = read_secret(key).await;

	match env_var {
		Ok(v) => Ok(Some(v)),
		Err(var_error) => match var_error {
			EnvVarError::Missing(_) => Ok(None),
			EnvVarError::Invalid(_) => Err(var_error),
		},
	}
}

/// Name of env var holding a given secret.
pub fn secret_env_var_key(key: &[impl AsRef<str>]) -> String {
	key.iter()
		.map(|x| x.as_ref().to_uppercase())
		.collect::<Vec<_>>()
		.join("_")
}

pub fn var(name: impl AsRef<str>) -> Result<String, EnvVarError> {
	let env_var = std::env::var(name.as_ref());

	match env_var {
		Ok(v) => Ok(v),
		Err(var_error) => match var_error {
			std::env::VarError::NotPresent => Err(EnvVarError::Missing(name.as_ref().to_string())),
			_ => Err(EnvVarError::Invalid(var_error)),
		},
	}
}
