use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// Tracks which resources should be prewarmed in the ATS cache.
#[derive(Debug)]
pub struct PrewarmAtsContext {
	pub region_ids: HashSet<Uuid>,
	pub paths: HashMap<String, u64>,
	#[allow(unused)]
	pub total_size: u64,
}

#[derive(sqlx::FromRow)]
struct VlanIp {
	datacenter_id: Uuid,
	vlan_ip: String,
}

/// Requests resources from the ATS cache to make sure any subsequent requests will be faster.
///
/// This is important for games that (a) don't have idle lobbies and need the lobbies to start
/// quickly and (b) use custom lobbies that need to be started as fast as possible.
///
/// This works by scheduling a Nomad job that requests the given artifacts and exits immediately.
/// Under the hood, this will:
///
/// 1. Schedule a Nomad job with artifact requests for the resources
/// 2. Nomad will make a request for the given artifact to the Envoy outbound proxy (127.0.0.1:8080)
/// 3. Envoy will use Maglev to route the request to the correct ATS cache instance (10.0.0.0/26)
/// 4. ATS will check the cache for the artifact and, if not found, request it from the S3 origin
///
/// Next time the artifact is requested (i.e. on a new lobby or custom lobby created), it will
/// already be in the cache.
#[tracing::instrument]
pub async fn prewarm_ats_cache(
	ctx: &OperationContext<mm_config::version_prepare::Request>,
	prewarm_ctx: PrewarmAtsContext,
) -> GlobalResult<()> {
	if prewarm_ctx.paths.is_empty() {
		return Ok(());
	}

	let job_spec_json = serde_json::to_string(&gen_prewarm_job(prewarm_ctx.paths.len())?)?;

	// Get all vlan ips
	let vlan_ips = sql_fetch_all!(
		[ctx, VlanIp]
		"
		SELECT
			datacenter_id, vlan_ip
		FROM db_cluster.servers
		WHERE
			datacenter_id = ANY($1) AND
			pool_type = $2 AND
			vlan_ip IS NOT NULL AND
			drain_ts IS NULL AND
			cloud_destroy_ts IS NULL
		",
		// NOTE: region_id is just the old name for datacenter_id
		prewarm_ctx.region_ids.iter().cloned().collect::<Vec<_>>(),
		backend::cluster::PoolType::Ats as i64,
	)
	.await?;

	for region_id in prewarm_ctx.region_ids {
		let mut vlan_ips_in_region = vlan_ips.iter().filter(|row| row.datacenter_id == region_id);
		let vlan_ip_count = vlan_ips_in_region.clone().count() as i64;

		ensure!(vlan_ip_count != 0, "no ats servers found");

		// Pass artifact URLs to the job
		let parameters = prewarm_ctx
			.paths
			.iter()
			.enumerate()
			.map(|(i, (path, build_id_hash))| {
				// NOTE: The algorithm here for deterministically choosing the vlan ip should match the one
				// used in the SQL statement in mm-lobby-create @ resolve_image_artifact_url
				let idx = (*build_id_hash as i64 % vlan_ip_count).abs() as usize;
				let vlan_ip = &unwrap!(vlan_ips_in_region.nth(idx), "no vlan ip").vlan_ip;

				Ok(backend::job::Parameter {
					key: format!("artifact_url_{i}"),
					value: format!("http://{vlan_ip}:8080{path}"),
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?;

		// Run the job and forget about it
		let run_id = Uuid::new_v4();
		msg!([ctx] job_run::msg::create(run_id) {
			run_id: Some(run_id.into()),
			region_id: Some(region_id.into()),
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
