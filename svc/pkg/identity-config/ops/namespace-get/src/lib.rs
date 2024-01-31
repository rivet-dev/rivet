use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameNamespace {
	namespace_id: Uuid,
}

#[operation(name = "identity-config-namespace-get")]
async fn handle(
	ctx: OperationContext<identity_config::namespace_get::Request>,
) -> GlobalResult<identity_config::namespace_get::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let _sql_pool = ctx.crdb().await?;
	let namespaces = sql_fetch_all!(
		[ctx, GameNamespace]
		"
			SELECT namespace_id
			FROM db_identity_config.game_namespaces
			WHERE namespace_id = ANY($1)
			",
		&namespace_ids,
	)
	.await?;

	let namespace_proto = namespaces
		.into_iter()
		.map(|ns| identity_config::namespace_get::response::Namespace {
			namespace_id: Some(ns.namespace_id.into()),
			config: Some(backend::identity::NamespaceConfig {}),
		})
		.collect::<Vec<_>>();

	Ok(identity_config::namespace_get::Response {
		namespaces: namespace_proto,
	})
}
