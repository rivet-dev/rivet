use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "server-list-for-game")]
async fn handle(
	ctx: OperationContext<dynamic_servers::server_list_for_game::Request>,
) -> GlobalResult<dynamic_servers::server_list_for_game::Response> {
	let env_id = unwrap_ref!(ctx.env_id).as_uuid();

	let server_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT
			server_id
		FROM
			db_dynamic_servers.servers
		WHERE
			env_id = $1
		AND
			tags @> $2
		",
		env_id,
		serde_json::to_value(&ctx.tags)?
	)
	.await?
	.into_iter()
	.map(|(id,)| common::Uuid::from(id))
	.collect::<Vec<_>>();

	Ok(dynamic_servers::server_list_for_game::Response { server_ids })
}
