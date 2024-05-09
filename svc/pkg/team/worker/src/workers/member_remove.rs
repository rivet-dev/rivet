use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "team-member-remove")]
async fn worker(ctx: &OperationContext<team::msg::member_remove::Message>) -> GlobalResult<()> {
	let team_id: Uuid = unwrap_ref!(ctx.team_id).as_uuid();
	let user_id: Uuid = unwrap_ref!(ctx.user_id).as_uuid();

	sql_execute!(
		[ctx]
		"DELETE FROM db_team.team_members WHERE team_id = $1 AND user_id = $2",
		team_id,
		user_id,
	)
	.await?;

	ctx.cache().purge("user_team_list", [user_id]).await?;

	// Dispatch events
	tokio::try_join!(
		async {
			GlobalResult::Ok(
				msg!([ctx] team::msg::update(team_id) {
					team_id: Some(team_id.into()),
				})
				.await?,
			)
		},
		async {
			Ok(msg!([ctx] user::msg::update(user_id) {
				user_id: Some(user_id.into()),
			})
			.await?)
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
				event_id: Some(Uuid::new_v4().into()),
				name: "team.member.remove".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
					"user_id": user_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
