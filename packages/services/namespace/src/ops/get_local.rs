use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use udb_util::{FormalKey, SERIALIZABLE};
use universaldb as udb;

use crate::{errors, keys, types::Namespace};

#[derive(Debug)]
pub struct Input {
	pub namespace_ids: Vec<Id>,
}

#[derive(Debug)]
pub struct Output {
	pub namespaces: Vec<Namespace>,
}

#[operation]
pub async fn namespace_get(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	let namespaces = ctx
		.udb()?
		.run(|tx, _mc| async move {
			futures_util::stream::iter(input.namespace_ids.clone())
				.map(|namespace_id| {
					let tx = tx.clone();

					async move { get_inner(namespace_id, &tx).await }
				})
				.buffer_unordered(1024)
				.try_filter_map(|x| std::future::ready(Ok(x)))
				.try_collect::<Vec<_>>()
				.await
		})
		.custom_instrument(tracing::info_span!("namespace_get_tx"))
		.await?;

	Ok(Output { namespaces })
}

pub(crate) async fn get_inner(
	namespace_id: Id,
	tx: &udb::RetryableTransaction,
) -> std::result::Result<Option<Namespace>, udb::FdbBindingError> {
	let name_key = keys::NameKey::new(namespace_id);
	let display_name_key = keys::DisplayNameKey::new(namespace_id);
	let create_ts_key = keys::CreateTsKey::new(namespace_id);

	let (name_entry, display_name_entry, create_ts_entry) = tokio::try_join!(
		tx.get(&keys::subspace().pack(&name_key), SERIALIZABLE),
		tx.get(&keys::subspace().pack(&display_name_key), SERIALIZABLE),
		tx.get(&keys::subspace().pack(&create_ts_key), SERIALIZABLE),
	)?;

	// Namespace not found
	let Some(name_entry) = name_entry else {
		return Ok(None);
	};

	let name = name_key
		.deserialize(&name_entry)
		.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
	let display_name = display_name_key
		.deserialize(&display_name_entry.ok_or(udb::FdbBindingError::CustomError(
			format!("key should exist: {display_name_key:?}").into(),
		))?)
		.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;
	let create_ts = create_ts_key
		.deserialize(&create_ts_entry.ok_or(udb::FdbBindingError::CustomError(
			format!("key should exist: {create_ts_key:?}").into(),
		))?)
		.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

	Ok(Some(Namespace {
		namespace_id,
		name,
		display_name,
		create_ts,
	}))
}
