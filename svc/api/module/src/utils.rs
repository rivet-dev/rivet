use api_helper::anchor::WatchResponse;
use rivet_api::models;
use rivet_operation::prelude::*;

pub async fn validate_config(ctx: &OperationContext<()>, namespace_id: Uuid) -> GlobalResult<()> {
	let namespaces_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let namespace = internal_unwrap_owned!(namespaces_res.namespaces.first());

	let version_id = internal_unwrap!(namespace.version_id);
	let config_res = op!([ctx] module_game_version_get {
		version_ids: vec![*version_id],
	})
	.await?;

	if config_res.versions.first().is_none() {
		panic_with!(
			API_FORBIDDEN,
			reason = "modulesfnot enabled for this namespace"
		);
	}

	Ok(())
}
