use std::convert::TryInto;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Team {
	team_id: Uuid,
	owner_user_id: Uuid,
	display_name: String,
	bio: String,
	profile_id: Option<Uuid>,
	create_ts: i64,
	publicity: i64,
}

#[operation(name = "team-get")]
async fn handle(ctx: OperationContext<team::get::Request>) -> GlobalResult<team::get::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let teams = sqlx::query_as::<_, Team>(indoc!(
		"
		SELECT
			team_id,
			owner_user_id,
			display_name,
			bio,
			profile_id,
			create_ts,
			publicity
		FROM teams
		WHERE team_id = ANY($1)
		"
	))
	.bind(team_ids)
	.fetch_all(&ctx.crdb("db-team").await?)
	.await?;

	let upload_ids = teams
		.iter()
		.filter_map(|team| team.profile_id.map(Into::into))
		.collect::<Vec<_>>();

	let upload_res = op!([ctx] upload_get {
		upload_ids: upload_ids.clone(),
	})
	.await?;
	let files_res = op!([ctx] upload_file_list {
		upload_ids: upload_ids.clone(),
	})
	.await?;

	Ok(team::get::Response {
		teams: teams
			.into_iter()
			.map(|team| {
				let profile_id = team.profile_id.map(Into::<common::Uuid>::into);

				// Fetch all information relating to the profile image
				let (profile_upload_complete_ts, profile_file_name, profile_provider) = {
					let upload = upload_res
						.uploads
						.iter()
						.find(|upload| upload.upload_id == profile_id);
					let file = files_res
						.files
						.iter()
						.find(|file| file.upload_id == profile_id);

					if let (Some(upload), Some(file)) = (upload, file) {
						let profile_file_name = file
							.path
							.rsplit_once('/')
							.map(|(_, file_name)| file_name.to_owned())
							.or(Some(file.path.clone()));
						(upload.complete_ts, profile_file_name, Some(upload.provider))
					} else {
						Default::default()
					}
				};

				Ok(backend::team::Team {
					team_id: Some(team.team_id.into()),
					owner_user_id: Some(team.owner_user_id.into()),
					display_name: team.display_name,
					bio: team.bio,
					profile_upload_id: if profile_upload_complete_ts.is_some() {
						profile_id
					} else {
						None
					},
					profile_file_name,
					profile_provider,
					create_ts: team.create_ts,
					publicity: team.publicity.try_into()?,
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
