use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Debug, sqlx::FromRow)]
struct UploadRow {
	user_id: Uuid,
	upload_id: Uuid,
	create_ts: i64,
}

#[operation(name = "upload-list-for-user")]
async fn handle(
	ctx: OperationContext<upload::list_for_user::Request>,
) -> GlobalResult<upload::list_for_user::Response> {
	let crdb = ctx.crdb("db-upload").await?;

	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();
	let limit = ctx.limit as i64;

	let uploads = if let Some(anchor) = ctx.anchor {
		sqlx::query_as::<_, UploadRow>(indoc!(
			"
			SELECT user_id, upload_id, create_ts
			FROM uploads
			WHERE
				user_id = ANY($1) AND
				create_ts > $2
			ORDER BY create_ts DESC
			LIMIT $3
			"
		))
		.bind(&user_ids)
		.bind(anchor)
		.bind(limit)
		.fetch_all(&crdb)
		.await?
	} else {
		sqlx::query_as::<_, UploadRow>(indoc!(
			"
			SELECT user_id, upload_id, create_ts
			FROM uploads
			WHERE
				user_id = ANY($1)
			ORDER BY create_ts DESC
			LIMIT $2
			"
		))
		.bind(&user_ids)
		.bind(limit)
		.fetch_all(&crdb)
		.await?
	};

	let users = user_ids
		.into_iter()
		.map(|user_id| {
			let uploads = uploads
				.iter()
				.filter(|upload| upload.user_id == user_id)
				.collect::<Vec<_>>();

			upload::list_for_user::response::User {
				user_id: Some(user_id.into()),
				upload_ids: uploads
					.iter()
					.map(|upload| (upload.upload_id).into())
					.collect::<Vec<_>>(),
				anchor: uploads.last().map(|upload| upload.create_ts),
			}
		})
		.collect::<Vec<_>>();

	Ok(upload::list_for_user::Response { users })
}
