use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_types::actors::CrashPolicy;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct CreateQuery {
	pub namespace: String,
	pub datacenter: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsCreateRequest)]
pub struct CreateRequest {
	pub name: String,
	pub key: Option<String>,
	pub input: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsCreateResponse)]
pub struct CreateResponse {
	pub actor: rivet_types::actors::Actor,
}

/// ## Datacenter Round Trips
///
/// **If actor is created in the current datacenter:**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)
///
/// **If actor is created in a different datacenter:**
///
/// 3 round trips:
/// - namespace::ops::resolve_for_name_global
/// - POST /actors to remote datacenter
/// - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)
///
/// actor::get will always be in the same datacenter.
#[utoipa::path(
    post,
	operation_id = "actors_create",
    path = "/actors",
    params(CreateQuery),
    request_body(content = CreateRequest, content_type = "application/json"),
    responses(
        (status = 200, body = CreateResponse),
    ),
)]
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

	// Determine which datacenter to create the actor in
	let target_dc_label = if let Some(dc_name) = &query.datacenter {
		ctx.config()
			.dc_for_name(dc_name)
			.ok_or_else(|| crate::errors::Datacenter::NotFound.build())?
			.datacenter_label
	} else {
		ctx.config().dc_label()
	};

	let actor_id = Id::new_v1(target_dc_label);

	let key: Option<String> = body.key;

	let res = ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key,
			runner_name_selector: body.runner_name_selector,
			input: body.input.clone(),
			crash_policy: body.crash_policy,
			// Forward requests to the correct api-peer datacenter
			forward_request: true,
			datacenter_name: query.datacenter.clone(),
		})
		.await?;

	let actor = res.actor;

	Ok(CreateResponse { actor })
}
