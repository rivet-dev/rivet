use gas::prelude::*;
use universaldb::options::ConflictRangeType;
use universaldb::utils::IsolationLevel::*;
use xxhash_rust::xxh3::xxh3_128_with_seed;

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_key: String,
	/// Re-read freshness markers inside the expire transaction before deleting any keys.
	///
	/// This must use serializable reads so a heartbeat committed after a stale allocator
	/// observation conflicts with this transaction instead of being missed.
	#[serde(default)]
	pub skip_if_fresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub did_expire: bool,
}

#[operation]
pub async fn pegboard_envoy_expire(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	expire_with_pools(ctx.config(), ctx.pools(), input).await
}

pub async fn expire_with_pools(
	config: &rivet_config::Config,
	pools: &rivet_pools::PoolsHandle,
	input: &Input,
) -> Result<Output> {
	let envoy_eligible_threshold = config.pegboard().envoy_eligible_threshold();

	pools
		.udb()?
		.txn("pegboard_envoy_expire", |tx| {
			async move {
				let tx = tx.with_subspace(keys::subspace());
				let now = util::timestamp::now();

				let pool_name_key = keys::envoy::PoolNameKey::new(input.namespace_id, input.envoy_key.clone());
				let version_key = keys::envoy::VersionKey::new(input.namespace_id, input.envoy_key.clone());
				let create_ts_key = keys::envoy::CreateTsKey::new(input.namespace_id, input.envoy_key.clone());
				let last_ping_ts_key = keys::envoy::LastPingTsKey::new(input.namespace_id, input.envoy_key.clone());
				let expired_ts_key = keys::envoy::ExpiredTsKey::new(input.namespace_id, input.envoy_key.clone());
				let virtual_nodes_key = keys::envoy::VirtualNodesKey::new(input.namespace_id, input.envoy_key.clone());

				if input.skip_if_fresh {
					let (last_ping_ts, expired_ts) = tokio::try_join!(
						tx.read_opt(&last_ping_ts_key, Serializable),
						tx.read_opt(&expired_ts_key, Serializable),
					)?;

					if expired_ts.is_some()
						|| last_ping_ts.is_some_and(|ts| ts >= now - envoy_eligible_threshold)
					{
						return Ok(Output { did_expire: false });
					}
				}

				let (
					pool_name_entry,
					version_entry,
					create_ts_entry,
					last_ping_ts_entry,
					expired,
					virtual_nodes_entry,
				) = tokio::try_join!(
					tx.read_opt(&pool_name_key, Serializable),
					tx.read_opt(&version_key, Serializable),
					tx.read_opt(&create_ts_key, Serializable),
					tx.read_opt(&last_ping_ts_key, Serializable),
					tx.exists(&expired_ts_key, Serializable),
					tx.read_opt(&virtual_nodes_key, Serializable),
				)?;

				let (
					Some(pool_name),
					Some(version),
					Some(create_ts),
					Some(old_last_ping_ts),
				) = (
					pool_name_entry,
					version_entry,
					create_ts_entry,
					last_ping_ts_entry,
				)
				else {
					tracing::debug!(namespace_id=?input.namespace_id, envoy_key=%input.envoy_key, "envoy not found");
					return Ok(Output { did_expire: false });
				};

				if !expired {
					let idx_key = keys::ns::EnvoyLoadBalancerIdxKey::new(
						input.namespace_id,
						pool_name.clone(),
						version,
						old_last_ping_ts,
						input.envoy_key.clone(),
					);

					tx.add_conflict_key(&idx_key, ConflictRangeType::Read)?;
					tx.delete(&idx_key);

					if let Some(virtual_nodes) = virtual_nodes_entry {
						for i in 0..virtual_nodes {
							tx.delete(&keys::ns::EnvoyHashIdxKey::new(
								input.namespace_id,
								pool_name.clone(),
								version,
								xxh3_128_with_seed(input.envoy_key.as_bytes(), i as u64).to_be_bytes(),
								input.envoy_key.clone(),
							));
						}
					}

					tx.write(&expired_ts_key, now)?;
					tx.delete(&virtual_nodes_key);
					tx.delete(
						&keys::ns::ActiveEnvoyKey::new(
							input.namespace_id,
							create_ts,
							input.envoy_key.clone(),
						),
					);
					tx.delete(
						&keys::ns::ActiveEnvoyByNameKey::new(
							input.namespace_id,
							pool_name.clone(),
							create_ts,
							input.envoy_key.clone(),
						),
					);
				}

				Ok(Output { did_expire: !expired })
			}
		})
		.custom_instrument(tracing::info_span!("envoy_expire_tx"))
		.await
}
