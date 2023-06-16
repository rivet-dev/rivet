use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct SiteRow {
	site_id: Uuid,
	game_id: Uuid,
	upload_id: Uuid,
	display_name: String,
	create_ts: i64,
}

#[operation(name = "cdn-site-get")]
async fn handle(
	ctx: OperationContext<cdn::site_get::Request>,
) -> GlobalResult<cdn::site_get::Response> {
	let site_ids = ctx
		.site_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let sites = sqlx::query_as::<_, SiteRow>(indoc!(
		"
		SELECT site_id, game_id, upload_id, display_name, create_ts
		FROM sites
		WHERE site_id = ANY($1)
		"
	))
	.bind(site_ids)
	.fetch_all(&ctx.crdb("db-cdn").await?)
	.await?
	.into_iter()
	.map(|site| cdn::site_get::response::Site {
		site_id: Some(site.site_id.into()),
		game_id: Some(site.game_id.into()),
		upload_id: Some(site.upload_id.into()),
		display_name: site.display_name.clone(),
		create_ts: site.create_ts,
	})
	.collect::<Vec<_>>();

	Ok(cdn::site_get::Response { sites })
}
