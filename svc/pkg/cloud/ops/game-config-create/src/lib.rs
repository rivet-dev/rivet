use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cloud-game-config-create")]
async fn handle(
	ctx: OperationContext<cloud::game_config_create::Request>,
) -> GlobalResult<cloud::game_config_create::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	sql_query!(
		[ctx]
		"INSERT INTO db_cloud.game_configs (game_id) VALUES ($1)",
		game_id,
	)
	.await?;

	Ok(cloud::game_config_create::Response {})
}
