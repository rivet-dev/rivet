use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "build-list-for-game")]
async fn handle(
	ctx: OperationContext<build::list_for_game::Request>,
) -> GlobalResult<build::list_for_game::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let build_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT build_id
		FROM db_build.builds
		WHERE game_id = $1
		"
	))
	.bind(game_id)
	.fetch_all(&ctx.crdb().await?)
	.await?
	.into_iter()
	.map(|(id,)| common::Uuid::from(id))
	.collect::<Vec<_>>();

	Ok(build::list_for_game::Response { build_ids })
}
