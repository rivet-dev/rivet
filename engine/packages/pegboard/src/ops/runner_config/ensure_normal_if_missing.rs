use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

use crate::{keys, utils};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

/// Creates a default normal runner config for this namespace and runner name if one does not
/// already exist. Returns `true` when the config was created.
#[operation]
pub async fn pegboard_runner_config_ensure_normal_if_missing(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<bool> {
	let created_runner_config = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			let runner_config_key =
				keys::runner_config::DataKey::new(input.namespace_id, input.name.clone());

			if tx
				.read_opt(&runner_config_key, Serializable)
				.await?
				.is_some()
			{
				return Ok(false);
			}

			let runner_config = rivet_types::runner_configs::RunnerConfig {
				kind: rivet_types::runner_configs::RunnerConfigKind::Normal {},
				metadata: None,
				drain_on_version_upgrade: false,
			};

			tx.write(&runner_config_key, runner_config.clone())?;
			tx.write(
				&keys::runner_config::ByVariantKey::new(
					input.namespace_id,
					utils::runner_config_variant(&runner_config),
					input.name.clone(),
				),
				runner_config,
			)?;

			Ok(true)
		})
		.custom_instrument(tracing::info_span!(
			"runner_config_ensure_normal_if_missing_tx"
		))
		.await?;

	if created_runner_config {
		utils::purge_runner_config_caches(ctx.cache(), input.namespace_id, &input.name).await?;
	}

	Ok(created_runner_config)
}
