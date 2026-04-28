use anyhow::ensure;
use gas::prelude::*;
use universaldb::utils::FormalKey;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
	/// If provided, limits fanout to only enabled dcs.
	pub pool_name: Option<String>,
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
	// TODO: See RVT-6224
	let replicas = if let Some(pool_name) = &input.pool_name {
		let res = ctx
			.op(
				crate::ops::runner::list_runner_config_epoxy_replica_ids::Input {
					namespace_id: input.namespace_id,
					runner_name: pool_name.clone(),
				},
			)
			.await?;

		ensure!(
			res.replicas.contains(&ctx.config().epoxy_replica_id()),
			"get_reservation_for_key called outside the scoped runner replica set"
		);

		Some(res.replicas)
	} else {
		None
	};

	let reservation_key = keys::epoxy::ns::ReservationByKeyKey::new(
		input.namespace_id,
		input.name.clone(),
		input.key.clone(),
	);
	let value = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: keys::subspace().pack(&reservation_key),
			caching_behavior: epoxy::protocol::CachingBehavior::Optimistic,
			target_replicas: replicas,
			save_empty: false,
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
