use api_helper::ctx::Ctx;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_chat_server::models;
use rivet_operation::prelude::*;

use crate::{auth::Auth, convert};

pub async fn handles(
	ctx: &Ctx<Auth>,
	raw_party_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::PartyHandle>> {
	let party_ids = raw_party_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let (parties, games) = parties_and_games(ctx, party_ids.clone(), Vec::new()).await?;

	parties
		.parties
		.iter()
		.map(|party| convert::party::handle(party, &games))
		.collect::<GlobalResult<Vec<_>>>()
}

pub async fn parties_and_games(
	ctx: &Ctx<Auth>,
	party_ids: Vec<common::Uuid>,
	extra_namespace_ids: Vec<common::Uuid>,
) -> GlobalResult<(party::get::Response, Vec<convert::GameWithNamespaceIds>)> {
	let parties_res = parties(ctx, party_ids).await?;
	let mut namespace_ids = game_namespace_ids(&parties_res)?;
	namespace_ids.extend(extra_namespace_ids);

	let game_resolve_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: namespace_ids,
	})
	.await?;

	let games_res = op!([ctx] game_get {
		game_ids: game_resolve_res.games
			.iter()
			.map(|game| Ok(*internal_unwrap!(game.game_id)))
			.collect::<GlobalResult<Vec<_>>>()?,
	})
	.await?;

	// Collects games and their namespace ids together
	let games_with_namespace_ids = game_resolve_res
		.games
		.iter()
		.map(|resolved_game| {
			let game = internal_unwrap_owned!(games_res
				.games
				.iter()
				.find(|game| resolved_game.game_id == game.game_id));

			Ok(convert::GameWithNamespaceIds {
				namespace_ids: resolved_game
					.namespace_ids
					.iter()
					.map(common::Uuid::as_uuid)
					.collect::<Vec<_>>(),
				game: game.clone(),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok((parties_res, games_with_namespace_ids))
}

pub async fn parties(
	ctx: &Ctx<Auth>,
	party_ids: Vec<common::Uuid>,
) -> GlobalResult<party::get::Response> {
	op!([ctx] party_get {
		party_ids: party_ids,
	})
	.await
}

pub fn game_namespace_ids(parties_res: &party::get::Response) -> GlobalResult<Vec<common::Uuid>> {
	Ok(parties_res
		.parties
		.iter()
		.map(|party| match &party.state {
			None => Ok(None),
			Some(backend::party::party::State::MatchmakerFindingLobby(
				backend::party::party::StateMatchmakerFindingLobby { namespace_id, .. },
			)) => Ok(Some(*internal_unwrap!(namespace_id))),
			Some(backend::party::party::State::MatchmakerLobby(
				backend::party::party::StateMatchmakerLobby { namespace_id, .. },
			)) => Ok(Some(*internal_unwrap!(namespace_id))),
		})
		.collect::<GlobalResult<Vec<_>>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>())
}
