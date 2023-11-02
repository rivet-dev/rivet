use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct LobbyRow {
	lobby_id: Uuid,
	creator_user_id: Uuid,
}

#[operation(name = "mm-lobby-list-for-user-id")]
async fn handle(
	ctx: OperationContext<mm::lobby_list_for_user_id::Request>,
) -> GlobalResult<mm::lobby_list_for_user_id::Response> {
	let crdb = ctx.crdb().await?;
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let lobbies = sql_fetch_all!(
		[ctx, LobbyRow]
		"
		SELECT
			lobby_id,
			creator_user_id
		FROM db_mm_state.lobbies
		WHERE creator_user_id = ANY($1)
		",
		&user_ids,
	)
	.await?;

	Ok(mm::lobby_list_for_user_id::Response {
		users: user_ids
			.into_iter()
			.map(|user_id| {
				let lobby_ids = lobbies
					.iter()
					.filter(|l| l.creator_user_id == user_id)
					.map(|l| l.lobby_id.into())
					.collect::<Vec<_>>();

				mm::lobby_list_for_user_id::response::User {
					user_id: Some(user_id.into()),
					lobby_ids,
				}
			})
			.collect::<Vec<_>>(),
	})
}
