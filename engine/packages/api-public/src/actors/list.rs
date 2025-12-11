use anyhow::{Context, Result};
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Query},
};
use rivet_api_types::{actors::list::*, pagination::Pagination};
use rivet_api_util::fanout_to_datacenters;

use crate::{actors::utils::fetch_actors_by_ids, ctx::ApiCtx, errors};

/// ## Datacenter Round Trips
///
/// **If key is some & `include_destroyed` is false**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - GET /actors (multiple DCs based on actor IDs)
///
///	This path is optimized because we can read the actor IDs fro the key directly from Epoxy with
///	stale consistency to determine which datacenter the actor lives in. Under most circumstances,
///	this means we don't need to fan out to all datacenters (like normal list does).
///
///	The reason `include_destroyed` has to be false is Epoxy only stores currently active actors. If
///	`include_destroyed` is true, we show all previous iterations of actors with the same key.
///
/// **Otherwise**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - GET /actors (fanout)
///
/// ## Optimized Alternative Routes
#[utoipa::path(
	get,
	operation_id = "actors_list",
	path = "/actors",
	params(ListQuery),
	responses(
		(status = 200, body = ListResponse),
	),
)]
pub async fn list(Extension(ctx): Extension<ApiCtx>, Query(query): Query<ListQuery>) -> Response {
	match list_inner(ctx, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(ctx: ApiCtx, query: ListQuery) -> Result<ListResponse> {
	ctx.auth().await?;

	// Parse query
	let actor_ids = [
		query.actor_id.clone(),
		query
			.actor_ids
			.as_ref()
			.map(|x| {
				x.split(',')
					.filter_map(|s| s.trim().parse::<rivet_util::Id>().ok())
					.collect::<Vec<_>>()
			})
			.unwrap_or_default(),
	]
	.concat();
	let include_destroyed = query.include_destroyed.unwrap_or(false);

	// Validate exclusive input: either (name + key) or actor_ids
	if !actor_ids.is_empty() && (query.name.is_some() || query.key.is_some()) {
		return Err(errors::Validation::InvalidInput {
			message: "Cannot provide both actor_id and (name + key). Use either actor_id or (name + key).".to_string(),
		}
		.build());
	}

	// Validate key
	if query.key.is_some() && query.name.is_none() {
		return Err(errors::Validation::InvalidInput {
			message: "Name is required when key is provided.".to_string(),
		}
		.build());
	}

	if !actor_ids.is_empty() {
		// Cap actor_ids count to 32
		if actor_ids.len() > 32 {
			return Err(errors::Validation::TooManyActorIds {
				max: 32,
				count: actor_ids.len(),
			}
			.build());
		}

		// Resolve namespace once to verify actors belong to it (namespace validation is done in api-peer)
		ctx.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		// Fetch actors
		let mut actors = fetch_actors_by_ids(
			&ctx,
			actor_ids,
			query.namespace.clone(),
			query.include_destroyed,
			None, // Don't apply limit in fetch, we'll apply it after cursor filtering
		)
		.await?;

		// Apply cursor filtering if provided
		if let Some(cursor_str) = &query.cursor {
			let cursor_ts: i64 = cursor_str.parse().context("invalid cursor format")?;
			actors.retain(|actor| actor.create_ts < cursor_ts);
		}

		// Apply limit after cursor filtering
		let limit = query.limit.unwrap_or(100);
		actors.truncate(limit);

		let cursor = actors.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			actors,
			pagination: Pagination { cursor },
		})
	} else if let Some(key) = &query.key
		&& !include_destroyed
		&& query.name.is_some()
	{
		// Existing path: fetch actors by key (when name is provided and not include_destroyed)
		// Resolve namespace once
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: query.namespace.clone(),
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		let name = query.name.as_ref().context("unreachable")?;
		let actor_id = ctx
			.op(pegboard::ops::actor::get_for_key::Input {
				namespace_id: namespace.namespace_id,
				name: name.clone(),
				key: key.clone(),
			})
			.await?
			.actor
			.map(|x| x.actor_id);

		// If no actors found, return empty result
		let Some(actor_id) = actor_id else {
			return Ok(ListResponse {
				actors: Vec::new(),
				pagination: Pagination { cursor: None },
			});
		};

		// Fetch actors
		let actors = fetch_actors_by_ids(
			&ctx,
			vec![actor_id],
			query.namespace.clone(),
			query.include_destroyed,
			query.limit,
		)
		.await?;

		let cursor = actors.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			actors,
			pagination: Pagination { cursor },
		})
	} else {
		// Fanout path: used when include_destroyed is true or when no key is provided
		// Require name for fanout operations
		if query.name.is_none() {
			return Err(errors::Validation::InvalidInput {
				message: "Name is required when not using actor_ids.".to_string(),
			}
			.build());
		}

		let limit = query.limit.unwrap_or(100);

		// Fanout to all datacenters
		let mut actors =
			fanout_to_datacenters::<ListResponse, _, _, _, _, Vec<rivet_types::actors::Actor>>(
				ctx.into(),
				"/actors",
				query,
				|ctx, query| async move { rivet_api_peer::actors::list::list(ctx, (), query).await },
				|_, res, agg| agg.extend(res.actors),
			)
			.await?;

		// Sort by create ts desc
		actors.sort_by_cached_key(|x| std::cmp::Reverse(x.create_ts));

		// Shorten array since returning all actors from all regions could end up returning `regions *
		// limit` results, which is a lot.
		actors.truncate(limit);

		let cursor = actors.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			actors,
			pagination: Pagination { cursor },
		})
	}
}
