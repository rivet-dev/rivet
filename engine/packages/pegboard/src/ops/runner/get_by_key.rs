use anyhow::Result;
use gas::prelude::*;
use rivet_types::runners::Runner;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub runner: Option<Runner>,
}

#[operation]
pub async fn pegboard_runner_get_by_key(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let dc_name = ctx.config().dc_name()?.to_string();

	let runner = ctx
		.udb()?
		.run(|tx| {
			let dc_name = dc_name.to_string();
			let input = input.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());

				// Look up runner by key
				let runner_by_key_key =
					keys::ns::RunnerByKeyKey::new(input.namespace_id, input.name, input.key);

				let runner_data = tx.read_opt(&runner_by_key_key, Serializable).await?;

				if let Some(data) = runner_data {
					// Get full runner details using the runner_id
					let runner = super::get::get_inner(&dc_name, &tx, data.runner_id).await?;
					Ok(runner)
				} else {
					Ok(None)
				}
			}
		})
		.custom_instrument(tracing::info_span!("runner_get_by_key_tx"))
		.await?;

	Ok(Output { runner })
}
