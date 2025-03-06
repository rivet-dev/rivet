use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-version-create")]
async fn handle(
	ctx: OperationContext<game::version_create::Request>,
) -> GlobalResult<game::version_create::Response> {
	let game_id = unwrap_ref!(ctx.game_id).as_uuid();

	// TODO: Replace all asserts with xxx-version-validate after it is split into multiple services
	ensure!(
		util::check::display_name_long(&ctx.display_name),
		"invalid display name"
	);

	// Validate display name uniqueness
	// TODO: Should this be replaced with a service that queries the versions db with a display name?
	{
		let version_list_res = op!([ctx] game_version_list {
			game_ids: vec![game_id.into()],
		})
		.await?;

		let version_list = version_list_res.games.first();
		let version_list = unwrap_ref!(version_list);

		let versions_res = op!([ctx] game_version_get {
			version_ids: version_list.version_ids.clone(),
		})
		.await?;

		ensure!(
			!versions_res
				.versions
				.iter()
				.any(|ver| ver.display_name == ctx.display_name),
			"display name not unique"
		);
	}

	let version_id = Uuid::new_v4();

	sql_execute!(
		[ctx]
		"INSERT INTO db_game.game_versions (version_id, game_id, create_ts, display_name) VALUES ($1, $2, $3, $4)",
		version_id,
		game_id,
		ctx.ts(),
		&ctx.display_name,
	)
		.await?;

	Ok(game::version_create::Response {
		version_id: Some(version_id.into()),
	})
}
