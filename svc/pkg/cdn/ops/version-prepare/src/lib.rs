use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "cdn-version-prepare")]
async fn handle(
	ctx: OperationContext<cdn::version_prepare::Request>,
) -> GlobalResult<cdn::version_prepare::Response> {
	let game_id = internal_unwrap!(ctx.game_id);
	let config = internal_unwrap!(ctx.config);
	let site_id = internal_unwrap!(config.site_id);

	// Validate the site exists
	let site_res = op!([ctx] cdn_site_get {
		site_ids: vec![*site_id],
	})
	.await?;
	let site = site_res.sites.first();
	let site = internal_unwrap!(site, "site not found");
	let site_game_id = internal_unwrap!(site.game_id);
	let upload_id = internal_unwrap!(site.upload_id);
	internal_assert_eq!(game_id, site_game_id, "site does not belong to this game");

	// Validate version has completed uploading
	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![*upload_id],
	})
	.await?;
	let upload = upload_res.uploads.first();
	let upload = internal_unwrap!(upload);
	internal_assert!(upload.complete_ts.is_some(), "upload is not complete");

	Ok(cdn::version_prepare::Response {
		config_ctx: Some(backend::cdn::VersionConfigCtx {}),
	})
}
