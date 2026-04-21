use anyhow::Result;
use gas::prelude::*;
use rivet_types::actors::Actor;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
	/// If provided, limits fanout to only enabled dcs.
	pub pool_name: Option<String>,
	pub fetch_error: bool,
}

#[derive(Debug)]
pub enum Output {
	Found { actor: Actor },
	NotFound,
	Forward { dc_label: u16 },
}

#[operation]
pub async fn pegboard_actor_get_for_key(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// Get the reservation ID for this key
	let reservation_res = ctx
		.op(crate::ops::actor::get_reservation_for_key::Input {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			key: input.key.clone(),
			pool_name: input.pool_name.clone(),
		})
		.await?;

	// If no reservation exists, no actor exists
	let Some(reservation_id) = reservation_res.reservation_id else {
		return Ok(Output::NotFound);
	};

	// Check if the actor is in the current datacenter
	if reservation_id.label() == ctx.config().dc_label() {
		let actors_res = ctx
			.op(crate::ops::actor::list_for_ns::Input {
				namespace_id: input.namespace_id,
				name: input.name.clone(),
				key: Some(input.key.clone()),
				include_destroyed: false,
				created_before: None,
				limit: 1,
				fetch_error: input.fetch_error,
			})
			.await?;

		let Some(actor) = actors_res.actors.into_iter().next() else {
			return Ok(Output::NotFound);
		};

		Ok(Output::Found { actor })
	} else {
		Ok(Output::Forward {
			dc_label: reservation_id.label(),
		})
	}
}
