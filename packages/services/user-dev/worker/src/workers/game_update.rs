use chirp_worker::prelude::*;
use futures_util::{
	stream::{StreamExt, TryStreamExt},
	FutureExt,
};
use proto::backend::pkg::*;

#[worker(name = "user-dev-game-update")]
async fn worker(ctx: &OperationContext<game::msg::update::Message>) -> GlobalResult<()> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = unwrap!(game_res.games.first());
	let developer_team_id = unwrap_ref!(game.developer_team_id);

	let members_res = op!([ctx] team_member_list {
		team_ids: vec![*developer_team_id],
		limit: None,
		anchor: None,
	})
	.await?;
	let team = unwrap!(members_res.teams.first());

	// Insert event for each team member
	let mut events = Vec::new();
	for member in &team.members {
		let user_id = unwrap_ref!(member.user_id).as_uuid();

		events.push(
			msg!([ctx] user_dev::msg::game_update(user_id) {
				user_id: member.user_id,
				game_id: ctx.game_id,
			})
			.boxed(),
		);
	}

	// Dispatch events
	futures_util::stream::iter(events)
		.buffer_unordered(32)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}
