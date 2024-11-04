use api_helper::ctx::Ctx;
use rivet_operation::prelude::*;

use crate::{auth::Auth, route::GlobalQuery};

/// Converts the legacy UUID-based routing for games & ns to the slug-based routing.
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

	Ok(GlobalQuery::ProjectAndEnvironment {
		project: game.name_id.clone(),
		environment: ns.name_id.clone(),
	})
}
