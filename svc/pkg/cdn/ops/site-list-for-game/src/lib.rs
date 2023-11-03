use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cdn-site-list-for-game")]
async fn handle(
	ctx: OperationContext<cdn::site_list_for_game::Request>,
) -> GlobalResult<cdn::site_list_for_game::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let site_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT site_id
		FROM db_cdn.sites
		WHERE game_id = $1
		",
		game_id,
	)
	.await?
	.into_iter()
	.map(|(id,)| common::Uuid::from(id))
	.collect::<Vec<_>>();

	Ok(cdn::site_list_for_game::Response { site_ids })
}
