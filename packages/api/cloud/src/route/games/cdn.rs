use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /games/{}/cdn/sites
pub async fn get_sites(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::CloudGamesListGameCdnSitesResponse> {
	ctx.auth()
		.check_game_read_or_admin(ctx.op_ctx(), game_id)
		.await?;

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

	let mut sites = sites_res.sites.iter().collect::<Vec<_>>();

	// Sort by date desc
	sites.sort_by_key(|u| u.create_ts);
	sites.reverse();

	// Convert the site data structures
	let sites = sites
		.iter()
		.map(|site| {
			let upload = uploads_res
				.uploads
				.iter()
				.find(|u| u.upload_id == site.upload_id);
			if upload.is_none() {
				tracing::warn!("unable to find upload for site");
			}

			GlobalResult::Ok(models::CloudCdnSiteSummary {
				site_id: unwrap_ref!(site.site_id).as_uuid(),
				upload_id: unwrap_ref!(site.upload_id).as_uuid(),
				display_name: site.display_name.clone(),
				create_ts: util::timestamp::to_string(site.create_ts)?,
				content_length: upload
					.map_or(0, |upload| upload.content_length)
					.api_try_into()?,
				complete: upload.map_or(true, |upload| upload.complete_ts.is_some()),
			})
		})
		.collect::<Result<Vec<_>, _>>()?;

	Ok(models::CloudGamesListGameCdnSitesResponse { sites })
}

// MARK: POST /games/{}/versions/cdn/sites
pub async fn create_site(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::CloudGamesCreateGameCdnSiteRequest,
) -> GlobalResult<models::CloudGamesCreateGameCdnSiteResponse> {
	ctx.auth()
		.check_game_write_or_admin(ctx.op_ctx(), game_id)
		.await?;

	let create_res = op!([ctx] cdn_site_create {
		game_id: Some(game_id.into()),
		display_name: body.display_name,
		files: body
			.files
			.into_iter()
			.map(ApiTryInto::api_try_into)
			.collect::<GlobalResult<_>>()?,
	})
	.await?;

	Ok(models::CloudGamesCreateGameCdnSiteResponse {
		site_id: unwrap_ref!(create_res.site_id).as_uuid(),
		upload_id: unwrap_ref!(create_res.upload_id).as_uuid(),
		presigned_requests: create_res
			.presigned_requests
			.iter()
			.cloned()
			.map(ApiTryInto::api_try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
