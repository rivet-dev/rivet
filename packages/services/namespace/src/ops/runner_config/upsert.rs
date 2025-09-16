use gas::prelude::*;
use rivet_cache::CacheKey;
use udb_util::TxnExt;
use universaldb::options::MutationType;

use crate::{errors, keys, types::RunnerConfig};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub config: RunnerConfig,
}

#[operation]
pub async fn namespace_runner_config_upsert(ctx: &OperationCtx, input: &Input) -> Result<()> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			// TODO: Once other types of configs get added, delete previous config before writing
			txs.write(
				&keys::RunnerConfigKey::new(input.namespace_id, input.name.clone()),
				input.config.clone(),
			)?;

			// Write to secondary idx
			txs.write(
				&keys::RunnerConfigByVariantKey::new(
					input.namespace_id,
					input.config.variant(),
					input.name.clone(),
				),
				input.config.clone(),
			)?;

			match &input.config {
				RunnerConfig::Serverless {
					url,
					slots_per_runner,
					..
				} => {
					// Validate url
					if let Err(err) = url::Url::parse(url) {
						return Ok(Err(errors::RunnerConfig::Invalid {
							reason: format!("invalid serverless url: {err}"),
						}));
					}

					// Validate slots per runner
					if *slots_per_runner == 0 {
						return Ok(Err(errors::RunnerConfig::Invalid {
							reason: "`slots_per_runner` cannot be 0".to_string(),
						}));
					}

					// Sets desired count to 0 if it doesn't exist
					let txs = tx.subspace(rivet_types::keys::pegboard::subspace());
					txs.atomic_op(
						&rivet_types::keys::pegboard::ns::ServerlessDesiredSlotsKey::new(
							input.namespace_id,
							input.name.clone(),
						),
						&0u32.to_le_bytes(),
						MutationType::Add,
					);
				}
			}

			Ok(Ok(()))
		})
		.custom_instrument(tracing::info_span!("runner_config_upsert_tx"))
		.await?
		.map_err(|err| err.build())?;

	// Purge cache in all dcs
	let variant_str = serde_json::to_string(&input.config.variant())?;
	ctx.op(internal::ops::cache::purge_global::Input {
		base_key: format!("namespace.runner_config.{variant_str}.get_global"),
		keys: vec![(input.namespace_id, input.name.as_str()).cache_key().into()],
	})
	.await?;

	// Bump autoscaler in all dcs
	ctx.op(internal::ops::bump_serverless_autoscaler_global::Input {})
		.await?;

	Ok(())
}
