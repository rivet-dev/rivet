use anyhow::Result;
use gas::prelude::*;
use rivet_api_client::{Method, request_remote_datacenter};
use rivet_types::actors::Actor;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
}

#[derive(Debug)]
pub struct Output {
	pub actor: Option<Actor>,
}

#[operation]
pub async fn pegboard_actor_get_for_key(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// Get the reservation ID for this key
	let reservation_res = ctx
		.op(crate::ops::actor::get_reservation_for_key::Input {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			key: input.key.clone(),
		})
		.await?;

	// If no reservation exists, no actor exists
	let Some(reservation_id) = reservation_res.reservation_id else {
		return Ok(Output { actor: None });
	};

	// Check if the actor is in the current datacenter
	if reservation_id.label() == ctx.config().dc_label() {
		// Local datacenter - get the actor directly
		let actors_res = ctx
			.op(crate::ops::actor::list_for_ns::Input {
				namespace_id: input.namespace_id,
				name: input.name.clone(),
				key: Some(input.key.clone()),
				include_destroyed: false,
				created_before: None,
				limit: 1,
			})
			.await?;

		Ok(Output {
			actor: actors_res.actors.into_iter().next(),
		})
	} else {
		// Remote datacenter - request from the correct datacenter
		let _target_dc = ctx
			.config()
			.dc_for_label(reservation_id.label())
			.ok_or_else(|| {
				anyhow::anyhow!("datacenter not found for label {}", reservation_id.label())
			})?;

		// Get namespace name for the remote call
		let namespace = ctx
			.op(namespace::ops::get_global::Input {
				namespace_id: input.namespace_id,
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		// Make request to remote datacenter
		let res = request_remote_datacenter::<rivet_api_types::actors::list::ListResponse>(
			ctx.config(),
			reservation_id.label(),
			"/actors",
			Method::GET,
			Default::default(), // Empty headers
			Some(&rivet_api_types::actors::list::ListQuery {
				namespace: namespace.name.clone(),
				name: Some(input.name.clone()),
				key: Some(input.key.clone()),
				actor_ids: None,
				include_destroyed: Some(false),
				limit: Some(1),
				cursor: None,
			}),
			Option::<&()>::None,
		)
		.await?;

		Ok(Output {
			actor: res.actors.into_iter().next(),
		})
	}
}
