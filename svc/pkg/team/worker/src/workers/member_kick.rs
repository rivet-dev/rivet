use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-member-kick")]
async fn worker(ctx: &OperationContext<team::msg::member_kick::Message>) -> GlobalResult<()> {
	let team_id = unwrap_ref!(ctx.team_id).as_uuid();
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	// TODO: Establish audit logs
	// sql_query!(
	// 	[ctx]
	// 	"INSERT INTO team_audit_logs WHERE team_id = $1",
	// 	team_id,
	// 	user_id,
	// )
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

	msg!([ctx] team::msg::member_kick_complete(team_id, user_id) { }).await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.member.kick".into(),
				user_id: ctx.kicker_user_id,
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
					"kick_user_id": user_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
