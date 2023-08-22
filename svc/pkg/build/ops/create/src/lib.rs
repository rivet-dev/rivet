use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

const MAX_UPLOAD_SIZE: u64 = util::file_size::gigabytes(8);

#[operation(name = "build-create")]
async fn handle(
	ctx: OperationContext<build::create::Request>,
) -> GlobalResult<build::create::Response> {
	let crdb = ctx.crdb("db-build").await?;

	let game_id = **internal_unwrap!(ctx.game_id);
	internal_assert!(
		util::check::display_name_long(&ctx.display_name),
		"invalid display name"
	);

	// Validate game exists
	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = game_res.games.first();
	internal_assert!(game.is_some(), "game not found");

	let (image_tag, upload_id, image_presigned_requests) =
		if let Some(build_kind) = &ctx.default_build_kind {
			let (image_tag, upload_id) = sqlx::query_as::<_, (String, Uuid)>(
				"SELECT image_tag, upload_id FROM default_builds WHERE kind = $1",
			)
			.bind(build_kind)
			.fetch_one(&crdb)
			.await?;

			(image_tag, upload_id, Vec::new())
		} else {
			let image_file = internal_unwrap!(ctx.image_file);
			let image_tag = internal_unwrap!(ctx.image_tag);

			let tag_split = image_tag.split_once(':');
			let (tag_base, tag_tag) = internal_unwrap!(tag_split, "missing separator in image tag");
			internal_assert!(
				util::check::docker_ident(tag_base),
				"invalid image tag base"
			);
			internal_assert!(util::check::docker_ident(tag_tag), "invalid image tag tag");

			assert_with!(
				image_file.content_length < MAX_UPLOAD_SIZE,
				UPLOAD_TOO_LARGE
			);

			// Check if build is unique
			let (build_exists,) = sqlx::query_as::<_, (bool,)>(
				"SELECT EXISTS (SELECT 1 FROM builds WHERE image_tag = $1)",
			)
			.bind(image_tag)
			.fetch_one(&crdb)
			.await?;
			if build_exists {
				tracing::info!(?image_tag, "build image is not unique");
				internal_panic!("build image tag not unique");
			} else {
				tracing::info!(?image_tag, "build image is unique");
			}

			// Create the upload
			let upload_prepare_res = op!([ctx] upload_prepare {
				bucket: "bucket-build".into(),
				files: vec![
					backend::upload::PrepareFile {
						path: "image.tar".into(),
						mime: Some("application/x-tar".into()),
						content_length: image_file.content_length,
						multipart: true,
						..Default::default()
					},
				],
			})
			.await?;
			let upload_id = **internal_unwrap!(upload_prepare_res.upload_id);

			(
				image_tag.clone(),
				upload_id,
				upload_prepare_res.presigned_requests.clone(),
			)
		};

	// Create build
	let build_id = Uuid::new_v4();
	sqlx::query(indoc!(
		"
		INSERT INTO builds (build_id, game_id, upload_id, display_name, image_tag, create_ts)
		VALUES ($1, $2, $3, $4, $5, $6)
		"
	))
	.bind(build_id)
	.bind(game_id)
	.bind(upload_id)
	.bind(&ctx.display_name)
	.bind(image_tag)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	Ok(build::create::Response {
		build_id: Some(build_id.into()),
		upload_id: Some(upload_id.into()),
		image_presigned_requests,
	})
}
