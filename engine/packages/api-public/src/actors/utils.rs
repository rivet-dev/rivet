use anyhow::Result;
use axum::http::Method;
use rivet_api_builder::ApiCtx;
use rivet_api_util::request_remote_datacenter;
use rivet_error::RivetError;
use rivet_types::actors::Actor;
use rivet_util::Id;
use std::collections::HashMap;

/// Helper function to fetch an actor by ID, automatically routing to the correct datacenter
/// based on the actor ID's label.
#[tracing::instrument(skip_all)]
pub async fn fetch_actor_by_id(ctx: &ApiCtx, actor_id: Id, namespace: String) -> Result<Actor> {
	let list_query = rivet_api_types::actors::list::ListQuery {
		namespace,
		actor_ids: Some(actor_id.to_string()),
		..Default::default()
	};

	if actor_id.label() == ctx.config().dc_label() {
		// Local datacenter - use peer API directly
		let res = rivet_api_peer::actors::list::list(ctx.clone().into(), (), list_query).await?;
		let actor = res
			.actors
			.into_iter()
			.next()
			.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

		Ok(actor)
	} else {
		// Remote datacenter - make HTTP request
		let res = request_remote_datacenter::<rivet_api_types::actors::list::ListResponse>(
			ctx.config(),
			actor_id.label(),
			"/actors",
			Method::GET,
			Some(&list_query),
			Option::<&()>::None,
		)
		.await?;
		let actor = res
			.actors
			.into_iter()
			.next()
			.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

		Ok(actor)
	}
}

/// Helper function to fetch multiple actors by their IDs, automatically routing to the correct datacenters
/// based on each actor ID's label. This function batches requests by datacenter for efficiency.
#[tracing::instrument(skip_all)]
pub async fn fetch_actors_by_ids(
	ctx: &ApiCtx,
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
		let namespace = namespace.clone();
		let include_destroyed = include_destroyed;
		let limit = limit;

		async move {
			// Prepare peer query with actor_ids
			let peer_query = rivet_api_types::actors::list::ListQuery {
				namespace: namespace.clone(),
				name: None,
				key: None,
				actor_ids: None,
				actor_id: dc_actor_ids,
				include_destroyed,
				limit,
				cursor: None,
			};

			if dc_label == ctx.config().dc_label() {
				// Local datacenter - use peer API directly
				let res = rivet_api_peer::actors::list::list(ctx.into(), (), peer_query).await?;
				Ok::<Vec<Actor>, anyhow::Error>(res.actors)
			} else {
				// Remote datacenter - make HTTP request
				let res = request_remote_datacenter::<rivet_api_types::actors::list::ListResponse>(
					ctx.config(),
					dc_label,
					"/actors",
					Method::GET,
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

/// Determine the datacenter label to create the actor in.
#[tracing::instrument(skip_all)]
pub async fn find_dc_for_actor_creation(
	ctx: &ApiCtx,
	namespace_id: Id,
	namespace_name: &str,
	runner_name: &str,
	dc_name: Option<&str>,
) -> Result<u16> {
	let target_dc_label = if let Some(dc_name) = &dc_name {
		// Use user-configured DC
		ctx.config()
			.dc_for_name(dc_name)
			.ok_or_else(|| crate::errors::Datacenter::NotFound.build())?
			.datacenter_label
	} else {
		// Find the nearest DC with runners
		let res = ctx
			.op(pegboard::ops::runner::find_dc_with_runner::Input {
				namespace_id,
				runner_name: runner_name.into(),
			})
			.await?;
		if let Some(dc_label) = res.dc_label {
			dc_label
		} else {
			return Err(pegboard::errors::Actor::NoRunnersAvailable {
				namespace: namespace_name.into(),
				runner_name: runner_name.into(),
			}
			.build());
		}
	};

	Ok(target_dc_label)
}
