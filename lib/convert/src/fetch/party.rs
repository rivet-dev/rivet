use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::convert;

pub async fn summaries(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	raw_party_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::PartySummary>> {
	let party_ids = raw_party_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let ((parties, games), (members, users), threads) = tokio::try_join!(
		parties_and_games(ctx, party_ids.clone(), Vec::new()),
		members(ctx, party_ids.clone()),
		threads(ctx, party_ids.clone()),
	)?;

	let parties = parties
		.parties
		.iter()
		.map(|party| {
			(
				party.party_id,
				convert::party::summary(
					&current_user_id,
					party,
					&games,
					&members,
					&users,
					&threads.threads,
				),
			)
		})
		.filter_map(|(party_id, result)| match result {
			Ok(x) => Some(x),
			Err(err) => {
				tracing::error!(?err, ?party_id, "failed to fetch party");
				None
			}
		})
		.collect::<Vec<_>>();

	Ok(parties)
}

pub async fn parties_and_games(
	ctx: &OperationContext<()>,
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
					.map(|id| id.as_uuid())
					.collect::<Vec<_>>(),
				game: game.clone(),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok((parties_res, games_with_namespace_ids))
}

pub async fn parties(
	ctx: &OperationContext<()>,
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

pub async fn members(
	ctx: &OperationContext<()>,
	party_ids: Vec<common::Uuid>,
) -> GlobalResult<(Vec<backend::party::PartyMember>, Vec<backend::user::User>)> {
	let member_list_res = op!([ctx] party_member_list {
		party_ids: party_ids,
	})
	.await?;
	let user_ids = member_list_res
		.parties
		.iter()
		.flat_map(|party| party.user_ids.clone())
		.collect::<Vec<_>>();

	let (member_res, user_res) = tokio::try_join!(
		op!([ctx] party_member_get {
			user_ids: user_ids.clone(),
		}),
		op!([ctx] user_get {
			user_ids: user_ids,
		}),
	)?;

	Ok((member_res.party_members.clone(), user_res.users.clone()))
}

pub async fn threads(
	ctx: &OperationContext<()>,
	party_ids: Vec<common::Uuid>,
) -> GlobalResult<chat_thread::get_for_topic::Response> {
	op!([ctx] chat_thread_get_for_topic {
		topics: party_ids
			.iter()
			.map(|party_id| backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Party(backend::chat::topic::Party {
					party_id: Some(*party_id),
				})),
			})
			.collect::<Vec<_>>(),
	})
	.await
}

pub async fn publicity(
	ctx: &OperationContext<()>,
	user_id: Uuid,
	party_ids: Vec<common::Uuid>,
) -> GlobalResult<party::publicity_for_user::Response> {
	op!([ctx] party_publicity_for_user {
		user_id: Some(user_id.into()),
		party_ids: party_ids,
	})
	.await
}
