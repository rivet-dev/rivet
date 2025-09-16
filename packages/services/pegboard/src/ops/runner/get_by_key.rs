use anyhow::*;
use gas::prelude::*;
use rivet_types::runners::Runner;
use udb_util::{SERIALIZABLE, TxnExt};
use universaldb as udb;

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
		.run(|tx, _mc| {
			let dc_name = dc_name.to_string();
			let input = input.clone();
			async move {
				let txs = tx.subspace(keys::subspace());

				// Look up runner by key
				let runner_by_key_key =
					keys::ns::RunnerByKeyKey::new(input.namespace_id, input.name, input.key);

				let runner_data = txs.read_opt(&runner_by_key_key, SERIALIZABLE).await?;

				if let Some(data) = runner_data {
					// Get full runner details using the runner_id
					let runner = super::get::get_inner(&dc_name, &tx, data.runner_id).await?;
					std::result::Result::<_, udb::FdbBindingError>::Ok(runner)
				} else {
					std::result::Result::<_, udb::FdbBindingError>::Ok(None)
				}
			}
		})
		.await?;

	Ok(Output { runner })
}
