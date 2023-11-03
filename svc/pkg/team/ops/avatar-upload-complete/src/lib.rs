use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "team-avatar-upload-complete")]
async fn handle(
	ctx: OperationContext<team::avatar_upload_complete::Request>,
) -> GlobalResult<team::avatar_upload_complete::Response> {
	let team_id = unwrap_ref!(ctx.team_id).as_uuid();
	let upload_id = unwrap_ref!(ctx.upload_id).as_uuid();

	op!([ctx] upload_complete {
		upload_id: ctx.upload_id,
		bucket: Some("bucket-team-avatar".into()),
	})
	.await?;

	// Set avatar id
	sql_query!(
		[ctx]
		"
		UPDATE db_team.teams set profile_id = $2
		WHERE team_id = $1
		",
		team_id,
		upload_id,
	)
	.await?;

	msg!([ctx] team::msg::update(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.avatar_set".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(team::avatar_upload_complete::Response {})
}
