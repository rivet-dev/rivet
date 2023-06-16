use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cloud-game-config-create")]
async fn handle(
	ctx: OperationContext<cloud::game_config_create::Request>,
) -> GlobalResult<cloud::game_config_create::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();

	sqlx::query("INSERT INTO game_configs (game_id) VALUES ($1)")
		.bind(game_id)
		.execute(&ctx.crdb("db-cloud").await?)
		.await?;

	Ok(cloud::game_config_create::Response {})
}
