use anyhow::*;
use futures_util::stream::{StreamExt, TryStreamExt};
use rivet_api::{apis, models};
use serde::Serialize;
use std::{path::PathBuf, sync::Arc};
use tokio::fs;
use uuid::Uuid;

use crate::{
	config,
	project::environment::TEMPEnvironment,
	toolchain_ctx::ToolchainCtx,
	util::{net::upload, task, term},
};

pub struct PushOpts {
	pub env: TEMPEnvironment,

	/// Path to already created tar.
	pub path: PathBuf,

	/// Docker inside the image.
	pub docker_tag: String,

	pub bundle: config::build::docker::BundleKind,

	pub compression: config::build::Compression,
}

#[derive(Serialize)]
pub struct PushOutput {
	pub build_id: Uuid,
}

pub async fn push_tar(
	ctx: &ToolchainCtx,
	task: task::TaskCtx,
	push_opts: &PushOpts,
) -> Result<PushOutput> {
	let reqwest_client = Arc::new(reqwest::Client::new());

	// Inspect the image
	let image_file_meta = fs::metadata(&push_opts.path)
		.await
		.with_context(|| anyhow!("failed to open image file: {}", push_opts.path.display()))?;
	ensure!(image_file_meta.len() > 0, "docker image archive is empty");

	let content_type = "binary/octet-stream";

	task.log(format!(
		"[Uploading] ({size})",
		size = upload::format_file_size(image_file_meta.len())?
	));

	let build_kind = match push_opts.bundle {
		config::build::docker::BundleKind::DockerImage => models::ActorBuildKind::DockerImage,
		config::build::docker::BundleKind::OciBundle => models::ActorBuildKind::OciBundle,
	};

	let build_compression = match push_opts.compression {
		config::build::Compression::None => models::ActorBuildCompression::None,
		config::build::Compression::Lz4 => models::ActorBuildCompression::Lz4,
	};

	let build_res = apis::actor_builds_api::actor_builds_prepare(
		&ctx.openapi_config_cloud,
		models::ActorPrepareBuildRequest {
			image_tag: Some(push_opts.docker_tag.clone()),
			image_file: Box::new(models::UploadPrepareFile {
				path: crate::util::build::file_name(build_kind, build_compression),
				content_type: Some(content_type.into()),
				content_length: image_file_meta.len() as i64,
			}),
			kind: Some(build_kind),
			compression: Some(build_compression),
		},
		Some(&ctx.project.name_id),
		Some(&push_opts.env.slug),
	)
	.await;
	if let Err(err) = build_res.as_ref() {
		task.log(format!("{err:?}"))
	}
	let build_res = build_res.context("build_res")?;
	let build_id = build_res.build;
	let pb = term::EitherProgressBar::Multi(term::multi_progress_bar(task.clone()));

	// Upload chunks in parallel
	futures_util::stream::iter(build_res.presigned_requests)
		.map(|presigned_request| {
			let task = task.clone();
			let reqwest_client = reqwest_client.clone();
			let pb = pb.clone();

			async move {
				upload::upload_file(
					task.clone(),
					&reqwest_client,
					&presigned_request,
					&push_opts.path,
					Some(content_type),
					pb,
				)
				.await
			}
		})
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	let complete_res = apis::actor_builds_api::actor_builds_complete(
		&ctx.openapi_config_cloud,
		&build_res.build.to_string(),
		Some(&ctx.project.name_id),
		Some(&push_opts.env.slug),
	)
	.await;
	if let Err(err) = complete_res.as_ref() {
		task.log(format!("{err:?}"));
	}
	complete_res.context("complete_res")?;

	Ok(PushOutput {
		build_id: build_id.to_owned(),
	})
}
