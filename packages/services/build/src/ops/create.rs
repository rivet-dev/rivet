use std::collections::HashMap;

use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend;

const MAX_UPLOAD_SIZE: u64 = util::file_size::gigabytes(8);
use crate::{
	types::{upload::PrepareFile, upload::PresignedUploadRequest, BuildCompression, BuildKind},
	utils,
};

#[derive(Debug)]
pub struct Input {
	pub game_id: Option<Uuid>,
	pub env_id: Option<Uuid>,
	pub display_name: String,
	pub content: Content,
	pub kind: BuildKind,
	pub compression: BuildCompression,
	pub tags: HashMap<String, String>,
}

#[derive(Debug)]
pub enum Content {
	New {
		image_file: PrepareFile,
		image_tag: String,
	},
	Default {
		build_kind: String,
	},
}

#[derive(Debug)]
pub struct Output {
	pub build_id: Uuid,
	pub upload_id: Uuid,
	pub presigned_requests: Vec<PresignedUploadRequest>,
}

#[operation]
pub async fn get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	ensure!(
		util::check::display_name_long(&input.display_name),
		"invalid display name"
	);

	// Validate game exists
	if let Some(game_id) = input.game_id {
		let game_res = op!([ctx] game_get {
			game_ids: vec![game_id.into()],
		})
		.await?;
		let game = game_res.games.first();
		ensure!(game.is_some(), "game not found");
	}
	if let Some(env_id) = input.env_id {
		let env_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![env_id.into()],
		})
		.await?;
		let env = env_res.namespaces.first();
		ensure!(env.is_some(), "game not found");
	}

	let (image_tag, upload_id, presigned_requests) = match &input.content {
		Content::Default { build_kind } => {
			let default_build_row = sql_fetch_optional!(
				[ctx, (String, Uuid)]
				"SELECT image_tag, upload_id FROM db_build.default_builds WHERE kind = $1",
				build_kind,
			)
			.await?;

			let (image_tag, upload_id) =
				unwrap!(default_build_row, "default build missing: {build_kind}");

			(image_tag, upload_id, Vec::new())
		}
		Content::New {
			image_file,
			image_tag,
		} => {
			let tag_split = image_tag.split_once(':');
			let (tag_base, tag) = unwrap_ref!(tag_split, "missing separator in image tag");
			ensure!(
				util::check::docker_ident(tag_base),
				"invalid image tag base"
			);
			ensure!(util::check::docker_ident(tag), "invalid tag");

			ensure_with!(
				image_file.content_length < MAX_UPLOAD_SIZE,
				UPLOAD_TOO_LARGE
			);

			// Check if build is unique
			let (build_exists,) = sql_fetch_one!(
				[ctx, (bool,)]
				"SELECT EXISTS (SELECT 1 FROM db_build.builds WHERE image_tag = $1)",
				image_tag,
			)
			.await?;
			if build_exists {
				bail!("build image tag not unique: {image_tag:?}");
			} else {
				tracing::debug!(?image_tag, "build image is unique");
			}

			// Create the upload
			let file_name = utils::file_name(input.kind, input.compression);
			let upload_prepare_res = op!([ctx] upload_prepare {
				bucket: "bucket-build".into(),
				files: vec![
					backend::upload::PrepareFile {
						path: file_name,
						content_length: image_file.content_length,
						..Default::default()
					},
				],
			})
			.await?;
			let upload_id = unwrap_ref!(upload_prepare_res.upload_id).as_uuid();

			(
				image_tag.clone(),
				upload_id,
				upload_prepare_res.presigned_requests.clone(),
			)
		}
	};

	// Create build
	let build_id = Uuid::new_v4();
	sql_execute!(
		[ctx]
		"
		INSERT INTO
			db_build.builds (
				build_id,
				game_id,
				env_id,
				upload_id,
				display_name,
				image_tag,
				create_ts,
				kind,
				compression
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9)
		",
		build_id,
		input.game_id,
		input.env_id,
		upload_id,
		&input.display_name,
		image_tag,
		ctx.ts(),
		input.kind as i32,
		input.compression as i32,
	)
	.await?;

	Ok(Output {
		build_id,
		upload_id,
		presigned_requests: presigned_requests.into_iter().map(Into::into).collect(),
	})
}
