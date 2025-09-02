use anyhow::Result;
use axum::http::{HeaderMap, Method};
use rivet_api_builder::ApiCtx;
use rivet_api_client::request_remote_datacenter;
use rivet_error::RivetError;
use rivet_types::actors::Actor;
use rivet_util::Id;
use std::collections::HashMap;

/// Helper function to fetch an actor by ID, automatically routing to the correct datacenter
/// based on the actor ID's label.
pub async fn fetch_actor_by_id(
	ctx: &ApiCtx,
	headers: HeaderMap,
	actor_id: Id,
	namespace: Option<String>,
) -> Result<Actor> {
	if actor_id.label() == ctx.config().dc_label() {
		// Local datacenter - use peer API directly
		let res = rivet_api_peer::actors::get::get(
			ctx.clone(),
			rivet_api_peer::actors::get::GetPath { actor_id },
			rivet_api_types::actors::get::GetQuery { namespace },
		)
		.await?;

		Ok(res.actor)
	} else {
		// Remote datacenter - make HTTP request
		let res = request_remote_datacenter::<rivet_api_types::actors::get::GetResponse>(
			ctx.config(),
			actor_id.label(),
			&format!("/actors/{}", actor_id),
			Method::GET,
			headers,
			Some(&rivet_api_types::actors::get::GetQuery { namespace }),
			Option::<&()>::None,
		)
		.await?;
		Ok(res.actor)
	}
}

/// Helper function to fetch multiple actors by their IDs, automatically routing to the correct datacenters
/// based on each actor ID's label. This function batches requests by datacenter for efficiency.
pub async fn fetch_actors_by_ids(
	ctx: &ApiCtx,
	headers: HeaderMap,
	actor_ids: Vec<Id>,
	namespace: String,
	include_destroyed: Option<bool>,
	limit: Option<usize>,
) -> Result<Vec<Actor>> {
	if actor_ids.is_empty() {
		return Ok(Vec::new());
	}

	// Group actor IDs by datacenter
	let mut actors_by_dc = HashMap::<u16, Vec<Id>>::new();
	for actor_id in actor_ids {
		actors_by_dc
			.entry(actor_id.label())
			.or_default()
			.push(actor_id);
	}

	// Fetch actors in batch from each datacenter
	let fetch_futures = actors_by_dc.into_iter().map(|(dc_label, dc_actor_ids)| {
		let ctx = ctx.clone();
		let headers = headers.clone();
		let namespace = namespace.clone();
		let include_destroyed = include_destroyed;
		let limit = limit;

		async move {
			// Convert actor IDs to comma-separated string
			let actor_ids_str = dc_actor_ids
				.iter()
				.map(|id| id.to_string())
				.collect::<Vec<_>>()
				.join(",");

			// Prepare peer query with actor_ids
			let peer_query = rivet_api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_ids: Some(actor_ids_str),
				include_destroyed,
				limit,
				cursor: None,
			};

			if dc_label == ctx.config().dc_label() {
				// Local datacenter - use peer API directly
				let res = rivet_api_peer::actors::list::list(ctx, (), peer_query).await?;
				Ok::<Vec<Actor>, anyhow::Error>(res.actors)
			} else {
				// Remote datacenter - make HTTP request
				let res = request_remote_datacenter::<rivet_api_types::actors::list::ListResponse>(
					ctx.config(),
					dc_label,
					"/actors",
					Method::GET,
					headers,
					Some(&peer_query),
					Option::<&()>::None,
				)
				.await?;
				Ok(res.actors)
			}
		}
	});

	// Execute all requests in parallel
	let results = futures_util::future::join_all(fetch_futures).await;

	// Aggregate results
	let mut actors = Vec::new();
	for res in results {
		match res {
			Ok(dc_actors) => actors.extend(dc_actors),
			Err(err) => tracing::error!(?err, "failed to fetch actors from datacenter"),
		}
	}

	// Sort by create ts desc
	actors.sort_by_cached_key(|x| std::cmp::Reverse(x.create_ts));

	Ok(actors)
}

/// Helper function to extract the existing actor ID from a duplicate key error
///
/// Returns Some(actor_id) if the error is a duplicate key error with metadata, None otherwise
pub fn extract_duplicate_key_error(err: &anyhow::Error) -> Option<Id> {
	// Try to downcast to RivetError first (local calls)
	let rivet_err = err.chain().find_map(|x| x.downcast_ref::<RivetError>());
	if let Some(rivet_err) = rivet_err {
		tracing::info!(group = ?rivet_err.group(), code = ?rivet_err.code(), "normal rivet error");
		if rivet_err.group() == "actor" && rivet_err.code() == "duplicate_key" {
			// Extract existing_actor_id from metadata
			if let Some(metadata) = rivet_err.metadata() {
				if let Some(actor_id_str) =
					metadata.get("existing_actor_id").and_then(|v| v.as_str())
				{
					if let Ok(actor_id) = actor_id_str.parse::<Id>() {
						return Some(actor_id);
					}
				}
			}
		}
	}

	// Try to downcast to RawErrorResponse (for remote API calls)
	let raw_err = err
		.chain()
		.find_map(|x| x.downcast_ref::<rivet_api_builder::error_response::RawErrorResponse>());
	if let Some(raw_err) = raw_err {
		tracing::info!(group = ?raw_err.1.group, code = ?raw_err.1.code, "raw rivet error");
		if raw_err.1.group == "actor" && raw_err.1.code == "duplicate_key" {
			// Extract existing_actor_id from metadata (now available in ErrorResponse)
			if let Some(metadata) = &raw_err.1.metadata {
				if let Some(actor_id_str) =
					metadata.get("existing_actor_id").and_then(|v| v.as_str())
				{
					if let Ok(actor_id) = actor_id_str.parse::<Id>() {
						return Some(actor_id);
					}
				}
			}
		}
	}

	None
}
