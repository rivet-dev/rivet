use std::{collections::HashMap, net::IpAddr};

use chirp_workflow::prelude::*;
use nomad_client::models::*;
use rivet_operation::prelude::proto::backend::{self, pkg::*};
use serde_json::json;

use crate::utils;

/// Tracks which resources should be prewarmed in the ATS cache.
#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
	pub build_ids: Vec<Uuid>,
}

struct Build {
	build_id: Uuid,
	path: String,
}

#[derive(sqlx::FromRow)]
struct VlanIp {
	datacenter_id: Uuid,
	vlan_ip: IpAddr,
}

/// Requests resources from the ATS cache to make sure any subsequent requests will be faster.
///
/// This works by scheduling a Nomad job that requests the given artifacts and exits immediately.
/// Under the hood, this will:
///
/// 1. Schedule a Nomad job with artifact requests for the resources
/// 2. Nomad will make a request for the given artifact to ATS
/// 3. ATS will check the cache for the artifact and, if not found, request it from the S3 origin
///
/// Next time the artifact is requested (i.e. on a new server created), it will
/// already be in the cache.
#[operation]
pub async fn prewarm_ats(ctx: &OperationCtx, input: &Input) -> GlobalResult<()> {
	// Get all vlan ips
	let (vlan_ips, builds) = tokio::try_join!(
		sql_fetch_all!(
			[ctx, VlanIp]
			"
			SELECT
				datacenter_id, vlan_ip
			FROM db_cluster.servers
			WHERE
				datacenter_id = ANY($1) AND
				pool_type2 = $2 AND
				vlan_ip IS NOT NULL AND
				drain_ts IS NULL AND
				cloud_destroy_ts IS NULL
			",
			input.datacenter_ids.clone(),
			serde_json::to_string(&cluster::types::PoolType::Ats)?,
		),
		async {
			let builds_res = ctx
				.op(crate::ops::get::Input {
					build_ids: input.build_ids.clone(),
				})
				.await?;

			let uploads_res = op!([ctx] upload_get {
				upload_ids: builds_res
					.builds
					.iter()
					.map(|build| build.upload_id.into())
					.collect(),
			})
			.await?;

			builds_res
				.builds
				.iter()
				.map(|build| {
					let proto_upload_id = Some(build.upload_id.into());
					let upload = unwrap!(uploads_res
						.uploads
						.iter()
						.find(|upload| upload.upload_id == proto_upload_id));

					// Parse provider
					let proto_provider = unwrap!(
						backend::upload::Provider::from_i32(upload.provider),
						"invalid upload provider"
					);
					let provider = match proto_provider {
						backend::upload::Provider::Minio => s3_util::Provider::Minio,
						backend::upload::Provider::Backblaze => s3_util::Provider::Backblaze,
						backend::upload::Provider::Aws => s3_util::Provider::Aws,
					};

					// Build path
					let path = format!(
						"/s3-cache/{provider}/{namespace}-bucket-build/{upload_id}/{file_name}",
						provider = heck::KebabCase::to_kebab_case(provider.as_str()),
						namespace = util::env::namespace(),
						upload_id = build.upload_id,
						file_name = utils::file_name(build.kind, build.compression),
					);

					Ok(Build {
						build_id: build.build_id,
						path,
					})
				})
				.collect::<GlobalResult<Vec<_>>>()
		},
	)?;

	let job_spec_json = serde_json::to_string(&gen_prewarm_job(input.build_ids.len())?)?;

	for datacenter_id in &input.datacenter_ids {
		let mut vlan_ips_in_region = vlan_ips
			.iter()
			.filter(|row| &row.datacenter_id == datacenter_id);
		let vlan_ip_count = vlan_ips_in_region.clone().count() as i64;

		if vlan_ip_count == 0 {
			continue;
		}

		// Pass artifact URLs to the job
		let parameters = builds
			.iter()
			.enumerate()
			.map(|(i, build)| {
				let build_id_hash = utils::build_hash(build.build_id) as i64;

				// NOTE: The algorithm here for deterministically choosing the vlan ip should match the one
				// used in the SQL statement in mm-lobby-create @ resolve_image_artifact_url
				let idx = (build_id_hash % vlan_ip_count.max(1)).unsigned_abs() as usize;
				let vlan_ip = &unwrap!(vlan_ips_in_region.nth(idx), "no vlan ip").vlan_ip;

				Ok(backend::job::Parameter {
					key: format!("artifact_url_{i}"),
					value: format!("http://{vlan_ip}:8080{}", &build.path),
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?;

		// Run the job and forget about it
		let run_id = Uuid::new_v4();
		msg!([ctx] job_run::msg::create(run_id) {
			run_id: Some(run_id.into()),
			region_id: Some((*datacenter_id).into()),
			parameters: parameters,
			job_spec_json: job_spec_json.clone(),
			..Default::default()
		})
		.await?;
	}

	Ok(())
}

/// Generates a Nomad job that will fetch the required assets then exit immediately.
///
/// This uses our job-run infrastructure to reuse dispatched jobs. This will generate a unique job
/// for every requested artifact count.
fn gen_prewarm_job(artifact_count: usize) -> GlobalResult<nomad_client::models::Job> {
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
			name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
			networks: Some(vec![NetworkResource {
				mode: Some("host".into()),
				..NetworkResource::new()
			}]),
			ephemeral_disk: Some(Box::new(EphemeralDisk {
				// This is only used for scheduling, not enforced for download size
				//
				// https://developer.hashicorp.com/nomad/docs/job-specification/ephemeral_disk#size
				size_mb: Some(2048),
				..EphemeralDisk::new()
			})),
			// This task will do nothing and exit immediately. The artifacts are prewarming the
			// cache for us.
			tasks: Some(vec![Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("exec".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("command".into(), json!("/bin/sh"));
					config.insert("args".into(), json!(["-c", "exit 0"]));
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
