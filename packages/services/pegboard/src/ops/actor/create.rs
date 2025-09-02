use anyhow::Result;
use gas::prelude::*;
use rivet_api_client::{Method, request_remote_datacenter};
use rivet_types::actors::{Actor, CrashPolicy};

#[derive(Debug)]
pub struct Input {
	pub actor_id: Id,
	pub namespace_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,
	pub input: Option<String>,
	/// If true, will handle ForwardToDatacenter errors by forwarding the request to the correct datacenter.
	/// Used by api-public. api-peer should set this to false.
	pub forward_request: bool,
	/// Datacenter to create the actor in
	///
	/// Providing this value will cause an error if attempting to create an actor where the key is
	/// reserved in a different datacenter.
	pub datacenter_name: Option<String>,
}

#[derive(Debug)]
pub struct Output {
	pub actor: Actor,
}

#[operation]
pub async fn pegboard_actor_create(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	// Set up subscriptions before dispatching workflow
	let mut create_sub = ctx
		.subscribe::<crate::workflows::actor::CreateComplete>(("actor_id", input.actor_id))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<crate::workflows::actor::Failed>(("actor_id", input.actor_id))
		.await?;
	let mut destroy_sub = ctx
		.subscribe::<crate::workflows::actor::DestroyStarted>(("actor_id", input.actor_id))
		.await?;

	// Dispatch actor workflow
	ctx.workflow(crate::workflows::actor::Input {
		actor_id: input.actor_id,
		name: input.name.clone(),
		key: input.key.clone(),
		namespace_id: input.namespace_id,
		runner_name_selector: input.runner_name_selector.clone(),
		input: input.input.clone(),
		crash_policy: input.crash_policy,
	})
	.tag("actor_id", input.actor_id)
	.dispatch()
	.await?;

	// Wait for actor creation to complete, fail, or be destroyed
	tokio::select! {
		res = create_sub.next() => { res?; },
		res = fail_sub.next() => {
			let msg = res?;
			let error = msg.into_body().error;

			// Check if this request needs to be forwarded
			//
			// We cannot forward if `datacenter_name` is specified because this actor is being
			// restricted to the given datacenter.
			if input.forward_request && input.datacenter_name.is_none() {
				if let crate::errors::Actor::KeyReservedInDifferentDatacenter { datacenter_label } = &error {
					// Forward the request to the correct datacenter
					return forward_to_datacenter(
						ctx,
						*datacenter_label,
						input.namespace_id,
						input.name.clone(),
						input.key.clone(),
						input.runner_name_selector.clone(),
						input.input.clone(),
					input.crash_policy
					).await;
				}
			}

			// Otherwise, return the error as-is
			return Err(error.build());
		}
		res = destroy_sub.next() => {
			res?;
			return Err(crate::errors::Actor::DestroyedDuringCreation.build());
		}
	}

	// Fetch the created actor
	let actors_res = ctx
		.op(crate::ops::actor::get::Input {
			actor_ids: vec![input.actor_id],
		})
		.await?;

	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.ok_or_else(|| crate::errors::Actor::NotFound.build())?;

	Ok(Output { actor })
}

/// Forward the actor creation request to the correct datacenter
async fn forward_to_datacenter(
	ctx: &OperationCtx,
	datacenter_label: u16,
	namespace_id: Id,
	name: String,
	key: Option<String>,
	runner_name_selector: String,
	input: Option<String>,
	crash_policy: CrashPolicy,
) -> Result<Output> {
	// Get the datacenter configuration
	let _target_dc = ctx
		.config()
		.dc_for_label(datacenter_label)
		.ok_or_else(|| anyhow::anyhow!("datacenter not found for label {}", datacenter_label))?;

	// Get namespace name for the remote call
	let namespace = ctx
		.op(namespace::ops::get_global::Input { namespace_id })
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Generate a new actor ID with the correct datacenter label
	let actor_id = Id::new_v1(datacenter_label);

	// Make request to remote datacenter
	let response = request_remote_datacenter::<rivet_api_types::actors::create::CreateResponse>(
		ctx.config(),
		datacenter_label,
		"/actors",
		Method::POST,
		Default::default(), // Empty headers
		Some(&rivet_api_types::actors::create::CreateQuery {
			namespace: namespace.name.clone(),
		}),
		Some(&rivet_api_types::actors::create::CreateRequest {
			actor_id,
			name,
			key,
			input,
			runner_name_selector,
			crash_policy,
		}),
	)
	.await?;

	Ok(Output {
		actor: response.actor,
	})
}
