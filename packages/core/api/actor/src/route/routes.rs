use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_operation::prelude::*;
use route::{ops::delete, ops::get, ops::list_for_env, ops::upsert};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use util::timestamp;

use crate::auth::{Auth, CheckOpts, CheckOutput};

use super::GlobalQuery;

// MARK: GET /routes
#[derive(Debug, Clone, Deserialize)]
pub struct ListQuery {
	#[serde(flatten)]
	global: GlobalQuery,
}

pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::RoutesListRoutesResponse> {
	let CheckOutput {
		env_id: namespace_id,
		..
	} = ctx.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	// Call route service to list routes for the environment
	let list_res = ctx.op(list_for_env::Input { namespace_id }).await?;

	// Fetch route details for all routes
	let routes_res = ctx
		.op(get::Input {
			route_ids: list_res.route_ids.clone(),
		})
		.await?;

	// Convert the routes to API models
	let routes = routes_res
		.routes
		.iter()
		.map(|route| {
			// Get the route target (which will be an Actors target)
			let target = match &route.target {
				route::types::RouteTarget::Actors { selector_tags } => models::RoutesRouteTarget {
					actors: Some(Box::new(models::RoutesRouteTargetActors {
						selector_tags: selector_tags.clone(),
					})),
				},
			};

			GlobalResult::Ok(models::RoutesRoute {
				id: route.name_id.clone(),
				created_at: timestamp::to_string(route.create_ts)?,
				updated_at: timestamp::to_string(route.update_ts)?,
				hostname: route.hostname.clone(),
				path: route.path.clone(),
				route_subpaths: route.route_subpaths,
				strip_prefix: route.strip_prefix,
				target: Box::new(target),
			})
		})
		.collect::<Result<Vec<_>, _>>()?;

	Ok(models::RoutesListRoutesResponse { routes })
}

// MARK: PUT /routes/{name_id}
pub async fn update(
	ctx: Ctx<Auth>,
	name_id: String,
	body: models::RoutesUpdateRouteBody,
	query: GlobalQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput {
		env_id: namespace_id,
		..
	} = ctx.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	// Extract actors selector tags from request
	let actors_selector_tags = body
		.target
		.actors
		.as_ref()
		.ok_or_else(|| {
			err_code!(
				ROUTE_INVALID_TARGET,
				msg = "actors target configuration is required"
			)
		})?
		.selector_tags
		.clone();

	// Call the upsert operation
	let _res = ctx
		.op(upsert::Input {
			namespace_id,
			name_id,
			hostname: body.hostname.clone(),
			path: body.path.clone(),
			route_subpaths: body.route_subpaths,
			strip_prefix: body.strip_prefix,
			actors_selector_tags,
		})
		.await?;

	Ok(serde_json::json!({}))
}

// MARK: DELETE /routes/{name_id}
pub async fn delete(
	ctx: Ctx<Auth>,
	name_id: String,
	query: GlobalQuery,
) -> GlobalResult<serde_json::Value> {
	let CheckOutput {
		env_id: namespace_id,
		..
	} = ctx.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	// Validate input
	ensure!(!name_id.is_empty(), "name_id cannot be empty");

	// Soft delete the route by name_id
	ctx.op(delete::Input {
		namespace_id,
		name_id,
	})
	.await?;

	Ok(json!({}))
}

// MARK: GET /routes/history
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryQuery {
	#[serde(flatten)]
	global: GlobalQuery,
	start: i64,
	end: i64,
	interval: i64,
	query_json: Option<String>,
	group_by: Option<String>, // JSON-encoded KeyPath
}

pub async fn history(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: HistoryQuery,
) -> GlobalResult<models::RoutesHistoryResponse> {
	let CheckOutput { .. } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: true,
				opt_auth: false,
			},
		)
		.await?;

	// Get server namespace
	let namespace = ctx.config().server()?.rivet.namespace.clone();

	// Parse user query if provided
	let user_query_expr = if let Some(query_json) = &query.query_json {
		match serde_json::from_str::<clickhouse_user_query::QueryExpr>(&query_json) {
			Ok(expr) => Some(expr),
			Err(e) => {
				bail_with!(API_BAD_QUERY, error = e.to_string());
			}
		}
	} else {
		None
	};

	// Parse group by key path if provided
	let group_by = if let Some(group_by_json) = &query.group_by {
		match serde_json::from_str::<clickhouse_user_query::KeyPath>(&group_by_json) {
			Ok(key_path) => Some(key_path),
			Err(e) => {
				bail_with!(API_BAD_QUERY, error = format!("Invalid group_by: {}", e));
			}
		}
	} else {
		None
	};

	// Call the guard operation to get history data
	let history_res = ctx
		.op(core_guard::ops::routes_history::Input {
			namespace,
			user_query_expr,
			group_by,
			start_ms: query.start,
			end_ms: query.end,
			interval_ms: query.interval,
		})
		.await?;

	Ok(models::RoutesHistoryResponse {
		metric_names: history_res.metric_names,
		metric_attributes: history_res
			.metric_attributes
			.into_iter()
			.map(|attrs| attrs.into_iter().collect::<HashMap<_, _>>())
			.collect(),
		metric_types: history_res.metric_types,
		metric_values: history_res.metric_values,
	})
}
