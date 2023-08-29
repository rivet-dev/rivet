use proto::backend;
use rivet_operation::prelude::*;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct BuildMetadata {
	pub image_tag: Option<String>,
}

#[tracing::instrument(skip_all)]
pub async fn run(
	pools: rivet_pools::Pools,
	ctx: OperationContext<()>,
	game_id: Uuid,
) -> GlobalResult<()> {
	let mut build_meta = BuildMetadata::default();

	// Fetch game ns
	let ns_res = op!([ctx] game_namespace_list {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let ns_ids = internal_unwrap_owned!(ns_res.games.first())
		.namespace_ids
		.clone();

	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: ns_ids.clone(),
	})
	.await?;
	let ns = internal_unwrap_owned!(ns_res.namespaces.iter().find(|x| x.name_id == "prod"));
	tracing::info!(?ns, "selected namespace");
	let version_id = internal_unwrap_owned!(ns.version_id);

	// Fetch version
	let version_res = op!([ctx] cloud_version_get {
		version_ids: vec![version_id],
	})
	.await?;
	let version = internal_unwrap_owned!(version_res.versions.first());
	let version_config = internal_unwrap!(version.config);

	// Prepare package
	let output_path = PathBuf::from(format!("/tmp/rivet-{}", version_id.as_uuid()));
	fs::create_dir_all(&output_path).await?;

	// Archive config
	tracing::info!("archiving config");
	let mut version_config_buf = Vec::with_capacity(prost::Message::encoded_len(version_config));
	prost::Message::encode(version_config, &mut version_config_buf)?;
	tokio::fs::write(output_path.join("version_config"), &version_config_buf).await?;

	// Pull build
	if let Some(matchmaker) = &version_config.matchmaker {
		let s3_client = s3_util::Client::from_env("bucket-build").await?;

		let lobby_group = internal_unwrap_owned!(matchmaker.lobby_groups.first());
		let backend::matchmaker::LobbyRuntime {
			runtime: Some(backend::matchmaker::lobby_runtime::Runtime::Docker(docker)),
		} = internal_unwrap!(lobby_group.runtime)
		else {
			internal_panic!("invalid runtime");
		};
		let build_id = internal_unwrap_owned!(docker.build_id);

		tracing::info!(?build_id, "build id");
		let build_res = op!([ctx] build_get {
			build_ids: vec![build_id],
		})
		.await?;
		let build = internal_unwrap_owned!(build_res.builds.first());
		let upload_id = internal_unwrap_owned!(build.upload_id);

		build_meta.image_tag = Some(build.image_tag.clone());

		// Download image
		let res = s3_client
			.get_object()
			.bucket(s3_client.bucket())
			.key(format!("{}/image.tar", upload_id))
			.send()
			.await?;
		let mut stream = res.body.into_async_read();

		// Write image to file
		let mut file = fs::File::create(output_path.join("image.tar")).await?;
		tokio::io::copy(&mut stream, &mut file).await?;
	}

	// Write metadata
	let meta_json = serde_json::to_vec(&build_meta)?;
	tokio::fs::write(output_path.join("meta.json"), &meta_json).await?;

	tracing::info!(path = %output_path.display(), "successfully wrote version");

	Ok(())
}
