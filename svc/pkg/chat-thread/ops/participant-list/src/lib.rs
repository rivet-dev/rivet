use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("missing thread kind")]
	MissingThreadKind,
	#[error("missing team in response")]
	MissingTeamInResponse,
}

#[operation(name = "chat-thread-participant-list")]
async fn handle(
	ctx: OperationContext<chat_thread::participant_list::Request>,
) -> Result<chat_thread::participant_list::Response, GlobalError> {
	let thread_ids = ctx
		.thread_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch thread data
	let threads_res = op!([ctx] chat_thread_get {
		thread_ids: thread_ids
			.iter()
			.cloned()
			.map(common::Uuid::from)
			.collect::<Vec<_>>(),
	})
	.await?;

	// Fetch thread participants for each given thread kind
	let mut thread_futs = Vec::new();
	for thread in threads_res.threads.clone() {
		let ctx = ctx.base();

		let fut = async move {
			let thread_kind = thread
				.topic
				.ok_or(Error::MissingThreadKind)?
				.kind
				.ok_or(Error::MissingThreadKind)?;
			let participants: Vec<chat_thread::participant_list::response::Participant> =
				match thread_kind {
					backend::chat::topic::Kind::Team(team) => {
						// Fetch team
						let team_id = internal_unwrap!(team.team_id).as_uuid();
						let team_members_res = op!([ctx] team_member_list {
							team_ids: vec![team_id.into()],
							limit: None,
							anchor: None,
						})
						.await?;

						// Extract participants
						team_members_res
							.teams
							.first()
							.ok_or(Error::MissingTeamInResponse)?
							.members
							.iter()
							.map(
								|user| chat_thread::participant_list::response::Participant {
									user_id: user.user_id,
								},
							)
							.collect::<Vec<_>>()
					}
					backend::chat::topic::Kind::Party(party) => {
						// Fetch party
						let party_id = internal_unwrap!(party.party_id).as_uuid();
						let party_members_res = op!([ctx] party_member_list {
							party_ids: vec![party_id.into()],
						})
						.await?;

						// Extract participants
						internal_unwrap_owned!(party_members_res.parties.first())
							.user_ids
							.iter()
							.map(
								|user_id| chat_thread::participant_list::response::Participant {
									user_id: Some(*user_id),
								},
							)
							.collect::<Vec<_>>()
					}
					backend::chat::topic::Kind::Direct(direct) => {
						// Fetch direct chat
						let user_a_id = internal_unwrap!(direct.user_a_id);
						let user_b_id = internal_unwrap!(direct.user_b_id);

						let direct_chat_users_res = op!([ctx] user_get {
							user_ids: vec![*user_a_id, *user_b_id],
						})
						.await?;

						// Extract participants
						direct_chat_users_res
							.users
							.iter()
							.map(
								|user| chat_thread::participant_list::response::Participant {
									user_id: user.user_id,
								},
							)
							.collect::<Vec<_>>()
					}
				};

			Result::<_, GlobalError>::Ok(chat_thread::participant_list::response::Thread {
				thread_id: thread.thread_id,
				participants,
			})
		};
		thread_futs.push(fut);
	}
	let threads = futures_util::future::try_join_all(thread_futs).await?;

	Ok(chat_thread::participant_list::Response { threads })
}
