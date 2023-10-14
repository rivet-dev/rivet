use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tracing::Instrument;

#[operation(name = "mm-config-version-prepare")]
async fn handle(
	ctx: OperationContext<mm_config::version_prepare::Request>,
) -> GlobalResult<mm_config::version_prepare::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();
	let config = internal_unwrap!(ctx.config);

	// Deduplicated list of build paths that will be used to prewarm the ATS cache
	let mut all_region_ids = HashSet::new();
	let mut prewarm_ats_paths = HashSet::new();

	let mut lobby_group_ctxs = Vec::new();
	for lobby_group in &config.lobby_groups {
		// Validate regions
		internal_assert!(!lobby_group.regions.is_empty(), "no regions provided");
		let region_ids = lobby_group
			.regions
			.iter()
			.flat_map(|r| r.region_id)
			.collect::<Vec<_>>();
		all_region_ids.extend(region_ids.iter().map(common::Uuid::as_uuid));

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
			&mut prewarm_ats_paths,
		)
		.await?;

		lobby_group_ctxs.push(backend::matchmaker::LobbyGroupCtx {
			runtime: Some(runtime_ctx),
		});
	}

	prewarm_ats_cache(ctx.chirp(), all_region_ids, prewarm_ats_paths).await?;

	Ok(mm_config::version_prepare::Response {
		config_ctx: Some(backend::matchmaker::VersionConfigCtx {
			lobby_groups: lobby_group_ctxs,
		}),
	})
}

#[tracing::instrument]
async fn prepare_runtime(
	ctx: &OperationContext<mm_config::version_prepare::Request>,
	game_id: Uuid,
	runtime: &backend::matchmaker::lobby_runtime::Runtime,
	lg_regions: &[backend::matchmaker::lobby_group::Region],
	regions_data: &[backend::region::Region],
	tier_regions: &[tier::list::response::Region],
	prewarm_ats_paths: &mut HashSet<String>,
) -> GlobalResult<backend::matchmaker::LobbyRuntimeCtx> {
	let runtime: backend::matchmaker::LobbyRuntimeCtx = match runtime {
		backend::matchmaker::lobby_runtime::Runtime::Docker(runtime) => {
			let build_id = internal_unwrap!(runtime.build_id).as_uuid();
			let _ = validate_build(ctx, game_id, build_id, prewarm_ats_paths).await?;

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

#[tracing::instrument]
async fn validate_build(
	ctx: &OperationContext<mm_config::version_prepare::Request>,
	game_id: Uuid,
	build_id: Uuid,
	prewarm_ats_paths: &mut HashSet<String>,
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

	// Generate path used to request the image
	let path = format!(
		"/s3-cache/{provider}/{namespace}-bucket-build/{upload_id}/image.tar",
		provider = heck::KebabCase::to_kebab_case(provider.as_str()),
		namespace = util::env::namespace(),
	);
	prewarm_ats_paths.insert(path.clone());

	Ok((upload_id, build.image_tag.clone()))
}

#[tracing::instrument]
async fn prewarm_ats_cache(
	client: &chirp_client::Client,
	region_ids: HashSet<Uuid>,
	prewarm_ats_paths: HashSet<String>,
) -> GlobalResult<()> {
	if prewarm_ats_paths.is_empty() {
		return Ok(());
	}

	let job_spec_json = serde_json::to_string(&gen_prewarm_job(prewarm_ats_paths.len())?)?;

	for region_id in region_ids {
		// Pass artifact URLs to the job
		let parameters = prewarm_ats_paths
			.iter()
			.enumerate()
			.map(|(i, path)| job_run::msg::create::Parameter {
				key: format!("artifact_url_{i}"),
				value: format!("http://127.0.0.1:8080{path}"),
			})
			.collect::<Vec<_>>();

		// Run the job and forget about it
		let run_id = Uuid::new_v4();
		msg!([client] job_run::msg::create(run_id) {
			run_id: Some(run_id.into()),
			region_id: Some(region_id.into()),
			parameters: parameters,
			job_spec_json: job_spec_json.clone(),
			..Default::default()
		})
		.await
		.unwrap();
	}

	Ok(())
}

/// Generates a Nomad job that will fetch the required assets then exit immediately.
///
/// This uses our job-run infrastructure to reuse dispatched jobs. This will generate a unique job
/// for every requrested artifact count.
fn gen_prewarm_job(artifact_count: usize) -> GlobalResult<nomad_client::models::Job> {
	use nomad_client::models::*;

	// Build artifact metadata
	let mut meta_required = Vec::new();
	let mut artifacts = Vec::new();
	for i in 0..artifact_count {
		meta_required.push(format!("artifact_url_{i}"));
		artifacts.push(TaskArtifact {
			getter_source: Some(format!("${{NOMAD_META_ARTIFACT_URL_{i}}}")),
			getter_mode: Some("file".into()),
			getter_options: Some({
				let mut opts = HashMap::new();
				opts.insert("archive".into(), "false".into());
				opts
			}),
			relative_dest: Some(format!("local/artifact_{i}")),
		});
	}

	Ok(Job {
		_type: Some("batch".into()),
		constraints: Some(vec![Constraint {
			l_target: Some("${node.class}".into()),
			r_target: Some("job".into()),
			operand: Some("=".into()),
		}]),
		parameterized_job: Some(Box::new(ParameterizedJobConfig {
			meta_required: Some(meta_required),
			..ParameterizedJobConfig::new()
		})),
		task_groups: Some(vec![TaskGroup {
			name: Some("prewarm".into()),
			networks: Some(vec![NetworkResource {
				mode: Some("host".into()),
				..NetworkResource::new()
			}]),
			ephemeral_disk: Some(Box::new(EphemeralDisk {
				// TODO: Fix this size
				size_mb: Some(2048),
				..EphemeralDisk::new()
			})),
			tasks: Some(vec![Task {
				name: Some("prewarm".into()),
				driver: Some("docker".into()),
				config: Some({
					// TODO: Make this a null exec driver
					let mut config = HashMap::new();
					config.insert("image".into(), json!("alpine"));
					config.insert("entrypoint".into(), json!(["true"]));
					config
				}),
				resources: Some(Box::new(Resources {
					CPU: Some(100),
					memory_mb: Some(64),
					..Resources::new()
				})),
				artifacts: Some(artifacts),
				..Task::new()
			}]),
			..TaskGroup::new()
		}]),
		..Job::new()
	})
}
