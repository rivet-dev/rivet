use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-dev-create")]
async fn worker(ctx: &OperationContext<team_dev::msg::create::Message>) -> GlobalResult<()> {
	let team_id = unwrap_ref!(ctx.team_id).as_uuid();

	// Get the team
	let team_res = op!([ctx] team_get {
		team_ids: vec![team_id.into()],
	})
	.await?;
	let team = unwrap!(team_res.teams.first(), "team not found");
	let owner_user_id = unwrap!(team.owner_user_id);

	let dev_team_res = op!([ctx] team_dev_get {
		team_ids: vec![team_id.into()],
	})
	.await?;
	if !dev_team_res.teams.is_empty() {
		// TODO: RIV-2281
		tracing::info!("team is already a dev team");
		return Ok(());
	}

	// Create the dev team
	let crdb = ctx.crdb().await?;
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_team_dev.dev_teams (team_id, create_ts)
		VALUES ($1, $2)
		",
		team_id,
		ctx.ts(),
	)
	.await?;

	msg!([ctx] team::msg::update(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	msg!([ctx] team::msg::create_complete(team_id) {
		team_id: Some(team_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.dev.create".into(),
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
