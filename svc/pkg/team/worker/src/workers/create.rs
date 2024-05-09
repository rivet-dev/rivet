use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "team-create")]
async fn worker(ctx: &OperationContext<team::msg::create::Message>) -> GlobalResult<()> {
	let team_id = unwrap_ref!(ctx.team_id).as_uuid();

	// Validate team
	let validation_res = op!([ctx] team_validate {
		display_name: ctx.display_name.to_owned(),
	})
	.await?;
	if !validation_res.errors.is_empty() {
		tracing::warn!(errors = ?validation_res.errors, "validation errors");

		msg!([ctx] team::msg::create_fail(team_id) {
			team_id: Some(team_id.into()),
			error_code: team::msg::create_fail::ErrorCode::ValidationFailed as i32,
		})
		.await?;

		return Ok(());
	}

	let owner_user_id = unwrap_ref!(ctx.owner_user_id).as_uuid();

	// Create the team
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_team.teams (team_id, owner_user_id, display_name, create_ts)
		VALUES ($1, $2, $3, $4)
		",
		team_id,
		owner_user_id,
		&ctx.display_name,
		util::timestamp::now(),
	)
	.await?;

	// Wait for message to ensure it sends before team member creation
	msg!([ctx] @wait team::msg::create_complete(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	// Create team member (after `team::msg::create_complete` which creates the chat)
	msg!([ctx] team::msg::member_create(team_id, owner_user_id) -> team::msg::member_create_complete {
		team_id: Some(team_id.into()),
		user_id: Some(owner_user_id.into()),
		invitation: None,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "team.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
					"user_id": owner_user_id,
				}))?),
				..Default::default()
			},
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "team.profile_set".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"display_name": ctx.display_name,
					"has_bio": false,
				}))?),
				..Default::default()
			},
		],
	})
	.await?;

	Ok(())
}
