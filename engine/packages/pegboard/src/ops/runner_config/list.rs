use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::keys::namespace::runner_config::RunnerConfigVariant;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnerConfig {
	pub name: String,
	pub config: rivet_types::runner_configs::RunnerConfig,
	/// Unset if the runner's metadata endpoint has never returned `envoyProtocolVersion``
	pub protocol_version: Option<u16>,
}

#[operation]
pub async fn pegboard_runner_config_list(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Vec<RunnerConfig>> {
	let runner_configs = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(namespace::keys::subspace());

			let (start, end) = if let Some(variant) = input.variant {
				let (start, end) = namespace::keys::subspace()
					.subspace(&keys::runner_config::ByVariantKey::subspace_with_variant(
						input.namespace_id,
						variant,
					))
					.range();

				let start = if let Some(name) = &input.after_name {
					universaldb::utils::end_of_key_range(&tx.pack(
						&keys::runner_config::ByVariantKey::new(
							input.namespace_id,
							variant,
							name.clone(),
						),
					))
				} else {
					start
				};

				(start, end)
			} else {
				let (start, end) = namespace::keys::subspace()
					.subspace(&keys::runner_config::DataKey::subspace(input.namespace_id))
					.range();

				let start = if let Some(name) = &input.after_name {
					universaldb::utils::end_of_key_range(&tx.pack(
						&keys::runner_config::DataKey::new(input.namespace_id, name.clone()),
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
			.map(|res| {
				let tx = tx.clone();
				async move {
					match res {
						Ok(entry) => {
							if input.variant.is_some() {
								let (key, config) =
									tx.read_entry::<keys::runner_config::ByVariantKey>(&entry)?;
								let protocol_version = tx
									.read_opt(
										&keys::runner_config::ProtocolVersionKey::new(
											input.namespace_id,
											key.name.clone(),
										),
										Serializable,
									)
									.await?;

								Ok(RunnerConfig {
									name: key.name,
									config,
									protocol_version,
								})
							} else {
								let (key, config) =
									tx.read_entry::<keys::runner_config::DataKey>(&entry)?;
								let protocol_version = tx
									.read_opt(
										&keys::runner_config::ProtocolVersionKey::new(
											input.namespace_id,
											key.name.clone(),
										),
										Serializable,
									)
									.await?;

								Ok(RunnerConfig {
									name: key.name,
									config,
									protocol_version,
								})
							}
						}
						Err(err) => Err(err.into()),
					}
				}
			})
			.buffer_unordered(512)
			.try_collect()
			.await
		})
		.custom_instrument(tracing::info_span!("runner_config_list_tx"))
		.await?;

	Ok(runner_configs)
}
