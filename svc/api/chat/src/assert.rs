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
