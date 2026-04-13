use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub actor_id: Id,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
	pub name: String,
	pub namespace_id: Id,
}

#[operation]
pub async fn pegboard_actor_get_for_kv(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<Output>> {
	ctx.cache()
		.clone()
		.request()
		.fetch_one_json("actor.actor_get_for_kv", input.actor_id, {
			move |mut cache, key| async move {
				let output = ctx
					.udb()?
					.run(|tx| async move {
						let tx = tx.with_subspace(keys::subspace());

						let name_key = keys::actor::NameKey::new(input.actor_id);
						let namespace_id_key = keys::actor::NamespaceIdKey::new(input.actor_id);

						let (name_entry, namespace_id_entry) = tokio::try_join!(
							tx.read_opt(&name_key, Serializable),
							tx.read_opt(&namespace_id_key, Serializable),
						)?;

						let (Some(name), Some(namespace_id)) = (name_entry, namespace_id_entry)
						else {
							return Ok(None);
						};

						Ok(Some(Output { name, namespace_id }))
					})
					.custom_instrument(tracing::info_span!("actor_get_for_kv_tx"))
					.await?;

				if let Some(output) = output {
					cache.resolve(&key, output);
				}

				Ok(cache)
			}
		})
		.await
}
