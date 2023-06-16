use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_cloud_server::models;
use rivet_convert::{ApiTryFrom, ApiTryInto};
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /games/{}/cdn/sites
pub async fn get_sites(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ListGameCdnSitesResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;

	let list_res = op!([ctx] cdn_site_list_for_game {
		game_id: Some(game_id.into()),
	})
	.await?;

	let sites_res = op!([ctx] cdn_site_get {
		site_ids: list_res.site_ids.clone(),
	})
	.await?;

	let uploads_res = op!([ctx] upload_get {
		upload_ids: sites_res
			.sites
			.iter()
			.flat_map(|site| site.upload_id)
			.collect::<Vec<_>>(),
	})
	.await?;

	// Convert the site data structures
	let mut sites = sites_res
		.sites
		.iter()
		.map(|site| {
			let upload = uploads_res
				.uploads
				.iter()
				.find(|u| u.upload_id == site.upload_id);
			if upload.is_none() {
				tracing::warn!("unable to find upload for site");
			}

			GlobalResult::Ok(models::CdnSiteSummary {
				site_id: internal_unwrap!(site.site_id).as_uuid().to_string(),
				upload_id: internal_unwrap!(site.upload_id).as_uuid().to_string(),
				display_name: site.display_name.clone(),
				create_ts: util::timestamp::to_chrono(site.create_ts)?,
				content_length: upload.map_or(0, |upload| upload.content_length) as i64,
				complete: upload.map_or(true, |upload| upload.complete_ts.is_some()),
			})
		})
		.collect::<Result<Vec<_>, _>>()?;

	// Sort by date desc
	sites.sort_by_key(|u| u.create_ts);
	sites.reverse();

	Ok(models::ListGameCdnSitesResponse { sites })
}

// MARK: POST /games/{}/versions/cdn/sites
pub async fn create_site(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::CreateGameCdnSiteRequest,
) -> GlobalResult<models::CreateGameCdnSiteResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;

	let create_res = op!([ctx] cdn_site_create {
		game_id: Some(game_id.into()),
		display_name: body.display_name,
		files: body
			.files
			.into_iter()
			.map(ApiTryInto::try_into)
			.collect::<GlobalResult<_>>()?,
	})
	.await?;

	Ok(models::CreateGameCdnSiteResponse {
		site_id: internal_unwrap!(create_res.site_id).as_uuid().to_string(),
		upload_id: internal_unwrap!(create_res.upload_id).as_uuid().to_string(),
		presigned_requests: create_res
			.presigned_requests
			.clone()
			.into_iter()
			.map(models::UploadPresignedRequest::try_from)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
