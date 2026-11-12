use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Shortest confirm delay accepted. It must outlast V2's guard lookup, auth check and wake send,
/// which take up to 5 s each, so a wake that read the actor before the cutover has landed.
pub const MIN_CONFIRM_DELAY_MS: u64 = 15_000;
/// Shortest wake horizon accepted. V2 workers pull deadlines up to their poll interval (16 s by
/// default) ahead, and V2's confirm refuses deadlines inside that window plus 5 s.
pub const MIN_WAKE_HORIZON_MS: u64 = 21_000;
const MAX_IN_FLIGHT: u32 = 10_000;
const MAX_ACTORS_PER_SEC: u32 = 100_000;
const MAX_INTERVAL_MS: u64 = 7 * 24 * 60 * 60 * 1000;
const MAX_NAMESPACE_NAME_BYTES: usize = 63;

/// Settings for moving one V2 namespace's actors into this cluster.
///
/// The section is read through `config.dynamic()`, and a dynamic config update replaces it as a
/// whole. `migrate_seed` is the exception: it is fixed at startup, because a new seed reshuffles
/// which actors fall under `migrate_ratio`, so raising the ratio would no longer only add actors.
#[derive(Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrateFromV2 {
	/// V2's public gateway, where requests for actors this cluster does not hold are proxied.
	pub endpoint: String,
	/// V2's public API per datacenter label, which the migrator lists and moves actors through.
	#[serde(default)]
	pub api_urls: HashMap<String, String>,
	/// The namespace-wide V2 token for V2's migration API and the public API calls routing makes.
	/// Never logged.
	#[serde(default)]
	pub token: Option<String>,
	/// The V2 namespace being migrated.
	pub namespace: String,
	/// The namespace here that the actors land in. Defaults to the V2 namespace's name.
	#[serde(default)]
	pub target_namespace: Option<String>,
	/// Share of new creates that land in this cluster. Keyless creates draw against it; keyed
	/// creates move here only at 1.0.
	#[serde(default)]
	pub create_ratio: f64,
	/// Share of existing V2 actors the migrator moves, picked by a stable hash of the actor id.
	#[serde(default)]
	pub migrate_ratio: f64,
	/// Seed of the hash behind `migrate_ratio`.
	#[serde(default)]
	pub migrate_seed: u64,
	pub max_in_flight: Option<u32>,
	pub max_actors_per_sec: Option<u32>,
	pub pass_interval_ms: Option<u64>,
	pub min_sleep_ms: Option<u64>,
	pub wake_horizon_ms: Option<u64>,
	pub confirm_delay_ms: Option<u64>,
	/// Largest actor the migrator copies, by V2's reported SQLite size plus its KV estimate.
	/// Larger actors are skipped as `too_large` on every pass, so they never hold a slot; raise
	/// it once the rest of the namespace has moved to migrate them. Defaults to 10 GiB.
	pub max_copy_bytes: Option<u64>,
	/// Bytes per second the migrator may admit for copying, by the same size `max_copy_bytes`
	/// reads. Each admitted actor is charged its whole size when it is admitted; a new candidate
	/// waits while the bytes already admitted are more than one second of this rate ahead. It bounds
	/// the copy load on the storage nodes, which the actor count alone does not when actors are
	/// large. The migrator is a singleton, so the budget is per datacenter. Defaults to 32 MiB/s.
	pub max_copy_bytes_per_sec: Option<u64>,
}

impl std::fmt::Debug for MigrateFromV2 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MigrateFromV2")
			.field("endpoint", &self.endpoint)
			.field("api_urls", &self.api_urls)
			.field("token", &self.token.as_ref().map(|_| "<redacted>"))
			.field("namespace", &self.namespace)
			.field("target_namespace", &self.target_namespace)
			.field("create_ratio", &self.create_ratio)
			.field("migrate_ratio", &self.migrate_ratio)
			.field("migrate_seed", &self.migrate_seed)
			.field("max_in_flight", &self.max_in_flight)
			.field("max_actors_per_sec", &self.max_actors_per_sec)
			.field("pass_interval_ms", &self.pass_interval_ms)
			.field("min_sleep_ms", &self.min_sleep_ms)
			.field("wake_horizon_ms", &self.wake_horizon_ms)
			.field("confirm_delay_ms", &self.confirm_delay_ms)
			.field("max_copy_bytes", &self.max_copy_bytes)
			.field("max_copy_bytes_per_sec", &self.max_copy_bytes_per_sec)
			.finish()
	}
}

impl MigrateFromV2 {
	pub fn target_namespace(&self) -> &str {
		self.target_namespace.as_deref().unwrap_or(&self.namespace)
	}

