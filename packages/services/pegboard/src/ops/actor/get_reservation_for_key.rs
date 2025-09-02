use gas::prelude::*;
use udb_util::FormalKey;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
}

#[derive(Debug)]
pub struct Output {
	pub reservation_id: Option<Id>,
}

#[operation]
pub async fn pegboard_actor_get_reservation_for_key(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Output> {
	// Create the reservation key
	let reservation_key = keys::epoxy::ns::ReservationByKeyKey::new(
		input.namespace_id,
		input.name.clone(),
		input.key.clone(),
	);

	// Get the reservation ID using optimistic read (global consistency)
	let value = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: keys::subspace().pack(&reservation_key),
		})
		.await?
		.value;

	// Deserialize the reservation ID if it exists
	let reservation_id = match value {
		Some(value) => Some(reservation_key.deserialize(&value)?),
		None => None,
	};

	Ok(Output { reservation_id })
}
