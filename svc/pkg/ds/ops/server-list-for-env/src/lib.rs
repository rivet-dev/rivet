use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "server-list-for-env")]
async fn handle(
	ctx: OperationContext<dynamic_servers::server_list_for_env::Request>,
) -> GlobalResult<dynamic_servers::server_list_for_env::Response> {
	let env_id = unwrap_ref!(ctx.env_id).as_uuid();
	let cursor = ctx.cursor.map(|x| x.as_uuid());

	let server_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		WITH after_server AS (
			SELECT create_ts, server_id
			FROM db_ds.servers
			WHERE server_id = $4
		)
		SELECT server_id
		FROM db_ds.servers
		WHERE
			env_id = $1
			AND tags @> $2
			AND ($3 OR destroy_ts IS NOT NULL)
			AND (
				$4 IS NULL
				OR (create_ts, server_id) < (SELECT create_ts, server_id FROM after_server)
			)
		ORDER BY create_ts DESC, server_id DESC
		LIMIT 64
		",
		env_id,
		serde_json::to_value(&ctx.tags)?,
		ctx.include_destroyed,
		cursor,
	)
	.await?
	.into_iter()
	.map(|(id,)| common::Uuid::from(id))
	.collect::<Vec<_>>();

	Ok(dynamic_servers::server_list_for_env::Response { server_ids })
}
