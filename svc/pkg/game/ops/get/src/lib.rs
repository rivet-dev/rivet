use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Game {
	game_id: Uuid,
	create_ts: i64,
	name_id: String,
	display_name: String,
	url: String,
	developer_team_id: Uuid,
	description: String,
	tags: Vec<String>,
	logo_upload_id: Option<Uuid>,
	banner_upload_id: Option<Uuid>,
	plan_code: Option<String>,
	subscription_id: Option<Uuid>,
}

#[operation(name = "game-get")]
async fn handle(
	ctx: OperationContext<game::get::Request>,
) -> Result<game::get::Response, GlobalError> {
	let game_ids = ctx
		.game_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	let games = sqlx::query_as::<_, Game>(indoc!(
		"
		SELECT
			game_id,
			create_ts,
			name_id,
			display_name,
			url,
			developer_team_id,
			description,
			array(
				SELECT tag
				FROM game_tags
				WHERE game_tags.game_id = games.game_id
			) AS tags,
			logo_upload_id,
			banner_upload_id,
			plan_code,
			subscription_id
		FROM games
		WHERE game_id = ANY($1)
		"
	))
	.bind(game_ids)
	.fetch_all(&ctx.crdb("db-game").await?)
	.await?;

	let upload_ids = games
		.iter()
		.flat_map(|game| [game.logo_upload_id, game.banner_upload_id])
		.collect::<Vec<_>>()
		.into_iter()
		.flatten()
		.map(Into::into)
		.collect::<Vec<_>>();

	let upload_res = op!([ctx] upload_get {
		upload_ids: upload_ids.clone(),
	})
	.await?;

	let files_res = op!([ctx] upload_file_list {
		upload_ids: upload_ids.clone(),
	})
	.await?;

	Ok(game::get::Response {
		games: games
			.into_iter()
			.map(|game| {
				let logo_upload_id = game.logo_upload_id.map(Into::<common::Uuid>::into);
				let banner_upload_id = game.banner_upload_id.map(Into::<common::Uuid>::into);

				// Fetch all information relating to the logo image
				let (logo_upload_complete_ts, logo_file_name) = {
					let upload = upload_res
						.uploads
						.iter()
						.find(|upload| upload.upload_id == logo_upload_id);
					let file = files_res
						.files
						.iter()
						.find(|file| file.upload_id == logo_upload_id);

					if let (Some(upload), Some(file)) = (upload, file) {
						let logo_file_name = file
							.path
							.rsplit_once('/')
							.map(|(_, file_name)| file_name.to_owned())
							.or(Some(file.path.clone()));
						(upload.complete_ts, logo_file_name)
					} else {
						Default::default()
					}
				};

				// Fetch all information relating to the banner image
				let (banner_upload_complete_ts, banner_file_name) = {
					let upload = upload_res
						.uploads
						.iter()
						.find(|upload| upload.upload_id == banner_upload_id);
					let file = files_res
						.files
						.iter()
						.find(|file| file.upload_id == banner_upload_id);

					if let (Some(upload), Some(file)) = (upload, file) {
						let banner_file_name = file
							.path
							.rsplit_once('/')
							.map(|(_, file_name)| file_name.to_owned())
							.or(Some(file.path.clone()));
						(upload.complete_ts, banner_file_name)
					} else {
						Default::default()
					}
				};

				backend::game::Game {
					game_id: Some(game.game_id.into()),
					create_ts: game.create_ts,
					name_id: game.name_id,
					display_name: game.display_name,
					url: game.url,
					developer_team_id: Some(game.developer_team_id.into()),
					description: game.description,
					tags: game.tags,

					logo_upload_id: if logo_upload_complete_ts.is_some() {
						logo_upload_id
					} else {
						None
					},
					logo_file_name,
					banner_upload_id: if banner_upload_complete_ts.is_some() {
						banner_upload_id
					} else {
						None
					},
					banner_file_name,
					plan_code: game.plan_code.clone(),
					subscription_id: game.subscription_id.map(Into::into),
				}
			})
			.collect::<Vec<_>>(),
	})
}
