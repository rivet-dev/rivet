use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameNamespace {
	namespace_id: Uuid,
	lobby_count_max: i64,
	max_players_per_client: i64,
	max_players_per_client_vpn: i64,
	max_players_per_client_proxy: i64,
	max_players_per_client_tor: i64,
	max_players_per_client_hosting: i64,
}

#[operation(name = "mm-config-namespace-get")]
async fn handle(
	ctx: OperationContext<mm_config::namespace_get::Request>,
) -> GlobalResult<mm_config::namespace_get::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let namespaces = sqlx::query_as::<_, GameNamespace>(indoc!(
		"
		SELECT
			namespace_id,
			lobby_count_max,
			max_players_per_client,
			max_players_per_client_vpn,
			max_players_per_client_proxy,
			max_players_per_client_tor,
			max_players_per_client_hosting
		FROM game_namespaces
		WHERE namespace_id = ANY($1)
		"
	))
	.bind(namespace_ids)
	.fetch_all(&ctx.crdb("db-mm-config").await?)
	.await?
	.into_iter()
	.map(|ns| mm_config::namespace_get::response::Namespace {
		namespace_id: Some(ns.namespace_id.into()),
		config: Some(backend::matchmaker::NamespaceConfig {
			lobby_count_max: ns.lobby_count_max as u32,
			max_players_per_client: ns.max_players_per_client as u32,
			max_players_per_client_vpn: ns.max_players_per_client_vpn as u32,
			max_players_per_client_proxy: ns.max_players_per_client_proxy as u32,
			max_players_per_client_tor: ns.max_players_per_client_tor as u32,
			max_players_per_client_hosting: ns.max_players_per_client_hosting as u32,
		}),
	})
	.collect::<Vec<_>>();

	Ok(mm_config::namespace_get::Response { namespaces })
}
