use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::create::{CreateQuery, CreateRequest, CreateResponse};

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

	let res = ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id: body.actor_id,
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key: body.key.clone(),
			runner_name_selector: body.runner_name_selector.clone(),
			input: body.input.clone(),
			crash_policy: body.crash_policy,
			// Don't forward request since this request should be already forwarded if it is going
			// to be forward.
			//
			// This should never throw a request needs to be forwarded error. If it does, something
			// is broken.
			forward_request: false,
			// api-peer is always creating in its own datacenter
			datacenter_name: None,
		})
		.await?;

	let actor = res.actor;

	Ok(CreateResponse { actor })
}
