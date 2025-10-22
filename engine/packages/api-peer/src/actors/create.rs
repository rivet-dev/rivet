use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::create::{CreateQuery, CreateRequest, CreateResponse};

#[tracing::instrument(skip_all)]
pub async fn create(
	ctx: ApiCtx,
	_path: (),
	query: CreateQuery,
	body: CreateRequest,
) -> Result<CreateResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let actor_id = Id::new_v1(ctx.config().dc_label());

	let res = ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key: body.key,
			runner_name_selector: body.runner_name_selector,
			input: body.input.clone(),
			crash_policy: body.crash_policy,
			// NOTE: This can forward if the user attempts to create an actor with a target dc and this dc
			// ends up forwarding to another.
			forward_request: true,
			// api-peer is always creating in its own datacenter
			datacenter_name: None,
		})
		.await?;

	Ok(CreateResponse { actor: res.actor })
}
