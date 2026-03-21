use gas::prelude::*;
use universaldb::options::ConflictRangeType;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_key: String,
	pub update_lb: bool,
	pub rtt: u32,
}

#[operation]
pub async fn pegboard_envoy_update_ping(ctx: &OperationCtx, input: &Input) -> Result<()> {
	ctx
		.udb()?
		.run(|tx| {
			async move {
				let tx = tx.with_subspace(keys::subspace());

				let pool_name_key = keys::envoy::PoolNameKey::new(input.namespace_id, input.envoy_key.clone());
				let version_key = keys::envoy::VersionKey::new(input.namespace_id, input.envoy_key.clone());
				let last_ping_ts_key = keys::envoy::LastPingTsKey::new(input.namespace_id, input.envoy_key.clone());
				let expired_ts_key = keys::envoy::ExpiredTsKey::new(input.namespace_id, input.envoy_key.clone());

				let (
					pool_name_entry,
					version_entry,
					last_ping_ts_entry,
					expired,
				) = tokio::try_join!(
					tx.read_opt(&pool_name_key, Serializable),
					tx.read_opt(&version_key, Serializable),
					tx.read_opt(&last_ping_ts_key, Serializable),
					tx.exists(&expired_ts_key, Serializable),
				)?;

				let (
					Some(pool_name),
					Some(version),
					Some(old_last_ping_ts),
				) = (
					pool_name_entry,
					version_entry,
					last_ping_ts_entry,
				)
				else {
					tracing::debug!(namespace_id=?input.namespace_id, envoy_key=%input.envoy_key, "envoy has not initiated yet");
					return Ok(());
				};

				let last_ping_ts = util::timestamp::now();

				// Write new ping
				tx.write(&last_ping_ts_key, last_ping_ts)?;

				let last_rtt_key = keys::envoy::LastRttKey::new(input.namespace_id, input.envoy_key.clone());
				tx.write(&last_rtt_key, input.rtt)?;

				if input.update_lb && !expired {
					let old_lb_key = keys::ns::EnvoyLoadBalancerIdxKey::new(
						input.namespace_id,
						pool_name.clone(),
						version,
						old_last_ping_ts,
						input.envoy_key.clone(),
					);

					// Add read conflict
					tx.add_conflict_key(&old_lb_key, ConflictRangeType::Read)?;

					// Clear old key
					tx.delete(&old_lb_key);

					tx.write(
						&keys::ns::EnvoyLoadBalancerIdxKey::new(
							input.namespace_id,
							pool_name.clone(),
							version,
							last_ping_ts,
							input.envoy_key.clone(),
						),
						(),
					)?;
				}

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("envoy_update_ping_tx"))
		.await
}