	pub fn max_in_flight(&self) -> u32 {
		self.max_in_flight.unwrap_or(16)
	}

	pub fn max_actors_per_sec(&self) -> u32 {
		self.max_actors_per_sec.unwrap_or(50)
	}

	pub fn pass_interval_ms(&self) -> u64 {
		self.pass_interval_ms.unwrap_or(5 * 60 * 1000)
	}

	pub fn min_sleep_ms(&self) -> u64 {
		self.min_sleep_ms.unwrap_or(10 * 60 * 1000)
	}

	/// Must exceed V2 gasoline's pull-ahead window (`worker_poll_interval`, 16 s by default),
	/// inside which a worker holds a sleeping workflow in memory.
	pub fn wake_horizon_ms(&self) -> u64 {
		self.wake_horizon_ms.unwrap_or(5 * 60 * 1000)
	}

	pub fn confirm_delay_ms(&self) -> u64 {
		self.confirm_delay_ms.unwrap_or(20_000)
	}

	pub fn max_copy_bytes(&self) -> u64 {
		self.max_copy_bytes.unwrap_or(10 * 1024 * 1024 * 1024)
	}

	pub fn max_copy_bytes_per_sec(&self) -> u64 {
		self.max_copy_bytes_per_sec.unwrap_or(32 * 1024 * 1024)
	}

	pub fn validate(&self) -> Result<()> {
		url::Url::parse(&self.endpoint).context("migrate_from_v2.endpoint is not a url")?;
		for (label, api_url) in &self.api_urls {
			label.parse::<u16>().with_context(|| {
				format!("migrate_from_v2.api_urls key {label} is not a datacenter label")
			})?;
			url::Url::parse(api_url)
				.with_context(|| format!("migrate_from_v2.api_urls.{label} is not a url"))?;
		}
		ensure!(
			!self.namespace.is_empty(),
			"migrate_from_v2.namespace is empty"
		);
		ensure!(
			valid_namespace_name(self.target_namespace()),
			"migrate_from_v2.target_namespace {} is not a valid namespace name: use 1 to {MAX_NAMESPACE_NAME_BYTES} lowercase letters, digits and inner hyphens, or set it explicitly",
			self.target_namespace()
		);
		for (name, ratio) in [
			("create_ratio", self.create_ratio),
			("migrate_ratio", self.migrate_ratio),
		] {
			if !(0.0..=1.0).contains(&ratio) {
				bail!("migrate_from_v2.{name} must be between 0.0 and 1.0, got {ratio}");
			}
		}
		ensure!(
			(1..=MAX_IN_FLIGHT).contains(&self.max_in_flight()),
			"migrate_from_v2.max_in_flight must be between 1 and {MAX_IN_FLIGHT}"
		);
		ensure!(
			(1..=MAX_ACTORS_PER_SEC).contains(&self.max_actors_per_sec()),
			"migrate_from_v2.max_actors_per_sec must be between 1 and {MAX_ACTORS_PER_SEC}"
		);
		for (name, value) in [
			("pass_interval_ms", self.pass_interval_ms()),
			("min_sleep_ms", self.min_sleep_ms()),
			("wake_horizon_ms", self.wake_horizon_ms()),
			("confirm_delay_ms", self.confirm_delay_ms()),
		] {
			ensure!(
				(1..=MAX_INTERVAL_MS).contains(&value),
				"migrate_from_v2.{name} must be between 1 and {MAX_INTERVAL_MS}"
			);
		}
		ensure!(
			self.max_copy_bytes() > 0,
			"migrate_from_v2.max_copy_bytes must be positive"
		);
		ensure!(
			self.max_copy_bytes_per_sec() > 0,
			"migrate_from_v2.max_copy_bytes_per_sec must be positive"
		);
		ensure!(
			self.wake_horizon_ms() >= MIN_WAKE_HORIZON_MS,
			"migrate_from_v2.wake_horizon_ms must be at least {MIN_WAKE_HORIZON_MS}, past V2's deadline pull-ahead"
		);
		ensure!(
			self.confirm_delay_ms() >= MIN_CONFIRM_DELAY_MS,
			"migrate_from_v2.confirm_delay_ms must be at least {MIN_CONFIRM_DELAY_MS}"
		);
		Ok(())
	}
}

/// The namespace name rule pegboard applies to every request, checked here so a misconfigured
/// target is refused when the config loads instead of failing every routed request.
fn valid_namespace_name(namespace: &str) -> bool {
	!namespace.is_empty()
		&& namespace.len() <= MAX_NAMESPACE_NAME_BYTES
		&& namespace
			.bytes()
			.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
		&& !namespace.starts_with('-')
		&& !namespace.ends_with('-')
}
