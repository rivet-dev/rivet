use api_helper::ctx::Ctx;
use cluster::types::PoolType;
use rivet_operation::prelude::*;

use crate::{auth::Auth, route::GlobalQuery};

/// Converts the legacy UUID-based routing for games & ns to the slug-based routing.
#[tracing::instrument(skip_all)]
pub async fn build_global_query_compat(
	ctx: &Ctx<Auth>,
	project_id: Uuid,
	env_id: Uuid,
) -> GlobalResult<GlobalQuery> {
	let (game_res, ns_res) = tokio::try_join!(
		op!([ctx] game_get {
			game_ids: vec![project_id.into()],
		}),
		op!([ctx] game_namespace_get {
			namespace_ids: vec![env_id.into()]
		}),
	)?;

	let game = unwrap!(game_res.games.first());
	let ns = unwrap!(ns_res.namespaces.first());

	ensure_eq!(ns.game_id, game.game_id, "env does not belong to project");

	Ok(GlobalQuery {
		project: Some(game.name_id.clone()),
		environment: Some(ns.name_id.clone()),
	})
}

/// Called to validate that a datacenter can be contacted.
///
/// If there is no worker & guard nodes in this dc, the dc is probably shut down.
pub fn filter_edge_dc(
	config: &rivet_config::Config,
	dc: &cluster::types::Datacenter,
) -> GlobalResult<bool> {
	if config.server()?.rivet.has_multiple_server_types() {
		// Validate that the dc has a worker & guard so it can be contacted
		Ok(dc.pools.iter().any(|x| {
			x.pool_type == PoolType::Worker && x.desired_count.max(x.min_count).min(x.max_count) > 0
		}) && dc.pools.iter().any(|x| {
			x.pool_type == PoolType::Guard && x.desired_count.max(x.min_count).min(x.max_count) > 0
		}))
	} else {
		// All DC are valid
		Ok(true)
	}
}
