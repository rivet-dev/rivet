use gas::prelude::*;
use universaldb::options::ConflictRangeType;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub envoy_key: String,
}

#[operation]
pub async fn pegboard_envoy_expire(ctx: &OperationCtx, input: &Input) -> Result<()> {
	ctx
		.udb()?
		.run(|tx| {
			async move {
				let tx = tx.with_subspace(keys::subspace());

				let pool_name_key = keys::envoy::PoolNameKey::new(input.namespace_id, input.envoy_key.clone());
				let version_key = keys::envoy::VersionKey::new(input.namespace_id, input.envoy_key.clone());
				let create_ts_key = keys::envoy::CreateTsKey::new(input.namespace_id, input.envoy_key.clone());
				let last_ping_ts_key = keys::envoy::LastPingTsKey::new(input.namespace_id, input.envoy_key.clone());
				let expired_ts_key = keys::envoy::ExpiredTsKey::new(input.namespace_id, input.envoy_key.clone());

				let (
					pool_name_entry,
					version_entry,
					create_ts_entry,
					last_ping_ts_entry,
					expired,
				) = tokio::try_join!(
					tx.read_opt(&pool_name_key, Serializable),
					tx.read_opt(&version_key, Serializable),
					tx.read_opt(&create_ts_key, Serializable),
					tx.read_opt(&last_ping_ts_key, Serializable),
					tx.exists(&expired_ts_key, Serializable),
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
					return Ok(());
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

					tx.write(&expired_ts_key, util::timestamp::now())?;
					tx.delete(
						&keys::ns::ActiveEnvoyKey::new(
							input.namespace_id,
							create_ts,
							input.envoy_key.clone(),
						),
					);
				}

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("envoy_expire_tx"))
		.await
}
