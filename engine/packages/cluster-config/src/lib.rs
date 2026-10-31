//! Durable dynamic configuration coordinated by a per-process ephemeral state machine.
use anyhow::Result;
use object_store::ObjectStore;
use rivet_config::{Config, DynamicConfigUpdate};
use rivet_data::generated::rivet_cluster_config_v1::Data as ClusterConfig;
use rivet_env::Env;
use rivet_keys::keys::codec::FormalKey;
use std::sync::Arc;

pub mod cluster_config_machine;
pub mod dynamic_config;
pub mod keys;
pub mod runtime;
pub mod services;
mod store;

use cluster_config_machine::types::{Purpose, ReadResult};
use keys::s3::ClusterConfigKey;

/// Bounded bootstrap before UPS/Topology/Depot: create if absent, validate, then apply locally.
pub async fn load(
	env: Arc<dyn Env>,
	object_store: Arc<dyn ObjectStore>,
	current: &Config,
) -> Result<()> {
	runtime::run_once(env, object_store, current, Purpose::Bootstrap).await
}

/// Inspect the durable snapshot without starting a background service.
/// Runtime readers should use Config::dynamic() or Config::dynamic_watch().
pub async fn fetch_config(object_store: &dyn ObjectStore) -> Result<Option<ClusterConfig>> {
	match store::read(object_store).await {
		ReadResult::Found(snapshot) => Ok(Some(
			ClusterConfigKey::new().deserialize(snapshot.bytes.into())?,
		)),
		ReadResult::Missing => Ok(None),
		ReadResult::Unknown(error) | ReadResult::Invalid(error) => Err(anyhow::Error::msg(error)),
	}
}

fn dynamic_config(config: &ClusterConfig) -> Result<DynamicConfigUpdate> {
	match &config.dynamic {
		Some(bytes) => Ok(serde_json::from_slice(bytes)?),
		None => Ok(DynamicConfigUpdate::default()),
	}
}

/// Only validated snapshots reach the shared config and tracing adapters.
fn validate_and_apply_config(config: &ClusterConfig, current: &Config) -> Result<()> {
	let dynamic = dynamic_config(config)?;
	current.validate_dynamic(&dynamic)?;
	current.apply_dynamic(&dynamic)?;
	dynamic_config::apply_tracing(&dynamic);
	Ok(())
}

/// Bounded read/apply using the same protocol as the periodic Always service.
pub async fn refresh_config(
	env: Arc<dyn Env>,
	object_store: Arc<dyn ObjectStore>,
	current: &Config,
) -> Result<()> {
	runtime::run_once(env, object_store, current, Purpose::Refresh).await
}

/// Merge and validate against one S3 snapshot, then preserve the exact CAS through unknown outcomes.
/// A changed base is reported to the operator instead of silently rebasing over a newer decision.
/// Followers in other processes pick up the change on their next periodic refresh.
pub async fn update_dynamic_config(
	env: Arc<dyn Env>,
	object_store: Arc<dyn ObjectStore>,
	current: &Config,
	update: &DynamicConfigUpdate,
) -> Result<()> {
	runtime::run_once(
		env,
		object_store,
		current,
		Purpose::Update {
			patch: serde_json::to_vec(update)?,
		},
	)
	.await
}


