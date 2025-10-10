use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

use crate::{keys, utils::runner_config_variant};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn namespace_runner_config_delete(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let bump_autoscaler = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			// Read existing config to determine variant
			let runner_config_key =
				keys::runner_config::DataKey::new(input.namespace_id, input.name.clone());

			let mut bump_autoscaler = false;

			if let Some(config) = tx.read_opt(&runner_config_key, Serializable).await? {
				bump_autoscaler = config.affects_autoscaler();

				tx.delete(&runner_config_key);

				// Clear secondary idx
				let variant = runner_config_variant(&config);
				tx.delete(&keys::runner_config::ByVariantKey::new(
					input.namespace_id,
					variant,
					input.name.clone(),
				));
			}

			Ok(bump_autoscaler)
		})
		.custom_instrument(tracing::info_span!("runner_config_delete_tx"))
		.await?;

	// Bump autoscaler when a serverless config is modified
	if bump_autoscaler {
		ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
			.send()
			.await?;
	}

	Ok(())
}
