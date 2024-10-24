use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-config-game-upsert")]
pub async fn handle(
	ctx: OperationContext<mm_config::game_upsert::Request>,
) -> GlobalResult<mm_config::game_upsert::Response> {
	let game_id = unwrap!(ctx.game_id).as_uuid();
	let config = unwrap_ref!(ctx.config);

	sql_execute!(
		[ctx]
		"
		UPSERT INTO db_mm_config.games (game_id, host_networking_enabled, root_user_enabled)
		VALUES ($1, $2, $3)
		",
		game_id,
		config.host_networking_enabled,
		config.root_user_enabled,
	)
	.await?;

	Ok(mm_config::game_upsert::Response {})
}
