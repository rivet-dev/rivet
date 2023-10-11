use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use std::time::Instant;
use tracing::Instrument;

#[operation(name = "mm-config-version-prepare")]
async fn handle(
	ctx: OperationContext<mm_config::version_prepare::Request>,
) -> GlobalResult<mm_config::version_prepare::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();
	let config = internal_unwrap!(ctx.config);

	let mut lobby_group_ctxs = Vec::new();
	for lobby_group in &config.lobby_groups {
		// Validate regions
		internal_assert!(!lobby_group.regions.is_empty(), "no regions provided");
		let region_ids = lobby_group
			.regions
			.iter()
			.flat_map(|r| r.region_id)
			.collect::<Vec<_>>();

		// Resolve region name IDs
		let (regions_res, tier_res) = tokio::try_join!(
			op!([ctx] region_get {
				region_ids: region_ids.clone(),
			}),
			op!([ctx] tier_list {
				region_ids: region_ids.clone(),
			}),
		)?;

		// Validate regions exist
		for region in &lobby_group.regions {
			let has_region = regions_res
				.regions
				.iter()
				.any(|x| x.region_id == region.region_id);
			internal_assert!(has_region, "invalid region id");
		}

		// Prepare runtime
		let runtime = internal_unwrap!(lobby_group.runtime);
		let runtime = internal_unwrap!(runtime.runtime);
		let runtime_ctx = prepare_runtime(
			&ctx,
			game_id,
			runtime,
			&lobby_group.regions,
			&regions_res.regions,
			&tier_res.regions,
		)
		.await?;

		lobby_group_ctxs.push(backend::matchmaker::LobbyGroupCtx {
			runtime: Some(runtime_ctx),
		});
	}

	Ok(mm_config::version_prepare::Response {
		config_ctx: Some(backend::matchmaker::VersionConfigCtx {
			lobby_groups: lobby_group_ctxs,
		}),
	})
}

async fn prepare_runtime(
	ctx: &OperationContext<mm_config::version_prepare::Request>,
	game_id: Uuid,
	runtime: &backend::matchmaker::lobby_runtime::Runtime,
	lg_regions: &[backend::matchmaker::lobby_group::Region],
	regions_data: &[backend::region::Region],
	tier_regions: &[tier::list::response::Region],
) -> GlobalResult<backend::matchmaker::LobbyRuntimeCtx> {
	let runtime: backend::matchmaker::LobbyRuntimeCtx = match runtime {
		backend::matchmaker::lobby_runtime::Runtime::Docker(runtime) => {
			let build_id = internal_unwrap!(runtime.build_id).as_uuid();
			let _ = validate_build(ctx, game_id, build_id).await?;

			// Validate regions
			for lg_region in lg_regions {
				// Validate region
				internal_assert!(
					regions_data
						.iter()
						.any(|x| x.region_id == lg_region.region_id),
					"invalid region id"
				);

				// Validate tier
				let tier_region = internal_unwrap_owned!(tier_regions
					.iter()
					.find(|x| x.region_id == lg_region.region_id));
				internal_assert!(
					tier_region
						.tiers
						.iter()
						.any(|x| x.tier_name_id == lg_region.tier_name_id),
					"invalid tier name id"
				);
			}

			backend::matchmaker::lobby_runtime_ctx::Docker {
				#[allow(deprecated)]
				job_template_id: None,
			}
			.into()
		}
	};

	Ok(runtime)
}

async fn validate_build(
	ctx: &OperationContext<mm_config::version_prepare::Request>,
	game_id: Uuid,
	build_id: Uuid,
) -> GlobalResult<(Uuid, String)> {
	// Validate build exists and belongs to this game
	let build_get = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await?;
	let build = internal_unwrap_owned!(build_get.builds.first(), "build not found");
	let build_upload_id = internal_unwrap!(build.upload_id).as_uuid();
	let build_game_id = internal_unwrap!(build.game_id).as_uuid();
	internal_assert_eq!(game_id, build_game_id);

	tracing::info!(?build);

	// Validate build has completed uploading
	let upload_res = op!([ctx] upload_get {
		upload_ids: vec![build_upload_id.into()],
	})
	.await?;
	let upload = internal_unwrap_owned!(upload_res.uploads.first(), "build upload not found");
	let upload_id = internal_unwrap!(upload.upload_id).as_uuid();
	internal_assert!(upload.complete_ts.is_some(), "build upload is not complete");

	// Parse provider
	let proto_provider = internal_unwrap_owned!(
		backend::upload::Provider::from_i32(upload.provider),
		"invalid upload provider"
	);
	let provider = match proto_provider {
		backend::upload::Provider::Minio => s3_util::Provider::Minio,
		backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
		backend::upload::Provider::Aws => s3_util::Provider::Aws,
	};

	// // prewarm ATS cache
	// let path = format!(
	// 	"/s3-cache/{provider}/{namespace}-bucket-build/{upload_id}/image.tar",
	// 	provider = heck::KebabCase::to_kebab_case(provider.to_string().as_str()),
	// 	namespace = util::env::namespace(),
	// );
	// prewarm_ats_cache(&path).await;

	Ok((upload_id, build.image_tag.clone()))
}

// // TODO: Validate ports are within a given range and don't use our reserved ports, since we need a reserved port range for Rivet sidecars to run

// /// Prewarms the ATS cache with the given resources in order to make the future requests faster.
// #[tracing::instrument]
// async fn prewarm_ats_cache(path: &str) {
// 	// TODO: This is hardcoded
// 	let ats_urls = vec!["10.0.25.2", "10.0.50.2"];
// 	for ats_url in &ats_urls {
// 		let url = format!("http://{ats_url}:9300{path}");
// 		let spawn_res = tokio::task::Builder::new()
// 			.name("mm_config_version_prepare::prewarm_ats_cache")
// 			.spawn(
// 				async move {
// 					match prewarm_ats_cache_inner(&url).await {
// 						Ok(_) => {}
// 						Err(err) => {
// 							tracing::error!(%err, "failed to prewarm ats cache");
// 						}
// 					}
// 				}
// 				.in_current_span(),
// 			);
// 		if let Err(err) = spawn_res {
// 			tracing::error!(?err, "failed to spawn prewarm_ats_cache_inner task");
// 		}
// 	}
// }

// /// Prewarm a specific ATS server with a resource.
// #[tracing::instrument]
// async fn prewarm_ats_cache_inner(url: &str) -> GlobalResult<()> {
// 	tracing::info!(?url, "populating ats build cache");

// 	let client = reqwest::Client::new();

// 	// Check if cache already prewarmed
// 	let start = Instant::now();
// 	let resp = client.head(url).send().await?;
// 	let age = internal_unwrap!(resp.headers().get("Age"))
// 		.to_str()?
// 		.parse::<u64>()?;
// 	if age > 0 {
// 		tracing::info!(?age, "object already cached");
// 		return Ok(());
// 	}
// 	tracing::info!(head_duration = ?start.elapsed(), "fetching object");

// 	// Make a GET request to prewarm the cache and do nothing with the response
// 	let start = Instant::now();
// 	let resp = client.get(url).send().await?.error_for_status()?;
// 	tracing::info!(get_duration = ?start.elapsed(), "cache prewarmd");

// 	Ok(())
// }
