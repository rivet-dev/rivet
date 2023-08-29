use super::pull_game::BuildMetadata;
use proto::backend;
use rivet_operation::prelude::*;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

#[tracing::instrument(skip_all)]
pub async fn run(
	pools: rivet_pools::Pools,
	ctx: OperationContext<()>,
	game_id: Uuid,
	input_path: &Path,
) -> GlobalResult<()> {
	let crdb_build = pools.crdb("db-build")?;

	let new_version_name = util::timestamp::now().to_string();

	// Validate game exists
	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let _game = internal_unwrap_owned!(game_res.games.first());

	let regions_res = op!([ctx] region_list {}).await?;
	let mut region_ids = regions_res
		.region_ids
		.iter()
		.map(|x| x.as_uuid())
		.collect::<Vec<_>>();
	region_ids.sort();
	let primary_region_id = *internal_unwrap_owned!(region_ids.first());

	// Decode config
	tracing::info!("decoding config");
	let version_config_buf = tokio::fs::read(input_path.join("version_config")).await?;
	let mut version_config =
		<backend::cloud::VersionConfig as prost::Message>::decode(version_config_buf.as_slice())?;

	// TODO: Add site support
	version_config.cdn = None;

	// Decode meta
	tracing::info!("decoding metadata");
	let build_meta_buf = tokio::fs::read(input_path.join("meta.json")).await?;
	let build_meta = serde_json::from_slice::<BuildMetadata>(&build_meta_buf)?;

	// Push build
	if let Some(matchmaker) = &mut version_config.matchmaker {
		let build_path = input_path.join("image.tar");
		let image_tag = internal_unwrap_owned!(build_meta.image_tag.clone());

		let build_row =
			sqlx::query_as::<_, (Uuid,)>("SELECT build_id FROM builds WHERE image_tag = $1")
				.bind(&image_tag)
				.fetch_optional(&crdb_build)
				.await?;
		let build_id = if let Some((build_id,)) = build_row {
			build_id
		} else {
			// Read file metadata
			let file_meta = fs::metadata(&build_path).await?;
			let content_length = file_meta.len();

			// Create build
			let build_res = op!([ctx] build_create {
				game_id: Some(game_id.into()),
				display_name: new_version_name.clone(),
				image_tag: Some(image_tag.clone()),
				image_file: Some(backend::upload::PrepareFile {
					path: "image.tar".into(),
					mime: Some("application/x-tar".into()),
					content_length,
					..Default::default()
				}),
			})
			.await?;
			let build_id = internal_unwrap_owned!(build_res.build_id).as_uuid();
			let image_presigned_request =
				internal_unwrap_owned!(build_res.image_presigned_requests.first());

			// Upload image
			reqwest::Client::new()
				.put(&image_presigned_request.url)
				.header(reqwest::header::CONTENT_TYPE, "application/x-tar")
				.header(reqwest::header::CONTENT_LENGTH, content_length)
				.body(tokio::fs::File::open(&build_path).await?)
				.send()
				.await?
				.error_for_status()?;

			op!([ctx] upload_complete {
				upload_id: build_res.upload_id,
				bucket: Some("bucket-build".into()),
			})
			.await?;

			build_id
		};

		// Update config
		for lobby_group in &mut matchmaker.lobby_groups {
			// Replace regions with just one region and a smaller tier
			let mut primary_region = internal_unwrap_owned!(lobby_group.regions.first()).clone();
			primary_region.region_id = Some(primary_region_id.into());
			primary_region.tier_name_id = "basic-1d1".into();
			lobby_group.regions = vec![primary_region];

			// Override the build with the new build
			let Some(backend::matchmaker::LobbyRuntime {
				runtime: Some(backend::matchmaker::lobby_runtime::Runtime::Docker(docker)),
			}) = lobby_group.runtime.as_mut()
			else {
				internal_panic!("invalid runtime")
			};
			docker.build_id = Some(build_id.into());

			for port in &mut docker.ports {
				if port.port_range.is_some() {
					port.proxy_kind = backend::matchmaker::lobby_runtime::ProxyKind::None as i32;
				}
			}
		}
	}

	// Publish version
	op!([ctx] cloud_version_publish {
		game_id: Some(game_id.into()),
		display_name: new_version_name.clone(),
		config: Some(version_config),
	})
	.await?;

	Ok(())
}
