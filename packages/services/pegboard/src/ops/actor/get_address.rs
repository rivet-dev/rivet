use gas::prelude::*;
use rivet_key_data::generated::pegboard_runner_address_v1::Data as AddressKeyData;
use udb_util::{FormalKey, SERIALIZABLE};
use universaldb as udb;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub actor_id: Id,
	pub address_name: String,
}

#[operation]
pub async fn pegboard_actor_get_address(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<AddressKeyData>> {
	let address = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let runner_id_key = keys::actor::RunnerIdKey::new(input.actor_id);
			let connectable_key = keys::actor::ConnectableKey::new(input.actor_id);
			let (runner_id_entry, connectable_entry) = tokio::try_join!(
				tx.get(&keys::subspace().pack(&runner_id_key), SERIALIZABLE),
				tx.get(&keys::subspace().pack(&connectable_key), SERIALIZABLE),
			)?;

			let Some(runner_id_entry) = runner_id_entry else {
				// Actor does not exist
				return Ok(None);
			};

			if connectable_entry.is_none() {
				// Actor is not ready yet
				return Ok(None);
			}

			let runner_id = runner_id_key
				.deserialize(&runner_id_entry)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			let address_key = keys::runner::AddressKey::new(runner_id, input.address_name.clone());
			let address_entry = tx
				.get(&keys::subspace().pack(&address_key), SERIALIZABLE)
				.await?;

			let Some(address_entry) = address_entry else {
				// Address does not exist
				return Ok(None);
			};

			let address = address_key
				.deserialize(&address_entry)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

			Ok(Some(address))
		})
		.custom_instrument(tracing::info_span!("actor_get_address_tx"))
		.await?;

	Ok(address)
}
