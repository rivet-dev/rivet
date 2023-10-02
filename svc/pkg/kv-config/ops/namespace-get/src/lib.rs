use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameNamespace {
	namespace_id: Uuid,
}

#[operation(name = "kv-config-namespace-get")]
async fn handle(
	ctx: OperationContext<kv_config::namespace_get::Request>,
) -> GlobalResult<kv_config::namespace_get::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let sql_pool = ctx.crdb().await?;
	let namespaces = sqlx::query_as::<_, GameNamespace>(indoc!(
		"
			SELECT namespace_id
			FROM db_kv_config.game_namespaces
			WHERE namespace_id = ANY($1)
			"
	))
	.bind(&namespace_ids)
	.fetch_all(&sql_pool)
	.await?;

	let namespace_proto = namespaces
		.into_iter()
		.map(|ns| kv_config::namespace_get::response::Namespace {
			namespace_id: Some(ns.namespace_id.into()),
			config: Some(backend::kv::NamespaceConfig {}),
		})
		.collect::<Vec<_>>();

	Ok(kv_config::namespace_get::Response {
		namespaces: namespace_proto,
	})
}
