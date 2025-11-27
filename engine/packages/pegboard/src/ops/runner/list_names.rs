use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub after_name: Option<String>,
	pub limit: usize,
}

#[derive(Debug)]
pub struct Output {
	pub names: Vec<String>,
}

#[operation]
pub async fn pegboard_runner_list_names(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let names = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let runner_name_subspace =
				keys::subspace().subspace(&keys::ns::RunnerNameKey::subspace(input.namespace_id));
			let (start, end) = runner_name_subspace.range();

			let start = if let Some(name) = &input.after_name {
				tx.pack(&keys::ns::RunnerNameKey::new(
					input.namespace_id,
					name.clone(),
				))
			} else {
				start
			};

			tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::Exact,
					limit: Some(input.limit),
					..(start, end).into()
				},
				// NOTE: This is not Serializable to prevent contention with inserting new names
				Snapshot,
			)
			.map(|res| {
				let key = tx.unpack::<keys::ns::RunnerNameKey>(res?.key())?;
				Ok(key.name)
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("runner_list_names_tx"))
		.await?;

	Ok(Output { names })
}
