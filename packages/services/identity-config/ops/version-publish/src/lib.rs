use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "identity-config-version-publish")]
async fn handle(
	ctx: OperationContext<identity_config::version_publish::Request>,
) -> GlobalResult<identity_config::version_publish::Response> {
	let version_id = unwrap_ref!(ctx.version_id).as_uuid();
	let config = unwrap_ref!(ctx.config);
	let _config_ctx = unwrap_ref!(ctx.config_ctx);

	sql_execute!(
		[ctx]
		"INSERT INTO db_identity_config.game_versions (version_id) VALUES ($1)",
		version_id,
	)
	.await?;

	// TODO: Parallelize all futures in this for loop
	for custom_display_name in &config.custom_display_names {
		sql_execute!(
			[ctx]
			"
			INSERT INTO db_identity_config.custom_display_names
			(version_id, display_name)
			VALUES ($1, $2)
			",
			version_id,
			&custom_display_name.display_name,
		)
		.await?;
	}

	for custom_avatar in &config.custom_avatars {
		let upload_id = unwrap_ref!(custom_avatar.upload_id).as_uuid();
		sql_execute!(
			[ctx]
			"
			INSERT INTO db_identity_config.custom_avatars
			(version_id, upload_id)
			VALUES ($1, $2)
			",
			version_id,
			upload_id,
		)
		.await?;
	}

	Ok(identity_config::version_publish::Response {})
}
