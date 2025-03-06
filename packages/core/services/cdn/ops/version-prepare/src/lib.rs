use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "cdn-version-prepare")]
async fn handle(
	ctx: OperationContext<cdn::version_prepare::Request>,
) -> GlobalResult<cdn::version_prepare::Response> {
	let game_id = unwrap_ref!(ctx.game_id);
	let config = unwrap_ref!(ctx.config);
	let site_id = unwrap_ref!(config.site_id);

	// Validate the site exists
	let site_res = op!([ctx] cdn_site_get {
		site_ids: vec![*site_id],
	})
	.await?;
	let site = site_res.sites.first();
	let site = unwrap_ref!(site, "site not found");
	let site_game_id = unwrap_ref!(site.game_id);
	let upload_id = unwrap_ref!(site.upload_id);
	ensure_eq!(game_id, site_game_id, "site does not belong to this game");

	// Validate version has completed uploading
	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![*upload_id],
	})
	.await?;
	let upload = upload_res.uploads.first();
	let upload = unwrap_ref!(upload);
	ensure!(upload.complete_ts.is_some(), "upload is not complete");

	Ok(cdn::version_prepare::Response {
		config_ctx: Some(backend::cdn::VersionConfigCtx {}),
	})
}
