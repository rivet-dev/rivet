use api_helper::ctx::Ctx;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_operation::prelude::*;

use crate::auth::Auth;

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

pub async fn members(
	ctx: &Ctx<Auth>,
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
			user_ids:	user_ids,
		}),
	)?;

	Ok((member_res.party_members, user_res.users))
}

pub async fn threads(
	ctx: &Ctx<Auth>,
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
	ctx: &Ctx<Auth>,
	user_id: Uuid,
	party_ids: Vec<common::Uuid>,
) -> GlobalResult<party::publicity_for_user::Response> {
	op!([ctx] party_publicity_for_user {
		user_id: Some(user_id.into()),
		party_ids: party_ids,
	})
	.await
}
