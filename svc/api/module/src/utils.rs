use rivet_operation::prelude::*;

pub async fn validate_config(ctx: &OperationContext<()>, namespace_id: Uuid) -> GlobalResult<()> {
	let namespaces_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let namespace = unwrap!(namespaces_res.namespaces.first());

	let version_id = unwrap_ref!(namespace.version_id);
	let config_res = op!([ctx] module_game_version_get {
		version_ids: vec![*version_id],
	})
	.await?;

	if config_res.versions.first().is_none() {
		bail_with!(
			API_FORBIDDEN,
			reason = "modulesfnot enabled for this namespace"
		);
	}

	Ok(())
}
