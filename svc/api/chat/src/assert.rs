use api_helper::ctx::Ctx;
use proto::{backend, common};
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that a given user ID is a participant in a chat thread.
pub async fn chat_thread_participant(
	ctx: &Ctx<Auth>,
	thread_id: Uuid,
	user_id: Uuid,
) -> GlobalResult<()> {
	// Fetch chat participants
	let participants_res = op!([ctx] chat_thread_participant_list {
		thread_ids: vec![thread_id.into()],
	})
	.await?;

	// Check if participant
	let thread = internal_unwrap_owned!(participants_res.threads.first()).clone();
	let is_participant = thread
		.participants
		.iter()
		.map(|p| Ok(internal_unwrap!(p.user_id).as_uuid()))
		.collect::<GlobalResult<Vec<_>>>()?
		.contains(&user_id);

	util::assert_with!(is_participant, CHAT_THREAD_NOT_FOUND);

	Ok(())
}

pub async fn party_leader(
	ctx: &OperationContext<()>,
	party_id: Uuid,
	user_id: Uuid,
) -> GlobalResult<()> {
	let party_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await?;
	let party = unwrap_with_owned!(party_res.parties.first(), PARTY_IDENTITY_NOT_IN_ANY_PARTY);

	party_leader_with_party(user_id, party)
}

pub fn party_leader_with_party(user_id: Uuid, party: &backend::party::Party) -> GlobalResult<()> {
	let leader_user_id = party.leader_user_id.as_ref().map(common::Uuid::as_uuid);

	assert_eq_with!(
		Some(user_id),
		leader_user_id,
		PARTY_IDENTITY_NOT_PARTY_LEADER
	);

	Ok(())
}
