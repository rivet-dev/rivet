use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

use crate::{keys, utils::runner_config_variant};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn pegboard_runner_config_delete(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let delete_pool = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(namespace::keys::subspace());

			// Read existing config to determine variant
			let runner_config_key =
				keys::runner_config::DataKey::new(input.namespace_id, input.name.clone());

			let delete_pool =
				if let Some(config) = tx.read_opt(&runner_config_key, Serializable).await? {
					tx.delete(&runner_config_key);

					// Clear secondary idx
					let variant = runner_config_variant(&config);
					tx.delete(&keys::runner_config::ByVariantKey::new(
						input.namespace_id,
						variant,
						input.name.clone(),
					));

					config.affects_pool()
				} else {
					false
				};

			Ok(delete_pool)
		})
		.custom_instrument(tracing::info_span!("runner_config_delete_tx"))
		.await?;

	// Bump pool when a serverless config is modified
	if delete_pool {
		let res = ctx
			.signal(crate::workflows::runner_pool::Bump {})
			.to_workflow::<crate::workflows::runner_pool::Workflow>()
			.tag("namespace_id", input.namespace_id)
			.tag("runner_name", input.name.clone())
			.graceful_not_found()
			.send()
			.await?;

		if res.is_none() {
			tracing::debug!(namespace_id=?input.namespace_id, name=%input.name, "no runner pool workflow to bump");
		}
	}

	Ok(())
}
