use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct User {
	user_id: Uuid,
	display_name: String,
	account_number: i64,
	avatar_id: String,
	profile_id: Option<Uuid>,
	join_ts: i64,
	bio: String,
	is_admin: bool,
	delete_request_ts: Option<i64>,
	delete_complete_ts: Option<i64>,
}

#[operation(name = "user-get")]
pub async fn handle(
	ctx: OperationContext<user::get::Request>,
) -> GlobalResult<user::get::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let users = sqlx::query_as::<_, User>(indoc!(
		"
		SELECT
			user_id,
			display_name,
			account_number,
			avatar_id,
			profile_id,
			join_ts,
			bio,
			is_admin,
			delete_request_ts,
			delete_complete_ts
		FROM db_user.users
		WHERE user_id = ANY($1)
		"
	))
	.bind(user_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	let upload_ids = users
		.iter()
		.filter_map(|user| user.profile_id.map(Into::into))
		.collect::<Vec<_>>();

	let (upload_res, files_res) = tokio::try_join!(
		op!([ctx] upload_get {
			upload_ids: upload_ids.clone(),
		}),
		op!([ctx] upload_file_list {
			upload_ids: upload_ids.clone(),
		})
	)?;

	Ok(user::get::Response {
		users: users
			.into_iter()
			.map(|user| {
				let profile_id = user.profile_id.map(Into::<common::Uuid>::into);

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

				backend::user::User {
					user_id: Some(user.user_id.into()),
					display_name: user.display_name,
					account_number: user.account_number as u32,
					avatar_id: user.avatar_id,
					profile_upload_id: if profile_upload_complete_ts.is_some() {
						profile_id
					} else {
						None
					},
					profile_file_name,
					profile_provider,
					join_ts: user.join_ts,
					bio: user.bio,
					is_admin: user.is_admin,
					delete_request_ts: user.delete_request_ts,
					delete_complete_ts: user.delete_complete_ts,
				}
			})
			.collect::<Vec<_>>(),
	})
}
