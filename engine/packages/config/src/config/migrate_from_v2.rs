use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ObjectStoreBackend;

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
/// Largest page V2's candidates and parked listings accept.
const MAX_PAGE_LIMIT: u32 = 500;

/// Settings for moving one V2 namespace's actors into this cluster.
///
/// The section is read through `config.dynamic()`. A dynamic config update overrides it field by
/// field with [`MigrateFromV2Overrides`], except for the two fields fixed at startup:
/// `migrate_seed`, because a new seed reshuffles which actors fall under `migrate_ratio`, so
/// raising the ratio would no longer only add actors, and `backup`, whose store is built once at
/// startup.
#[derive(Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrateFromV2 {
	/// V2's public gateway, where requests for actors this cluster does not hold are proxied.
	pub endpoint: String,
	/// V2's public API per datacenter label, which the migrator lists and moves actors through.
	#[serde(default)]
	pub api_urls: HashMap<String, String>,
	/// The namespace-wide V2 token for V2's migration API, the public API calls routing makes and
	/// gateway requests proxied to V2. Clients authenticate here with this cluster's tokens, which
	/// V2 cannot verify, so the gateway authorizes them here and reaches V2 with this token
	/// instead. Never logged.
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
	/// Moves only the V2 actors with this name when set. V2 lists one name's actors oldest first,
	/// so a name moves in creation order. Actors an operator requests by id move whatever their
	/// name. Needs a V2 whose candidates route takes the `name` filter; an older one refuses it.
	#[serde(default)]
	pub actor_name: Option<String>,
	pub max_in_flight: Option<u32>,
	pub max_actors_per_sec: Option<u32>,
	pub pass_interval_ms: Option<u64>,
	pub min_sleep_ms: Option<u64>,
	pub wake_horizon_ms: Option<u64>,
	pub confirm_delay_ms: Option<u64>,
	/// Where the migrator backs up every actor it copies from V2: its metadata, KV and SQLite
	/// pages as V2 exported them, outside live data, so the actor can be restored once its V2 data
	/// is deleted. While it is set, a copy whose backup cannot be written fails and is retried, so
	/// no actor is cut over from V2 without one. Unset by default, which writes no backup. Fixed at
	/// startup.
	#[serde(default)]
	pub backup: Option<ObjectStoreBackend>,
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
	/// Candidates the migrator asks V2 for in one listing request. V2 accepts 1 to 500; a smaller
	/// page keeps each V2 listing request shorter. Defaults to 200.
	pub candidates_page_limit: Option<u32>,
	/// Parked actors `rivet-engine migrate-from-v2 delete-parked` asks V2 for in one listing
	/// request. V2 accepts 1 to 500; a smaller page keeps each V2 listing request shorter.
	/// Defaults to 100.
	pub parked_page_limit: Option<u32>,
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
			.field("actor_name", &self.actor_name)
			.field("max_in_flight", &self.max_in_flight)
			.field("max_actors_per_sec", &self.max_actors_per_sec)
			.field("pass_interval_ms", &self.pass_interval_ms)
			.field("min_sleep_ms", &self.min_sleep_ms)
			.field("wake_horizon_ms", &self.wake_horizon_ms)
			.field("confirm_delay_ms", &self.confirm_delay_ms)
			.field("max_copy_bytes", &self.max_copy_bytes)
			.field("max_copy_bytes_per_sec", &self.max_copy_bytes_per_sec)
			.field("candidates_page_limit", &self.candidates_page_limit)
			.field("parked_page_limit", &self.parked_page_limit)
			.field("backup", &self.backup)
			.finish()
	}
}

/// A runtime override of `migrate_from_v2`, field by field, through dynamic config. Each field it
/// sets replaces that field of the section loaded at startup, and a field it leaves unset or sets
/// to `null` keeps the startup value. A field is doubly optional, as in `DynamicConfigUpdate`, so
/// a patch's `null` survives the merge into the stored overrides and clears that one field. `migrate_seed` and `backup` are fixed at startup, so it has no such fields. With
/// no section at startup, the override must set `endpoint` and `namespace`, which starts a
/// migration without a restart.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrateFromV2Overrides {
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub endpoint: Option<Option<String>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub api_urls: Option<Option<HashMap<String, String>>>,
	/// Never logged.
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub token: Option<Option<String>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub namespace: Option<Option<String>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub target_namespace: Option<Option<String>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub create_ratio: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub migrate_ratio: Option<Option<f64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub actor_name: Option<Option<String>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub max_in_flight: Option<Option<u32>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub max_actors_per_sec: Option<Option<u32>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub pass_interval_ms: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub min_sleep_ms: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub wake_horizon_ms: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub confirm_delay_ms: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub max_copy_bytes: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub max_copy_bytes_per_sec: Option<Option<u64>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub candidates_page_limit: Option<Option<u32>>,
	#[serde(
		default,
		deserialize_with = "crate::dynamic::double_option",
		skip_serializing_if = "Option::is_none"
	)]
	pub parked_page_limit: Option<Option<u32>>,
}

