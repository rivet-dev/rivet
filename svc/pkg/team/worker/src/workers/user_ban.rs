use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-user-ban")]
async fn worker(ctx: &OperationContext<team::msg::user_ban::Message>) -> GlobalResult<()> {
	let team_id = internal_unwrap!(ctx.team_id).as_uuid();
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO banned_users (team_id, user_id, ban_ts)
		VALUES ($1, $2, $3)
		"
	))
	.bind(team_id)
	.bind(user_id)
	.bind(util::timestamp::now())
	.execute(&ctx.crdb("db-team").await?)
	.await?;

	// TODO: Establish audit logs
	// sqlx::query("INSERT INTO team_audit_logs WHERE team_id = $1")
	// 	.bind(team_id)
	// 	.bind(user_id)
	// 	.execute(&ctx.crdb("db-team").await?)
	// 	.await?;

	// Dispatch events
	tokio::try_join!(
		async {
			Ok(msg!([ctx] team::msg::member_remove(team_id, user_id) -> team::msg::member_remove_complete {
				team_id: Some(team_id.into()),
				user_id: Some(user_id.into()),
				silent: true,
			})
			.await?)
		},
		async {
			// TODO: Make ban message?
			// Send team member kick message
			let chat_message_id = Uuid::new_v4();
			op!([ctx] chat_message_create_with_topic {
				chat_message_id: Some(chat_message_id.into()),
				topic: Some(backend::chat::Topic {
					kind: Some(backend::chat::topic::Kind::Team(
						backend::chat::topic::Team {
							team_id: Some(team_id.into()),
						},
					)),
				}),
				send_ts: util::timestamp::now(),
				body: Some(backend::chat::MessageBody {
					kind: Some(backend::chat::message_body::Kind::TeamMemberKick(backend::chat::message_body::TeamMemberKick {
						user_id: Some(user_id.into()),
					})),
				}),
			})
			.await
			.map_err(Into::<GlobalError>::into)
		},
	)?;

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
