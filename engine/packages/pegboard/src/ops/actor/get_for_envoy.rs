use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub actor_id: Id,
}

#[derive(Debug)]
pub struct Output {
	pub name: String,
	pub namespace_id: Id,
	pub envoy_key: String,
	pub is_connectable: bool,
}

// TODO: Add cache (remember to purge cache when runner changes)
#[operation]
pub async fn pegboard_actor_get_for_runner(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<Output>> {
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let name_key = keys::actor::NameKey::new(input.actor_id);
			let namespace_id_key = keys::actor::NamespaceIdKey::new(input.actor_id);
			let envoy_key_key = keys::actor::EnvoyKeyKey::new(input.actor_id);
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);

			let (name_entry, namespace_id_entry, envoy_key_entry, is_connectable) = tokio::try_join!(
				tx.read_opt(&name_key, Serializable),
				tx.read_opt(&namespace_id_key, Serializable),
				tx.read_opt(&envoy_key_key, Serializable),
				tx.exists(&connectable_key, Serializable),
			)?;

			let (Some(name), Some(namespace_id), Some(envoy_key)) =
				(name_entry, namespace_id_entry, envoy_key_entry)
			else {
				return Ok(None);
			};

			Ok(Some(Output {
				name,
				namespace_id,
				envoy_key,
				is_connectable,
			}))
		})
		.custom_instrument(tracing::info_span!("actor_get_for_runner_tx"))
		.await
}
