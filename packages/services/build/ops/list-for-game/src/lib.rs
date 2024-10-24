use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "build-list-for-game")]
async fn handle(
	ctx: OperationContext<build::list_for_game::Request>,
) -> GlobalResult<build::list_for_game::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let build_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT
			build_id
		FROM
			db_build.builds
		WHERE
			game_id = $1
		AND
			tags @> $2
		",
		game_id,
		serde_json::to_value(&ctx.tags)?
	)
	.await?
	.into_iter()
	.map(|(id,)| common::Uuid::from(id))
	.collect::<Vec<_>>();

	Ok(build::list_for_game::Response { build_ids })
}
