use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_types::namespaces::Namespace;
use universaldb::utils::IsolationLevel::*;

use crate::{errors, keys};

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
		.run(|tx| async move {
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
	tx: &universaldb::Transaction,
) -> Result<Option<Namespace>> {
	let tx = tx.with_subspace(keys::subspace());

	let name_key = keys::NameKey::new(namespace_id);
	let display_name_key = keys::DisplayNameKey::new(namespace_id);
	let create_ts_key = keys::CreateTsKey::new(namespace_id);

	let (name, display_name, create_ts) = tokio::try_join!(
		tx.read_opt(&name_key, Serializable),
		tx.read_opt(&display_name_key, Serializable),
		tx.read_opt(&create_ts_key, Serializable),
	)?;

	// Namespace not found
	let Some(name) = name else {
		return Ok(None);
	};

	let display_name = display_name.context("key should exist")?;
	let create_ts = create_ts.context("key should exist")?;

	Ok(Some(Namespace {
		namespace_id,
		name,
		display_name,
		create_ts,
	}))
}
