use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-user-mm-lobby-join")]
async fn worker(ctx: &OperationContext<user::msg::mm_lobby_join::Message>) -> GlobalResult<()> {
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	msg!([ctx] user::msg::event(user_id) {
		user_id: Some(user_id.into()),
		event: Some(backend::user::event::Event {
			kind: Some(backend::user::event::event::Kind::MatchmakerLobbyJoin(backend::user::event::MatchmakerLobbyJoin {
				namespace_id: ctx.namespace_id,
				query_id: ctx.query_id,
				lobby_id: ctx.lobby_id,
				player_id: ctx.player_id,
				player_token: ctx.player_token.clone(),
			})),
		}),
	}).await?;

	Ok(())
}
