use api_helper::anchor::WatchResponse;
use rivet_api::models;
use rivet_operation::prelude::*;

pub enum SlashConfig {
	NeedsTrailingSlash,
	NeedsNoTrailingSlash,
	None,
}

/// Validates a list of keys
pub fn validate_keys(keys: &[impl AsRef<str>], directory: bool) -> GlobalResult<()> {
	for (i, key) in keys.iter().enumerate() {
		let key = key.as_ref();

		if !directory {
			ensure_with!(
				!key.is_empty(),
				KV_KEY_VALIDATION_ERROR,
				index = i,
				error = "Too short"
			);
		}

		ensure_with!(
			key.len() < 512,
			KV_KEY_VALIDATION_ERROR,
			index = i,
			error = "Too long (> 512 chars)",
		);
	}

	Ok(())
}

pub async fn validate_config(
	ctx: &OperationContext<()>,
	namespace_id: common::Uuid,
) -> GlobalResult<()> {
	let namespaces_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id],
	})
	.await?;
	let namespace = unwrap!(namespaces_res.namespaces.first());

	let version_id = unwrap_ref!(namespace.version_id);
	let config_res = op!([ctx] kv_config_version_get {
		version_ids: vec![*version_id],
	})
	.await?;

	if config_res.versions.first().is_none() {
		bail_with!(
			API_FORBIDDEN,
			reason = "KV service not enabled for this namespace"
		);
	}

	Ok(())
}

pub fn watch_response(value: WatchResponse) -> models::WatchResponse {
	models::WatchResponse {
		index: value.watch_index().to_owned(),
	}
}
