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
			assert_with!(
				!key.is_empty(),
				KV_KEY_VALIDATION_ERROR,
				index = i,
				error = "Too short"
			);
		}

		assert_with!(
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
	let namespace = internal_unwrap_owned!(namespaces_res.namespaces.first());

	let version_id = internal_unwrap!(namespace.version_id);
	let config_res = op!([ctx] kv_config_version_get {
		version_ids: vec![*version_id],
	})
	.await?;

	if config_res.versions.first().is_none() {
		panic_with!(
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
