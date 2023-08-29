use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-member-remove")]
async fn worker(ctx: &OperationContext<team::msg::member_remove::Message>) -> GlobalResult<()> {
	let team_id: Uuid = internal_unwrap!(ctx.team_id).as_uuid();
	let user_id: Uuid = internal_unwrap!(ctx.user_id).as_uuid();

	sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
		.bind(team_id)
		.bind(user_id)
		.execute(&ctx.crdb("db-team").await?)
		.await?;

	// Dispatch events
	tokio::try_join!(
		async {
			Ok(msg!([ctx] team::msg::update(team_id) {
				team_id: Some(team_id.into()),
			})
			.await?)
		},
		async {
			Ok(msg!([ctx] user::msg::update(user_id) {
				user_id: Some(user_id.into()),
			})
			.await?)
		},
		async {
			if !ctx.silent {
				// Send team member leave message
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
						kind: Some(backend::chat::message_body::Kind::TeamLeave(backend::chat::message_body::TeamLeave {
							user_id: Some(user_id.into()),
						})),
					}),
				})
				.await?;
			}

			GlobalResult::Ok(())
		},
	)?;

	msg!([ctx] team::msg::member_remove_complete(team_id, user_id) {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.member.remove".into(),
				user_id: Some(user_id.into()),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
