use anyhow::Result;
use axum::{
	extract::{Extension, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_types::actors::CrashPolicy;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::actors::utils;
use crate::errors;

#[derive(Debug, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct GetOrCreateQuery {
	pub namespace: String,
	pub datacenter: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ActorsGetOrCreateRequest)]
pub struct GetOrCreateRequest {
	pub name: String,
	pub key: String,
	pub input: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsGetOrCreateResponse)]
pub struct GetOrCreateResponse {
	pub actor: rivet_types::actors::Actor,
	pub created: bool,
}

/// ## Datacenter Round Trips
///
/// **If actor exists**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - GET /actors/{}
///
/// **If actor does not exist and is created in the current datacenter:**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)
///
/// **If actor does not exist and is created in a different datacenter:**
///
/// 3 round trips:
/// - namespace::ops::resolve_for_name_global
/// - POST /actors to remote datacenter
/// - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)
///
/// actor::get will always be in the same datacenter.
///
/// ## Optimized Alternative Routes
///
/// For minimal round trips to get or create an actor, use `PUT /actors/by-id`. This doesn't
/// require fetching the actor's state from the other datacenter.
#[utoipa::path(
    put,
	operation_id = "actors_get_or_create",
    path = "/actors",
    params(GetOrCreateQuery),
    request_body(content = GetOrCreateRequest, content_type = "application/json"),
    responses(
        (status = 200, body = GetOrCreateResponse),
    ),
)]
pub async fn get_or_create(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Query(query): Query<GetOrCreateQuery>,
	Json(body): Json<GetOrCreateRequest>,
) -> Response {
	match get_or_create_inner(ctx, headers, query, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn get_or_create_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	query: GetOrCreateQuery,
	body: GetOrCreateRequest,
) -> Result<GetOrCreateResponse> {
	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Check if actor already exists for the key
	// The get_for_key op uses global consistency and handles datacenter routing
	let existing = ctx
		.op(pegboard::ops::actor::get_for_key::Input {
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key: body.key.clone(),
		})
		.await?;

	if let Some(actor) = existing.actor {
		// Actor exists, return it
		return Ok(GetOrCreateResponse {
			actor,
			created: false,
		});
	}

	// Actor doesn't exist for any key, create it
	// Determine which datacenter to create the actor in
	let target_dc_label = if let Some(dc_name) = &query.datacenter {
		ctx.config()
			.dc_for_name(dc_name)
			.ok_or_else(|| errors::Datacenter::NotFound.build())?
			.datacenter_label
	} else {
		ctx.config().dc_label()
	};

	let actor_id = Id::new_v1(target_dc_label);

	match ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key: Some(body.key.clone()),
			runner_name_selector: body.runner_name_selector,
			input: body.input.clone(),
			crash_policy: body.crash_policy,
			forward_request: true,
			datacenter_name: query.datacenter.clone(),
		})
		.await
	{
		Ok(res) => Ok(GetOrCreateResponse {
			actor: res.actor,
			created: true,
		}),
		Err(err) => {
			// Check if this is a DuplicateKey error and extract the existing actor ID
			if let Some(existing_actor_id) = utils::extract_duplicate_key_error(&err) {
				tracing::info!(
					?existing_actor_id,
					"received duplicate key error, returning existing actor id"
				);
				let actor = utils::fetch_actor_by_id(
					&ctx,
					headers.clone(),
					existing_actor_id,
					Some(query.namespace.clone()),
				)
				.await?;
				return Ok(GetOrCreateResponse {
					actor,
					created: false,
				});
			}

			// Re-throw the original error if it's not a DuplicateKey
			Err(err)
		}
	}
}
