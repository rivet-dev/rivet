//! Edits to a cluster runner pool's variables and the cluster secrets they
//! name. Everything here is pure, so the commands that call the Cloud API stay
//! thin and the rules are tested in one place.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};

use crate::cloud::v2::{EnvValue, RunnerPoolSummary, Secret, SecretRef, SecretRollout};

/// The largest secret value the Cloud API accepts, in bytes.
pub const MAX_SECRET_BYTES: usize = 1024;
/// The longest secret name the Cloud API accepts, in characters.
const MAX_SECRET_NAME_CHARS: usize = 64;

/// Checks a variable name: letters, digits and `_`, not starting with a digit.
pub fn validate_env_name(key: &str) -> Result<()> {
	let mut chars = key.chars();
	let valid = match chars.next() {
		Some(first) => {
			(first.is_ascii_alphabetic() || first == '_')
				&& chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
		}
		None => false,
	};
	if !valid {
		bail!(
			"variable name {key:?} is invalid; use letters, digits and `_`, not starting with a digit"
		);
	}
	Ok(())
}

/// Checks a secret name: letters, digits, `_` and `-`, at most 64 characters,
/// not starting or ending with `-`.
pub fn validate_secret_name(name: &str) -> Result<()> {
	let valid = !name.is_empty()
		&& name.chars().count() <= MAX_SECRET_NAME_CHARS
		&& name
			.chars()
			.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
		&& !name.starts_with('-')
		&& !name.ends_with('-');
	if !valid {
		bail!(
			"secret name {name:?} is invalid; use letters, digits, `_` and `-`, at most {MAX_SECRET_NAME_CHARS} characters, not starting or ending with `-`"
		);
	}
	Ok(())
}

/// Parses `--env KEY=VALUE` and `--secret KEY[=SECRET_NAME]` arguments into
/// the variables to set. A key given twice, in either flag or across both, is
/// refused so a typo cannot silently win over the intended value.
pub fn parse_set_args(env: &[String], secrets: &[String]) -> Result<BTreeMap<String, EnvValue>> {
	let mut set = BTreeMap::new();
	for arg in env {
		let Some((key, value)) = arg.split_once('=') else {
			bail!("--env must be KEY=VALUE, got {arg:?}");
		};
		validate_env_name(key)?;
		insert_once(&mut set, key, EnvValue::String(value.to_owned()))?;
	}
	for arg in secrets {
		let (key, name) = arg.split_once('=').unwrap_or((arg, arg));
		if key.is_empty() || name.is_empty() {
			bail!("--secret must be KEY or KEY=SECRET_NAME, got {arg:?}");
		}
		validate_env_name(key)?;
		validate_secret_name(name)?;
		insert_once(
			&mut set,
			key,
			EnvValue::Secret(SecretRef {
				name: name.to_owned(),
				pinned_version: None,
			}),
		)?;
	}
	Ok(set)
}

fn insert_once(set: &mut BTreeMap<String, EnvValue>, key: &str, value: EnvValue) -> Result<()> {
	if set.insert(key.to_owned(), value).is_some() {
		bail!(
			"variable {key} is given more than once; pass each variable once, with either --env or --secret"
		);
	}
	Ok(())
}

/// The secrets `set` names that the cluster does not hold.
pub fn missing_secrets(set: &BTreeMap<String, EnvValue>, existing: &[Secret]) -> Vec<String> {
	let existing: BTreeSet<&str> = existing.iter().map(|secret| secret.name.as_str()).collect();
	let mut missing = BTreeSet::new();
	for value in set.values() {
		match value {
			EnvValue::Secret(secret) => {
				if !existing.contains(secret.name.as_str()) {
					missing.insert(secret.name.clone());
				}
			}
			EnvValue::String(_) => {}
		}
	}
	missing.into_iter().collect()
}

/// Refuses variables that name secrets the cluster does not hold.
pub fn check_secrets_exist(
	cluster: &str,
	set: &BTreeMap<String, EnvValue>,
	existing: &[Secret],
) -> Result<()> {
	let missing = missing_secrets(set, existing);
	if missing.is_empty() {
		return Ok(());
	}
	bail!(
		"cluster {cluster} has no secret named {}; set it first with `rivet cluster secret set <name>`",
		missing.join(", ")
	)
}

/// A secret reference whose rollback pin was dropped by sending `env`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpinned {
	pub key: String,
	pub secret: String,
	pub version: u64,
}

/// The pool's variables after an edit, ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvChange {
	pub env: BTreeMap<String, EnvValue>,
	/// Variables that were pinned by a rollback and now take the latest version.
	pub unpinned: Vec<Unpinned>,
}

/// Applies `set` and `unset` to the pool's `current` variables. Returns
/// `None` when the edit changes nothing, so the pool is left alone and keeps
/// any pins.
///
/// The Cloud API only accepts a pin that was read back unchanged and unpins on
/// every write of `env`, so the result carries no pins and lists the variables
/// that lost one.
pub fn edit_env(
	current: &BTreeMap<String, EnvValue>,
	set: &BTreeMap<String, EnvValue>,
	unset: &BTreeSet<String>,
) -> Option<EnvChange> {
	let mut env = current.clone();
	for (key, value) in set {
		env.insert(key.clone(), value.clone());
	}
	for key in unset {
		env.remove(key);
	}
	if env == *current {
		return None;
	}
	let mut unpinned = Vec::new();
	for (key, value) in &mut env {
		match value {
			EnvValue::Secret(secret) => {
				if let Some(version) = secret.pinned_version.take() {
					unpinned.push(Unpinned {
						key: key.clone(),
						secret: secret.name.clone(),
						version,
					});
				}
			}
			EnvValue::String(_) => {}
		}
	}
	Some(EnvChange { env, unpinned })
}

