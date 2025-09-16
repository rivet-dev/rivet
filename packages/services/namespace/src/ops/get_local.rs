use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use udb_util::{SERIALIZABLE, TxnExt};
use universaldb as udb;

use crate::{errors, keys, types::Namespace};

#[derive(Debug)]
pub struct Input {
	pub namespace_ids: Vec<Id>,
}

#[operation]
pub async fn namespace_get_local(ctx: &OperationCtx, input: &Input) -> Result<Vec<Namespace>> {
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

	Ok(namespaces)
}

pub(crate) async fn get_inner(
	namespace_id: Id,
	tx: &udb::RetryableTransaction,
) -> std::result::Result<Option<Namespace>, udb::FdbBindingError> {
	let txs = tx.subspace(keys::subspace());

	let name_key = keys::NameKey::new(namespace_id);
	let display_name_key = keys::DisplayNameKey::new(namespace_id);
	let create_ts_key = keys::CreateTsKey::new(namespace_id);

	let (name, display_name, create_ts) = tokio::try_join!(
		txs.read_opt(&name_key, SERIALIZABLE),
		txs.read_opt(&display_name_key, SERIALIZABLE),
		txs.read_opt(&create_ts_key, SERIALIZABLE),
	)?;

	// Namespace not found
	let Some(name) = name else {
		return Ok(None);
	};

	let display_name = display_name.ok_or(udb::FdbBindingError::CustomError(
		format!("key should exist: {display_name_key:?}").into(),
	))?;
	let create_ts = create_ts.ok_or(udb::FdbBindingError::CustomError(
		format!("key should exist: {create_ts_key:?}").into(),
	))?;

	Ok(Some(Namespace {
		namespace_id,
		name,
		display_name,
		create_ts,
	}))
}
