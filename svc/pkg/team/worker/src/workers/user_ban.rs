use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-user-ban")]
async fn worker(ctx: &OperationContext<team::msg::user_ban::Message>) -> GlobalResult<()> {
	let team_id = unwrap_ref!(ctx.team_id).as_uuid();
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_team.banned_users (team_id, user_id, ban_ts)
		VALUES ($1, $2, $3)
		ON CONFLICT
		DO NOTHING
		",
		team_id,
		user_id,
		util::timestamp::now(),
	)
	.await?;

	// TODO: Establish audit logs
	// sql_execute!(
	// 	[ctx]
	// 	"INSERT INTO team_audit_logs WHERE team_id = $1",
	// 	team_id,
	// 	user_id,
	// )
	// 	.await?;

	// Dispatch events
	msg!([ctx] team::msg::member_remove(team_id, user_id) -> team::msg::member_remove_complete {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		silent: true,
	})
	.await?;

	msg!([ctx] team::msg::user_ban_complete(team_id, user_id) {
		team_id: ctx.team_id,
		user_id: ctx.user_id,
		banner_user_id: ctx.banner_user_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.user.ban".into(),
				user_id: ctx.banner_user_id,
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
					"banned_user_id": user_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
