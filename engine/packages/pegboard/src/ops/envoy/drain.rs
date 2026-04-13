use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use crate::{keys, metrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub pool_name: String,
	pub version: u32,
}

// NOTE: Only applies to serverless
#[operation]
pub async fn pegboard_envoy_drain_older_versions(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let pool_res = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.pool_name.clone())],
			bypass_cache: false,
		})
		.await?;

	let Some(pool) = pool_res.into_iter().next() else {
		return Ok(());
	};

	// Use config's drain_on_version_upgrade if config exists, otherwise default to false
	if !pool.config.drain_on_version_upgrade {
		return Ok(());
	}

	// Scan EnvoyLoadBalancerIdxKey for older versions
	let older_envoys = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			let mut older_envoys = Vec::new();

			let lb_subspace =
				keys::subspace().subspace(&keys::ns::EnvoyLoadBalancerIdxKey::subspace(
					input.namespace_id,
					input.pool_name.clone(),
				));

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&lb_subspace).into()
				},
				Snapshot,
			);

			while let Some(entry) = stream.try_next().await? {
				let (key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

				// Only collect envoys with older versions
				if key.version < input.version {
					older_envoys.push(key.envoy_key);
				}
			}

			Ok(older_envoys)
		})
		.custom_instrument(tracing::info_span!("drain_older_versions_tx"))
		.await?;

	if !older_envoys.is_empty() {
		tracing::info!(
			namespace_id = %input.namespace_id,
			pool_name = %input.pool_name,
			new_version = input.version,
			older_envoy_count = older_envoys.len(),
			"draining older envoy versions due to drain_on_version_upgrade"
		);

		metrics::ENVOY_VERSION_UPGRADE_DRAIN_TOTAL
			.with_label_values(&[&input.namespace_id.to_string(), &input.pool_name])
			.inc_by(older_envoys.len() as u64);

		for envoy_key in older_envoys {
			let receiver_subject =
				crate::pubsub_subjects::EnvoyReceiverSubject::new(input.namespace_id, envoy_key)
					.to_string();

			let message_serialized =
				versioned::ToEnvoyConn::wrap_latest(protocol::ToEnvoyConn::ToEnvoyConnClose)
					.serialize_with_embedded_version(PROTOCOL_VERSION)?;

			ctx.ups()?
				.publish(&receiver_subject, &message_serialized, PublishOpts::one())
				.await?;
		}
	}

	Ok(())
}