impl std::fmt::Debug for MigrateFromV2Overrides {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("MigrateFromV2Overrides")
			.field("endpoint", &self.endpoint)
			.field("api_urls", &self.api_urls)
			.field("token", &self.token.as_ref().map(|_| "<redacted>"))
			.field("namespace", &self.namespace)
			.field("target_namespace", &self.target_namespace)
			.field("create_ratio", &self.create_ratio)
			.field("migrate_ratio", &self.migrate_ratio)
			.field("actor_name", &self.actor_name)
			.field("max_in_flight", &self.max_in_flight)
			.field("max_actors_per_sec", &self.max_actors_per_sec)
			.field("pass_interval_ms", &self.pass_interval_ms)
			.field("min_sleep_ms", &self.min_sleep_ms)
			.field("wake_horizon_ms", &self.wake_horizon_ms)
			.field("confirm_delay_ms", &self.confirm_delay_ms)
			.field("max_copy_bytes", &self.max_copy_bytes)
			.field("max_copy_bytes_per_sec", &self.max_copy_bytes_per_sec)
			.field("candidates_page_limit", &self.candidates_page_limit)
			.field("parked_page_limit", &self.parked_page_limit)
			.finish()
	}
}

impl MigrateFromV2Overrides {
	/// The section `startup` with these overrides applied. With no startup section, the overrides
	/// must name the endpoint and namespace, and every other field starts at its default.
	pub fn apply(&self, startup: Option<&MigrateFromV2>) -> Result<MigrateFromV2> {
		let mut section = match startup {
			Some(startup) => startup.clone(),
			None => MigrateFromV2 {
				endpoint: self.endpoint.clone().flatten().context(
					"migrate_from_v2 is not configured at startup, so an override must set endpoint",
				)?,
				api_urls: HashMap::new(),
				token: None,
				namespace: self.namespace.clone().flatten().context(
					"migrate_from_v2 is not configured at startup, so an override must set namespace",
				)?,
				target_namespace: None,
				create_ratio: 0.0,
				migrate_ratio: 0.0,
				migrate_seed: 0,
				actor_name: None,
				max_in_flight: None,
				max_actors_per_sec: None,
				pass_interval_ms: None,
				min_sleep_ms: None,
				wake_horizon_ms: None,
				confirm_delay_ms: None,
				backup: None,
				max_copy_bytes: None,
				max_copy_bytes_per_sec: None,
				candidates_page_limit: None,
				parked_page_limit: None,
			},
		};

		if let Some(Some(endpoint)) = &self.endpoint {
			section.endpoint = endpoint.clone();
		}
		if let Some(Some(api_urls)) = &self.api_urls {
			section.api_urls = api_urls.clone();
		}
		if let Some(Some(token)) = &self.token {
			section.token = Some(token.clone());
		}
		if let Some(Some(namespace)) = &self.namespace {
			section.namespace = namespace.clone();
		}
		if let Some(Some(target_namespace)) = &self.target_namespace {
			section.target_namespace = Some(target_namespace.clone());
		}
		if let Some(Some(create_ratio)) = self.create_ratio {
			section.create_ratio = create_ratio;
		}
		if let Some(Some(migrate_ratio)) = self.migrate_ratio {
			section.migrate_ratio = migrate_ratio;
		}
		if let Some(Some(actor_name)) = &self.actor_name {
			section.actor_name = Some(actor_name.clone());
		}
		if let Some(Some(value)) = self.max_in_flight {
			section.max_in_flight = Some(value);
		}
		if let Some(Some(value)) = self.max_actors_per_sec {
			section.max_actors_per_sec = Some(value);
		}
		if let Some(Some(value)) = self.pass_interval_ms {
			section.pass_interval_ms = Some(value);
		}
		if let Some(Some(value)) = self.min_sleep_ms {
			section.min_sleep_ms = Some(value);
		}
		if let Some(Some(value)) = self.wake_horizon_ms {
			section.wake_horizon_ms = Some(value);
		}
		if let Some(Some(value)) = self.confirm_delay_ms {
			section.confirm_delay_ms = Some(value);
		}
		if let Some(Some(value)) = self.max_copy_bytes {
			section.max_copy_bytes = Some(value);
		}
		if let Some(Some(value)) = self.max_copy_bytes_per_sec {
			section.max_copy_bytes_per_sec = Some(value);
		}
		if let Some(Some(value)) = self.candidates_page_limit {
			section.candidates_page_limit = Some(value);
		}
		if let Some(Some(value)) = self.parked_page_limit {
			section.parked_page_limit = Some(value);
		}
		Ok(section)
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

	pub fn candidates_page_limit(&self) -> u32 {
		self.candidates_page_limit.unwrap_or(200)
	}

	pub fn parked_page_limit(&self) -> u32 {
		self.parked_page_limit.unwrap_or(100)
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
		for (name, value) in [
			("candidates_page_limit", self.candidates_page_limit()),
			("parked_page_limit", self.parked_page_limit()),
		] {
			ensure!(
				(1..=MAX_PAGE_LIMIT).contains(&value),
				"migrate_from_v2.{name} must be between 1 and {MAX_PAGE_LIMIT}, the page sizes V2 accepts"
			);
		}
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
