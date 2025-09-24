use gas::prelude::*;
use rivet_cache::CacheKey;
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys, utils::runner_config_variant};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn namespace_runner_config_delete(ctx: &OperationCtx, input: &Input) -> Result<()> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			// Read existing config to determine variant
			let runner_config_key =
				keys::RunnerConfigKey::new(input.namespace_id, input.name.clone());

			if let Some(config) = tx.read_opt(&runner_config_key, Serializable).await? {
				tx.delete(&runner_config_key);

				// Clear secondary idx
				tx.delete(&keys::RunnerConfigByVariantKey::new(
					input.namespace_id,
					runner_config_variant(&config),
					input.name.clone(),
				));
			}

			Ok(())
		})
		.custom_instrument(tracing::info_span!("runner_config_upsert_tx"))
		.await?;

	// Purge cache in all dcs
	ctx.op(internal::ops::cache::purge_global::Input {
		base_key: "namespace.runner_config.{}.get_global".to_string(),
		keys: vec![(input.namespace_id, input.name.as_str()).cache_key().into()],
	})
	.await?;

	// Bump autoscaler in all dcs
	ctx.op(internal::ops::bump_serverless_autoscaler_global::Input {})
		.await?;

	Ok(())
}
