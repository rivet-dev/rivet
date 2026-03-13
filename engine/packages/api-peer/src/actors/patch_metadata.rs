use anyhow::{Result, bail};
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::patch_metadata::*;

#[utoipa::path(
	patch,
	operation_id = "actors_patch_metadata",
	path = "/actors/{actor_id}/metadata",
	params(
		("actor_id" = Id, Path),
		PatchMetadataQuery,
	),
	request_body(content = PatchMetadataRequest, content_type = "application/json"),
	responses(
		(status = 200, body = PatchMetadataResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn patch_metadata(
	ctx: ApiCtx,
	path: PatchMetadataPath,
	query: PatchMetadataQuery,
	body: PatchMetadataRequest,
) -> Result<PatchMetadataResponse> {
	let patch = body
		.metadata
		.into_iter()
		.map(|(key, value)| pegboard::actor_metadata::PatchEntry { key, value })
		.collect::<Vec<_>>();
	pegboard::actor_metadata::validate_patch(&patch)?;

	let request_id = Uuid::new_v4().to_string();
	let mut patched_sub = ctx
		.subscribe::<pegboard::workflows::actor::MetadataPatched>((
			"request_id",
			request_id.clone(),
		))
		.await?;

	let (actors_res, namespace_res) = tokio::try_join!(
		ctx.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![path.actor_id],
			fetch_error: false,
			metadata: pegboard::actor_metadata::Projection::None,
		}),
		ctx.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace,
		}),
	)?;

	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;
	if actor.destroy_ts.is_some() {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	let namespace = namespace_res.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;
	if actor.namespace_id != namespace.namespace_id {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	let res = ctx
		.signal(pegboard::workflows::actor::PatchMetadata {
			patch,
			request_id: Some(request_id),
		})
		.to_workflow::<pegboard::workflows::actor::Workflow>()
		.tag("actor_id", path.actor_id)
		.graceful_not_found()
		.send()
		.await?;
	if res.is_none() {
		tracing::warn!(
			actor_id=?path.actor_id,
			"actor workflow not found while patching metadata"
		);
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	let patched = patched_sub.next().await?;
	if let Some(error) = &patched.error {
		if let Some(actor_error) = rebuild_actor_error(error) {
			return Err(actor_error);
		}

		bail!(
			"actor metadata patch failed: {}:{} {}",
			error.group,
			error.code,
			error.message
		);
	}

	Ok(PatchMetadataResponse {})
}

fn rebuild_actor_error(
	error: &pegboard::workflows::actor::SerializedError,
) -> Option<anyhow::Error> {
	if error.group != "actor" {
		return None;
	}

	match error.code.as_str() {
		"not_found" => Some(pegboard::errors::Actor::NotFound.build()),
		"metadata_patch_empty" => Some(pegboard::errors::Actor::MetadataPatchEmpty.build()),
		"metadata_key_invalid" => Some(
			pegboard::errors::Actor::MetadataKeyInvalid {
				key_preview: metadata_string_field(error.meta.as_ref(), "key_preview")?,
			}
			.build(),
		),
		"metadata_key_too_large" => Some(
			pegboard::errors::Actor::MetadataKeyTooLarge {
				max_size: metadata_usize_field(error.meta.as_ref(), "max_size")?,
				key_preview: metadata_string_field(error.meta.as_ref(), "key_preview")?,
			}
			.build(),
		),
		"metadata_value_too_large" => Some(
			pegboard::errors::Actor::MetadataValueTooLarge {
				max_size: metadata_usize_field(error.meta.as_ref(), "max_size")?,
				key_preview: metadata_string_field(error.meta.as_ref(), "key_preview")?,
			}
			.build(),
		),
		"metadata_too_large" => Some(
			pegboard::errors::Actor::MetadataTooLarge {
				max_size: metadata_usize_field(error.meta.as_ref(), "max_size")?,
			}
			.build(),
		),
		"metadata_too_many_keys" => Some(
			pegboard::errors::Actor::MetadataTooManyKeys {
				max: metadata_usize_field(error.meta.as_ref(), "max")?,
				count: metadata_usize_field(error.meta.as_ref(), "count")?,
			}
			.build(),
		),
		_ => None,
	}
}

fn metadata_string_field(meta: Option<&serde_json::Value>, field: &str) -> Option<String> {
	meta?.get(field)?.as_str().map(ToString::to_string)
}

fn metadata_usize_field(meta: Option<&serde_json::Value>, field: &str) -> Option<usize> {
	meta?
		.get(field)?
		.as_u64()
		.and_then(|value| usize::try_from(value).ok())
}
