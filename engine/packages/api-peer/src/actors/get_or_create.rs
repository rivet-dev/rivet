use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::get_or_create::{
	GetOrCreateQuery, GetOrCreateRequest, GetOrCreateResponse,
};
use rivet_error::RivetError;

#[tracing::instrument(skip_all)]
pub async fn get_or_create(
	ctx: ApiCtx,
	_path: (),
	query: GetOrCreateQuery,
	body: GetOrCreateRequest,
) -> Result<GetOrCreateResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Check if actor already exists for the key
	let existing = ctx
		.op(pegboard::ops::actor::get_for_key::Input {
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key: body.key.clone(),
			fetch_error: true,
		})
		.await?;

	if let Some(actor) = existing.actor {
		// Actor exists, return it
		return Ok(GetOrCreateResponse {
			actor,
			created: false,
		});
	}

	// Actor doesn't exist, create it
	let actor_id = Id::new_v1(ctx.config().dc_label());

	match ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: namespace.namespace_id,
			name: body.name.clone(),
			key: Some(body.key.clone()),
			runner_name_selector: body.runner_name_selector,
			input: body.input.clone(),
			crash_policy: body.crash_policy,
			// NOTE: This can forward if the user attempts to create an actor with a target dc and this dc
			// ends up forwarding to another.
			forward_request: true,
			// api-peer is always creating in its own datacenter
			datacenter_name: None,
		})
		.await
	{
		Ok(res) => Ok(GetOrCreateResponse {
			actor: res.actor,
			created: true,
		}),
		Err(err) => {
			// Check if this is a DuplicateKey error and extract the existing actor ID
			if let Some(existing_actor_id) = extract_duplicate_key_error(&err) {
				tracing::info!(
					?existing_actor_id,
					"received duplicate key error, fetching existing actor"
				);

				// Fetch the existing actor - it should be in this datacenter since
				// the duplicate key error came from this datacenter
				let res = ctx
					.op(pegboard::ops::actor::get::Input {
						actor_ids: vec![existing_actor_id],
						fetch_error: true,
					})
					.await?;

				let actor = res
					.actors
					.into_iter()
					.next()
					.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

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