/// Says which variables an edit unpinned, or `None` when it unpinned none.
pub fn unpinned_message(pool: &str, unpinned: &[Unpinned]) -> Option<String> {
	if unpinned.is_empty() {
		return None;
	}
	Some(format!(
		"unpinned on runner pool {pool}, now taking the secret's latest version: {}",
		unpinned_list(unpinned)
	))
}

/// Lists unpinned variables with the secret and version each was pinned to.
pub fn unpinned_list(unpinned: &[Unpinned]) -> String {
	unpinned
		.iter()
		.map(|entry| {
			format!(
				"{} (secret {}, was pinned at version {})",
				entry.key, entry.secret, entry.version
			)
		})
		.collect::<Vec<_>>()
		.join(", ")
}

/// Names the cluster and, when known, its organization.
pub fn cluster_label(cluster: &str, org: Option<&str>) -> String {
	match org {
		Some(org) => format!("cluster {cluster} (organization {org})"),
		None => format!("cluster {cluster}"),
	}
}

/// The runner pools whose variables name one secret.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SecretUsage {
	/// Pools with at least one reference that takes the latest version, which
	/// roll when the secret is set.
	pub rolling: Vec<String>,
	/// Pools whose every reference a rollback pinned, which do not roll.
	pub pinned: Vec<String>,
}

impl SecretUsage {
	pub fn is_unused(&self) -> bool {
		self.rolling.is_empty() && self.pinned.is_empty()
	}
}

/// Sorts the listed pools by how their variables use the secret `name`.
pub fn secret_usage(name: &str, pools: &[RunnerPoolSummary]) -> SecretUsage {
	let mut usage = SecretUsage::default();
	for pool in pools {
		let mut latest = false;
		let mut pins = BTreeSet::new();
		for value in pool.env.values() {
			match value {
				EnvValue::Secret(secret) if secret.name == name => match secret.pinned_version {
					Some(version) => {
						pins.insert(version);
					}
					None => latest = true,
				},
				EnvValue::Secret(_) | EnvValue::String(_) => {}
			}
		}
		let label = format!("{} in {}", pool.pool, pool.region);
		if latest {
			usage.rolling.push(label);
		} else if !pins.is_empty() {
			let versions = pins
				.iter()
				.map(u64::to_string)
				.collect::<Vec<_>>()
				.join(", ");
			usage
				.pinned
				.push(format!("{label} (pinned at version {versions})"));
		}
	}
	usage
}

/// What `secret set` is about to do, shown before it asks to confirm.
pub fn set_plan(
	cluster: &str,
	org: Option<&str>,
	name: &str,
	latest_version: Option<u64>,
	usage: &SecretUsage,
) -> String {
	let mut lines = vec![format!(
		"setting secret {name} in {}",
		cluster_label(cluster, org)
	)];
	match latest_version {
		Some(version) => lines.push(format!("  the new value replaces version {version}")),
		None => lines.push(format!("  {name} is a new secret")),
	}
	if !usage.rolling.is_empty() {
		lines.push(format!(
			"  runner pools that will roll: {}",
			usage.rolling.join(", ")
		));
	}
	if !usage.pinned.is_empty() {
		lines.push(format!(
			"  runner pools pinned by a rollback, which will NOT roll: {}",
			usage.pinned.join(", ")
		));
	}
	if usage.is_unused() {
		lines.push("  no runner pool uses it yet".to_owned());
	}
	lines.join("\n")
}

/// The result of setting a secret, naming only the pools that actually roll.
pub fn set_outcome(rollout: &SecretRollout, usage: &SecretUsage) -> String {
	if !rollout.changed {
		return format!(
			"secret {} already holds this value at version {}; nothing rolled",
			rollout.name, rollout.latest_version
		);
	}
	let mut message = format!(
		"set secret {} to version {}",
		rollout.name, rollout.latest_version
	);
	if usage.is_unused() {
		message.push_str("; no runner pool uses it yet");
		return message;
	}
	if usage.rolling.is_empty() {
		message.push_str("; no runner pool rolls");
	} else {
		message.push_str(&format!(
			"; rolling runner pools: {}",
			usage.rolling.join(", ")
		));
	}
	if !usage.pinned.is_empty() {
		message.push_str(&format!(
			"; pinned by a rollback and not rolling: {}",
			usage.pinned.join(", ")
		));
	}
	message
}

/// What `secret delete` is about to do, or why the Cloud API would refuse it.
pub fn delete_plan(
	cluster: &str,
	org: Option<&str>,
	name: &str,
	latest_version: u64,
	usage: &SecretUsage,
) -> Result<String> {
	let users: Vec<&str> = usage
		.rolling
		.iter()
		.chain(&usage.pinned)
		.map(String::as_str)
		.collect();
	if !users.is_empty() {
		bail!(
			"secret {name} is still named by runner pools: {}; unset those variables with `rivet cluster env unset` first",
			users.join(", ")
		);
	}
	Ok(format!(
		"deleting secret {name} (latest version {latest_version}) from {}\n  no runner pool uses it",
		cluster_label(cluster, org)
	))
}
