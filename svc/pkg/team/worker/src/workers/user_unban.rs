use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "team-user-unban")]
async fn worker(ctx: &OperationContext<team::msg::user_unban::Message>) -> GlobalResult<()> {
	let team_id = internal_unwrap!(ctx.team_id).as_uuid();
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	sqlx::query(indoc!(
		"
		DELETE FROM db_team.banned_users
		WHERE team_id = $1
		AND user_id = $2
		"
	))
	.bind(team_id)
	.bind(user_id)
	.execute(&ctx.crdb().await?)
	.await?;

	// TODO: Establish audit logs
	// sqlx::query("INSERT INTO team_audit_logs WHERE team_id = $1")
	// 	.bind(team_id)
	// 	.bind(user_id)
	// 	.execute(&ctx.crdb("db-team").await?)
	// 	.await?;

	msg!([ctx] team::msg::user_unban_complete(team_id, user_id) {
		team_id: ctx.team_id,
		user_id: ctx.user_id,
		unbanner_user_id: ctx.unbanner_user_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.user.unban".into(),
				user_id: ctx.unbanner_user_id,
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
					"unban_user_id": user_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
