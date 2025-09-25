use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::namespaces::RunnerConfig;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub variant: Option<keys::RunnerConfigVariant>,
	pub after_name: Option<String>,
	pub limit: usize,
}

#[operation]
pub async fn namespace_runner_config_list(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<(String, RunnerConfig)>> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	let runner_configs = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let (start, end) = if let Some(variant) = input.variant {
				let (start, end) = keys::subspace()
					.subspace(&keys::RunnerConfigByVariantKey::subspace_with_variant(
						input.namespace_id,
						variant,
					))
					.range();

				let start = if let Some(name) = &input.after_name {
					tx.pack(&keys::RunnerConfigByVariantKey::new(
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
					.subspace(&keys::RunnerConfigKey::subspace(input.namespace_id))
					.range();

				let start = if let Some(name) = &input.after_name {
					tx.pack(&keys::RunnerConfigKey::new(
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
					mode: StreamingMode::WantAll,
					limit: Some(input.limit),
					..(start, end).into()
				},
				Serializable,
			)
			.map(|res| match res {
				Ok(entry) => {
					if input.variant.is_some() {
						let (key, config) =
							tx.read_entry::<keys::RunnerConfigByVariantKey>(&entry)?;
						Ok((key.name, config))
					} else {
						let (key, config) = tx.read_entry::<keys::RunnerConfigKey>(&entry)?;
						Ok((key.name, config))
					}
				}
				Err(err) => Err(err.into()),
			})
			.try_collect()
			.await
		})
		.custom_instrument(tracing::info_span!("runner_config_get_local_tx"))
		.await?;

	Ok(runner_configs)
}
