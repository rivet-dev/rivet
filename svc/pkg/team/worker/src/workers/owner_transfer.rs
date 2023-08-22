use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-owner-transfer")]
async fn worker(ctx: OperationContext<team::msg::owner_transfer::Message>) -> GlobalResult<()> {
	let raw_team_id = internal_unwrap!(ctx.team_id);
	let team_id = raw_team_id.as_uuid();
	let new_owner_user_id = internal_unwrap!(ctx.new_owner_user_id).as_uuid();

	let crdb = ctx.crdb("db-team").await?;
	let (old_owner_user_id,) =
		sqlx::query_as::<_, (Uuid,)>("SELECT owner_user_id FROM teams WHERE team_id = $1")
			.bind(team_id)
			.fetch_one(&crdb)
			.await?;

	tokio::try_join!(
		sqlx::query("UPDATE teams SET owner_user_id = $2 WHERE team_id = $1",)
			.bind(team_id)
			.bind(new_owner_user_id)
			.execute(&crdb),
		sqlx::query(indoc!(
			"
			INSERT INTO team_owner_transfer_logs
			(team_id, old_owner_user_id, new_owner_user_id, transfer_ts)
			VALUES ($1, $2, $3, $4)
			",
		))
		.bind(team_id)
		.bind(old_owner_user_id)
		.bind(new_owner_user_id)
		.bind(util::timestamp::now())
		.execute(&crdb),
	)?;

	let teams_res = op!([ctx] team_dev_get {
		team_ids: vec![*raw_team_id],
	})
	.await?;

	// TODO: Update stripe account email

	msg!([ctx] team::msg::update(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	Ok(())
}
