use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "identity-config-version-publish")]
async fn handle(
	ctx: OperationContext<identity_config::version_publish::Request>,
) -> GlobalResult<identity_config::version_publish::Response> {
	let version_id = internal_unwrap!(ctx.version_id).as_uuid();
	let config = internal_unwrap!(ctx.config);
	let _config_ctx = internal_unwrap!(ctx.config_ctx);

	sqlx::query("INSERT INTO db_identity_config.game_versions (version_id) VALUES ($1)")
		.bind(version_id)
		.execute(&ctx.crdb().await?)
		.await?;

	// TODO: Parallelize all futures in this for loop
	for custom_display_name in &config.custom_display_names {
		sqlx::query(indoc!(
			"
			INSERT INTO db_identity_config.custom_display_names
			(version_id, display_name)
			VALUES ($1, $2)
			"
		))
		.bind(version_id)
		.bind(&custom_display_name.display_name)
		.execute(&ctx.crdb().await?)
		.await?;
	}

	for custom_avatar in &config.custom_avatars {
		let upload_id = internal_unwrap!(custom_avatar.upload_id).as_uuid();
		sqlx::query(indoc!(
			"
			INSERT INTO db_identity_config.custom_avatars
			(version_id, upload_id)
			VALUES ($1, $2)
			"
		))
		.bind(version_id)
		.bind(upload_id)
		.execute(&ctx.crdb().await?)
		.await?;
	}

	Ok(identity_config::version_publish::Response {})
}
