use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-owner-transfer")]
async fn worker(ctx: &OperationContext<team::msg::owner_transfer::Message>) -> GlobalResult<()> {
	let raw_team_id = unwrap_ref!(ctx.team_id);
	let team_id = raw_team_id.as_uuid();
	let new_owner_user_id = unwrap_ref!(ctx.new_owner_user_id).as_uuid();

	let crdb = ctx.crdb().await?;
	let (old_owner_user_id,) = sql_fetch_one!(
		[ctx, (Uuid,)]
		"SELECT owner_user_id FROM db_team.teams WHERE team_id = $1",
		team_id,
	)
	.await?;

	tokio::try_join!(
		sql_execute!(
			[ctx, &crdb]
			"UPDATE db_team.teams SET owner_user_id = $2 WHERE team_id = $1",
			team_id,
			new_owner_user_id,
		),
		sql_execute!(
			[ctx, &crdb]
			"
			INSERT INTO db_team.team_owner_transfer_logs
			(team_id, old_owner_user_id, new_owner_user_id, transfer_ts)
			VALUES ($1, $2, $3, $4)
			",
			team_id,
			old_owner_user_id,
			new_owner_user_id,
			util::timestamp::now(),
		),
	)?;

	msg!([ctx] team::msg::update(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	Ok(())
}
