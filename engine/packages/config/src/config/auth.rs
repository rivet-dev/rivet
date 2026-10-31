use std::time::Duration;

use anyhow::{Context, Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::secret::Secret;

const DEFAULT_AUDIENCE: &str = "rivet-api";
const DEFAULT_DURATION_SECS: u64 = 60 * 60;
const MAX_DURATION_SECS: u64 = 24 * 60 * 60;
const KEY_ROTATION_INTERVAL_SECS: u64 = 7 * 24 * 60 * 60;
const KEY_PUBLISH_LEAD_SECS: u64 = 10 * 60;
const KEY_MAX_SIGNING_LIFETIME_SECS: u64 = 14 * 24 * 60 * 60;
const VERIFIER_CACHE_TTL_SECS: u64 = 5 * 60;
const VERIFIER_MAX_STALE_SECS: u64 = 5 * 60;
const UNKNOWN_KID_REFRESH_COOLDOWN_SECS: u64 = 5;
const PUBLIC_KEY_PROPAGATION_MARGIN_SECS: u64 = 30;
const PROTOCOL_MAX_TTL_SECS: u64 = 24 * 60 * 60;
const MIN_SIGNING_FAILOVER_WINDOW_SECS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Auth {
	pub admin_token: Secret<String>,

	/// Restores legacy unauthenticated access behavior for actor and routing endpoints.
	#[serde(default)]
	pub insecure_allow_unauthenticated: bool,

	/// Short-lived, namespace-scoped JWT authentication. Enabled by default.
	#[serde(default)]
	pub jwt: Jwt,
}

impl Auth {
	pub fn validate(&self, desired_issuer: &str) -> Result<()> {
		if self.admin_token.read().is_empty() {
			anyhow::bail!("auth.admin_token cannot be empty");
		}

		self.jwt.validate(desired_issuer)
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Jwt {
	/// Controls verification, issuance, and key maintenance. Defaults to true.
	pub enabled: Option<bool>,
	/// Pauses only new token minting. Defaults to true while JWT is enabled.
	pub issuance_enabled: Option<bool>,
	/// Monotonic topology fence. Increment when changing the derived issuer or signing leader.
	pub issuer_generation: Option<u64>,
	pub audience: Option<String>,
	/// Default access-token lifetime, in seconds.
	pub default_duration: Option<u64>,
	/// Maximum access-token lifetime, in seconds. V1 is capped at one day.
	pub max_duration: Option<u64>,
	/// Normal active-key age at rotation, in seconds (seven days by default).
	pub key_rotation_interval: Option<u64>,
	/// Time to publish a pending public key before activation, in seconds (ten minutes by
	/// default). It provides verifier propagation time and does not extend the active key's
	/// lifetime.
	pub key_publish_lead: Option<u64>,
	/// Fail-closed maximum signing lifetime, in seconds (fourteen days by default). Normal
	/// rotation is still at day seven; this deadline only prevents A from signing forever if
	/// rotation stalls.
	pub key_max_signing_lifetime: Option<u64>,
	pub verifier_cache_ttl: Option<u64>,
	pub verifier_max_stale: Option<u64>,
	pub unknown_kid_refresh_cooldown: Option<u64>,
}

impl Jwt {
	pub fn enabled(&self) -> bool {
		self.enabled.unwrap_or(true)
	}

	pub fn issuance_enabled(&self) -> bool {
		self.enabled() && self.issuance_enabled.unwrap_or(true)
	}

	pub fn issuer_generation(&self) -> u64 {
		self.issuer_generation.unwrap_or(1)
	}

	pub fn audience(&self) -> &str {
		self.audience.as_deref().unwrap_or(DEFAULT_AUDIENCE)
	}

	pub fn default_duration(&self) -> Duration {
		Duration::from_secs(self.default_duration.unwrap_or(DEFAULT_DURATION_SECS))
	}

	pub fn max_duration(&self) -> Duration {
		Duration::from_secs(self.max_duration.unwrap_or(MAX_DURATION_SECS))
	}

	pub fn key_rotation_interval(&self) -> Duration {
		Duration::from_secs(
			self.key_rotation_interval
				.unwrap_or(KEY_ROTATION_INTERVAL_SECS),
		)
	}

	pub fn key_publish_lead(&self) -> Duration {
		Duration::from_secs(self.key_publish_lead.unwrap_or(KEY_PUBLISH_LEAD_SECS))
	}

	pub fn key_max_signing_lifetime(&self) -> Duration {
		Duration::from_secs(
			self.key_max_signing_lifetime
				.unwrap_or(KEY_MAX_SIGNING_LIFETIME_SECS),
		)
	}

	pub fn verifier_cache_ttl(&self) -> Duration {
		Duration::from_secs(self.verifier_cache_ttl.unwrap_or(VERIFIER_CACHE_TTL_SECS))
	}

	pub fn verifier_max_stale(&self) -> Duration {
		Duration::from_secs(self.verifier_max_stale.unwrap_or(VERIFIER_MAX_STALE_SECS))
	}

	pub fn unknown_kid_refresh_cooldown(&self) -> Duration {
		Duration::from_secs(
			self.unknown_kid_refresh_cooldown
				.unwrap_or(UNKNOWN_KID_REFRESH_COOLDOWN_SECS),
		)
	}

	pub fn validate(&self, _desired_issuer: &str) -> Result<()> {
		ensure!(
			self.issuer_generation() > 0,
			"auth.jwt.issuer_generation must be positive"
		);

		ensure!(
			!self.audience().is_empty(),
			"auth.jwt.audience cannot be empty"
		);

		let default_duration = self.default_duration().as_secs();
		let max_duration = self.max_duration().as_secs();
		let rotation = self.key_rotation_interval().as_secs();
		let publish_lead = self.key_publish_lead().as_secs();
		let max_signing_lifetime = self.key_max_signing_lifetime().as_secs();
		let cache_ttl = self.verifier_cache_ttl().as_secs();
		let max_stale = self.verifier_max_stale().as_secs();
		let unknown_kid_cooldown = self.unknown_kid_refresh_cooldown().as_secs();

		for (name, value) in [
			("default_duration", default_duration),
			("max_duration", max_duration),
			("key_rotation_interval", rotation),
			("key_publish_lead", publish_lead),
			("key_max_signing_lifetime", max_signing_lifetime),
			("verifier_cache_ttl", cache_ttl),
			("verifier_max_stale", max_stale),
			("unknown_kid_refresh_cooldown", unknown_kid_cooldown),
		] {
			ensure!(value > 0, "auth.jwt.{name} must be positive");
		}

		ensure!(
			default_duration <= max_duration,
			"auth.jwt.default_duration cannot exceed auth.jwt.max_duration"
		);
		ensure!(
			max_duration <= PROTOCOL_MAX_TTL_SECS,
			"auth.jwt.max_duration cannot exceed {PROTOCOL_MAX_TTL_SECS} seconds"
		);
		ensure!(
			publish_lead < rotation,
			"auth.jwt.key_publish_lead must be shorter than auth.jwt.key_rotation_interval"
		);
		ensure!(
			publish_lead > cache_ttl + PUBLIC_KEY_PROPAGATION_MARGIN_SECS,
			"auth.jwt.key_publish_lead must exceed auth.jwt.verifier_cache_ttl plus {PUBLIC_KEY_PROPAGATION_MARGIN_SECS} seconds"
		);
		ensure!(
			max_signing_lifetime >= rotation + MIN_SIGNING_FAILOVER_WINDOW_SECS,
			"auth.jwt.key_max_signing_lifetime must leave at least seven days after normal rotation"
		);
		ensure!(
			max_stale >= cache_ttl,
			"auth.jwt.verifier_max_stale cannot be shorter than auth.jwt.verifier_cache_ttl"
		);

		ensure!(
			max_stale <= VERIFIER_MAX_STALE_SECS,
			"auth.jwt.verifier_max_stale cannot exceed five minutes"
		);
		Ok(())
	}
}

/// Produces the canonical issuer string persisted with the JWT key ring.
pub fn normalize_issuer(url: &Url) -> Result<String> {
	ensure!(
		matches!(url.scheme(), "http" | "https"),
		"JWT issuers must use http or https"
	);
	ensure!(
		url.username().is_empty() && url.password().is_none(),
		"JWT issuers cannot contain credentials"
	);
	ensure!(url.query().is_none(), "JWT issuers cannot contain a query");
	ensure!(
		url.fragment().is_none(),
		"JWT issuers cannot contain a fragment"
	);
	ensure!(
		url.host_str().is_some(),
		"JWT issuers must be absolute URLs"
	);

	let mut normalized = url.clone();
	let trimmed_path = normalized.path().trim_end_matches('/').to_owned();
	normalized.set_path(if trimmed_path.is_empty() {
		"/"
	} else {
		&trimmed_path
	});
	let serialized = normalized.to_string();
	Ok(serialized
		.strip_suffix('/')
		.unwrap_or(&serialized)
		.to_owned())
}

pub fn derive_issuer(root: &super::Root) -> Result<String> {
	normalize_issuer(
		&root
			.leader_dc()
			.context("cannot derive JWT issuer without a topology leader")?
			.public_url,
	)
}


