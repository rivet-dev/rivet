use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::keys::namespace::runner_config::RunnerConfigVariant;
use rivet_types::runner_configs::RunnerConfig;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub variant: Option<RunnerConfigVariant>,
	pub after_name: Option<String>,
	pub limit: usize,
}

// TODO: Needs to return default configs if they exist (currently no way to list from epoxy)
#[operation]
pub async fn namespace_runner_config_list(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<(String, RunnerConfig)>> {
	let runner_configs = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let (start, end) = if let Some(variant) = input.variant {
				let (start, end) = keys::subspace()
					.subspace(&keys::runner_config::ByVariantKey::subspace_with_variant(
						input.namespace_id,
						variant,
					))
					.range();

				let start = if let Some(name) = &input.after_name {
					tx.pack(&keys::runner_config::ByVariantKey::new(
						input.namespace_id,
						variant,
						name.clone(),
					))
				} else {
					start
				};

				(start, end)
			} else {
				let (start, end) = keys::subspace()
					.subspace(&keys::runner_config::DataKey::subspace(input.namespace_id))
					.range();

				let start = if let Some(name) = &input.after_name {
					tx.pack(&keys::runner_config::DataKey::new(
						input.namespace_id,
						name.clone(),
					))
				} else {
					start
				};

				(start, end)
			};

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::Exact,
					limit: Some(input.limit),
					..(start, end).into()
				},
				Serializable,
			)
			.map(|res| match res {
				Ok(entry) => {
					if input.variant.is_some() {
						let (key, config) =
							tx.read_entry::<keys::runner_config::ByVariantKey>(&entry)?;
						Ok((key.name, config))
					} else {
						let (key, config) =
							tx.read_entry::<keys::runner_config::DataKey>(&entry)?;
						Ok((key.name, config))
					}
				}
				Err(err) => Err(err.into()),
			})
			.try_collect()
			.await
		})
		.custom_instrument(tracing::info_span!("runner_config_list_tx"))
		.await?;

	Ok(runner_configs)
}
