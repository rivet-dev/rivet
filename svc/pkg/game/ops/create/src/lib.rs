use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "game-create")]
async fn handle(
	ctx: OperationContext<game::create::Request>,
) -> GlobalResult<game::create::Response> {
	let developer_team_id_proto = unwrap!(ctx.developer_team_id);
	let developer_team_id = developer_team_id_proto.as_uuid();

	// Validate game
	let validation_res = op!([ctx] game_validate {
		name_id: ctx.name_id.clone(),
		display_name: ctx.display_name.clone(),
	})
	.await?;
	if !validation_res.errors.is_empty() {
		let readable_errors = validation_res
			.errors
			.iter()
			.map(|err| err.path.join("."))
			.collect::<Vec<_>>()
			.join(", ");
		bail_with!(VALIDATION_ERROR, error = readable_errors);
	}

	// Check if team can create a game
	{
		let team_res = op!([ctx] team_get {
			team_ids: vec![developer_team_id_proto],
		})
		.await?;
		let team = unwrap!(team_res.teams.first());
		ensure_with!(
			team.deactivate_reasons.is_empty(),
			GROUP_DEACTIVATED,
			reasons = util_team::format_deactivate_reasons(&team.deactivate_reasons)?,
		);
	}

	// TODO: Deprecate `url` and `description` columns
	let game_id = Uuid::new_v4();
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_game.games (
			game_id,
			create_ts,
			name_id,
			display_name,
			url,
			description,
			developer_team_id
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		",
		game_id,
		ctx.ts(),
		&ctx.name_id,
		&ctx.display_name,
		"",
		"",
		developer_team_id,
	)
	.await?;

	if let Some(cluster_id) = ctx.cluster_id {
		chirp_workflow::compat::tagged_signal(
			ctx.op_ctx(),
			&json!({
				"cluster_id": cluster_id.as_uuid(),
			}),
			cluster::workflows::cluster::GameLink { game_id },
		)
		.await
		.unwrap();
	}

	msg!([ctx] game::msg::create_complete(game_id) {
		game_id: Some(game_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "game.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"developer_team_id": developer_team_id,
					"game_id": game_id,
					"display_name": ctx.display_name,
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
