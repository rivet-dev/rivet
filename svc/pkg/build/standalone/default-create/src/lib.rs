use indoc::indoc;
use proto::backend;
use rivet_operation::prelude::*;
use uuid::Uuid;

const DEFAULT_BUILDS: &[DefaultBuildConfig] = &[
	DefaultBuildConfig {
		kind: "game-multiplayer",
		tag: include_str!(
			"../../../../../../infra/default-builds/outputs/game-multiplayer-tag.txt"
		),
		tar: include_bytes!("../../../../../../infra/default-builds/outputs/game-multiplayer.tar"),
	},
	DefaultBuildConfig {
		kind: "test-fail-immediately",
		tag: include_str!(
			"../../../../../../infra/default-builds/outputs/test-fail-immediately-tag.txt"
		),
		tar: include_bytes!(
			"../../../../../../infra/default-builds/outputs/test-fail-immediately.tar"
		),
	},
	DefaultBuildConfig {
		kind: "test-hang-indefinitely",
		tag: include_str!(
			"../../../../../../infra/default-builds/outputs/test-hang-indefinitely-tag.txt"
		),
		tar: include_bytes!(
			"../../../../../../infra/default-builds/outputs/test-hang-indefinitely.tar"
		),
	},
	DefaultBuildConfig {
		kind: "test-mm-lobby-ready",
		tag: include_str!(
			"../../../../../../infra/default-builds/outputs/test-mm-lobby-ready-tag.txt"
		),
		tar: include_bytes!(
			"../../../../../../infra/default-builds/outputs/test-mm-lobby-ready.tar"
		),
	},
	DefaultBuildConfig {
		kind: "test-mm-lobby-echo",
		tag: include_str!(
			"../../../../../../infra/default-builds/outputs/test-mm-lobby-echo-tag.txt"
		),
		tar: include_bytes!(
			"../../../../../../infra/default-builds/outputs/test-mm-lobby-echo.tar"
		),
	},
	DefaultBuildConfig {
		kind: "test-mm-player-connect",
		tag: include_str!(
			"../../../../../../infra/default-builds/outputs/test-mm-player-connect-tag.txt"
		),
		tar: include_bytes!(
			"../../../../../../infra/default-builds/outputs/test-mm-player-connect.tar"
		),
	},
];

struct DefaultBuildConfig {
	/// The kind of default build.
	kind: &'static str,
	/// Tag for the image that's archived.
	tag: &'static str,
	/// Bytes for the image that needs to be uploaded.
	tar: &'static [u8],
}

#[tracing::instrument]
pub async fn run_from_env() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("build-default-create").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("build-default-create");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"build-default-create".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);
	let crdb_pool = ctx.crdb().await?;

	for build in DEFAULT_BUILDS {
		// Check if this default build is already set
		let old_default_build = sqlx::query_as::<_, (String,)>(
			"SELECT image_tag FROM db_build.default_builds WHERE kind = $1",
		)
		.bind(build.kind)
		.fetch_optional(&crdb_pool)
		.await?;
		if old_default_build
			.as_ref()
			.map_or(false, |(old_image_tag,)| old_image_tag == build.tag)
		{
			tracing::info!(
				?old_default_build,
				"build already matches the given tag, skipping"
			);
			continue;
		}

		// Upload the build
		tracing::info!(tag = %build.tag, "uploading new build");
		let upload_id = upload_build(&ctx, build).await?;

		// Update default build
		tracing::info!(tag = %build.tag, ?upload_id, "setting default build");
		sql_query!(
			[ctx]
			"
			UPSERT INTO db_build.default_builds (kind, image_tag, upload_id)
			VALUES ($1, $2, $3)
			",
			build.kind,
			build.tag,
			upload_id,
		)
		.await?;
	}

	Ok(())
}

async fn upload_build(
	ctx: &OperationContext<()>,
	build: &DefaultBuildConfig,
) -> GlobalResult<Uuid> {
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-build".into(),
		files: vec![
			backend::upload::PrepareFile {
				path: "image.tar".into(),
				content_length: build.tar.len() as u64,
				multipart: true,
				..Default::default()
			},
		],
	})
	.await?;
	let upload_id = unwrap_ref!(upload_prepare_res.upload_id).as_uuid();

	for req in &upload_prepare_res.presigned_requests {
		let start = req.byte_offset as usize;
		let end = (req.byte_offset + req.content_length) as usize;

		let part = &build.tar[start..end];

		let url = &req.url;
		tracing::info!(%url, part=%req.part_number, "uploading file");
		let res = reqwest::Client::new()
			.put(url)
			.header(reqwest::header::CONTENT_LENGTH, part.len() as u64)
			.body(reqwest::Body::from(part))
			.send()
			.await?;

		if !res.status().is_success() {
			let status = res.status();
			let body = res.text().await?;
			tracing::warn!(?status, ?body, "failure uploading");
			bail!("failure uploading");
		}
	}

	tracing::info!("successfully uploaded");

	// Complete the upload
	op!([ctx] upload_complete {
		upload_id: Some(upload_id.into()),
		bucket: Some("bucket-build".into()),
	})
	.await?;

	Ok(upload_id)
}
