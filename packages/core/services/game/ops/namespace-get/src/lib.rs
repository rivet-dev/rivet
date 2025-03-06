use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Namespace {
	namespace_id: Uuid,
	game_id: Uuid,
	create_ts: i64,
	display_name: String,
	version_id: Uuid,
	name_id: String,
}

impl From<Namespace> for backend::game::Namespace {
	fn from(value: Namespace) -> Self {
		backend::game::Namespace {
			namespace_id: Some(value.namespace_id.into()),
			game_id: Some(value.game_id.into()),
			create_ts: value.create_ts,
			display_name: value.display_name,
			version_id: Some(value.version_id.into()),
			name_id: value.name_id,
		}
	}
}

#[operation(name = "game-namespace-get")]
async fn handle(
	ctx: OperationContext<game::namespace_get::Request>,
) -> GlobalResult<game::namespace_get::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let namespaces = ctx
		.cache()
		.immutable()
		.fetch_all_proto("namespace", namespace_ids, |mut cache, namespace_ids| {
			let ctx = ctx.base();
			async move {
				let namespaces = sql_fetch_all!(
					[ctx, Namespace]
					"
					SELECT namespace_id, game_id, create_ts, display_name, version_id, name_id
					FROM db_game.game_namespaces
					WHERE namespace_id = ANY($1)
					ORDER BY display_name
					",
					&namespace_ids,
				)
				.await?;

				for namespace in namespaces {
					cache.resolve(&namespace.namespace_id.clone(), namespace.into());
				}

				Ok(cache)
			}
		})
		.await?;

	Ok(game::namespace_get::Response { namespaces })
}
