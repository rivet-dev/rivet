use gas::prelude::*;
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
				keys::runner_config::DataKey::new(input.namespace_id, input.name.clone());

			if let Some(config) = tx.read_opt(&runner_config_key, Serializable).await? {
				tx.delete(&runner_config_key);

				// Clear secondary idx
				tx.delete(&keys::runner_config::ByVariantKey::new(
					input.namespace_id,
					runner_config_variant(&config),
					input.name.clone(),
				));
			}

			Ok(())
		})
		.custom_instrument(tracing::info_span!("runner_config_delete_tx"))
		.await?;

	// Bump autoscaler
	ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
		.send()
		.await?;

	Ok(())
}
