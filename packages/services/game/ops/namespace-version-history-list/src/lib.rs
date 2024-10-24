use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Version {
	namespace_id: Uuid,
	version_id: Uuid,
	deploy_ts: i64,
}

#[operation(name = "game-namespace-version-history-list")]
async fn handle(
	ctx: OperationContext<game::namespace_version_history_list::Request>,
) -> GlobalResult<game::namespace_version_history_list::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	// Fetch all members
	let versions = sql_fetch_all!(
		[ctx, Version]
		"
		SELECT namespace_id, version_id, deploy_ts
		FROM db_game.game_namespace_version_history
		WHERE namespace_id = ANY($1)
		ORDER BY deploy_ts DESC
		LIMIT $2
		",
		&namespace_ids,
		ctx.limit as i32,
	)
	.await?;

	// Group in to namespaces
	let namespaces = namespace_ids
		.iter()
		.map(
			|namespace_id| game::namespace_version_history_list::response::Namespace {
				namespace_id: Some((*namespace_id).into()),
				versions: versions
					.iter()
					.filter(|version| version.namespace_id == *namespace_id)
					.map(
						|version| game::namespace_version_history_list::response::Version {
							version_id: Some(version.version_id.into()),
							deploy_ts: version.deploy_ts,
						},
					)
					.collect::<Vec<_>>(),
			},
		)
		.collect::<Vec<_>>();

	Ok(game::namespace_version_history_list::Response { namespaces })
}
