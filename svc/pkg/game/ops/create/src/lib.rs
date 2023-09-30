use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "game-create")]
async fn handle(
	ctx: OperationContext<game::create::Request>,
) -> GlobalResult<game::create::Response> {
	let game::create::Request {
		name_id,
		display_name,
		developer_team_id,
		creator_user_id: _,
	} = ctx.body();
	let developer_team_id_proto = *internal_unwrap_owned!(developer_team_id);
	let developer_team_id = developer_team_id_proto.as_uuid();

	// Validate game
	let validation_res = op!([ctx] game_validate {
		name_id: name_id.to_owned(),
		display_name: display_name.to_owned(),
	})
	.await?;
	if !validation_res.errors.is_empty() {
		tracing::warn!(errors = ?validation_res.errors, "validation errors");

		let readable_errors = validation_res
			.errors
			.iter()
			.map(|err| err.path.join("."))
			.collect::<Vec<_>>()
			.join(", ");
		panic_with!(VALIDATION_ERROR, error = readable_errors);
	}

	// Check if team can create a game
	{
		let dev_team_res = op!([ctx] team_dev_get {
			team_ids: vec![developer_team_id_proto]
		})
		.await?;
		let dev_team = unwrap_with_owned!(dev_team_res.teams.first(), GROUP_NOT_DEVELOPER_GROUP);
		let status = internal_unwrap_owned!(backend::team::dev_team::DevStatus::from_i32(
			dev_team.status
		));
		assert_with!(
			matches!(status, backend::team::dev_team::DevStatus::Active),
			GROUP_INVALID_DEVELOPER_STATUS
		);
	}

	let crdb = ctx.crdb("db-game").await?;
	let game_id = Uuid::new_v4();
	let plan_code = "free";
	let subscription_id = Uuid::new_v4();
	sqlx::query(indoc!(
		"
		INSERT INTO games (
			game_id,
			create_ts,
			name_id,
			display_name,
			url,
			description,
			developer_team_id,
			plan_code,
			subscription_id
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		"
	))
	.bind(game_id)
	.bind(ctx.ts())
	.bind(name_id)
	.bind(display_name)
	.bind("")
	.bind("")
	.bind(developer_team_id)
	.bind(plan_code)
	.bind(subscription_id)
	.execute(&crdb)
	.await?;

	// TODO: Add stripe subscription for game

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "game.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"display_name": display_name,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(game::create::Response {
		game_id: Some(game_id.into()),
	})
}
