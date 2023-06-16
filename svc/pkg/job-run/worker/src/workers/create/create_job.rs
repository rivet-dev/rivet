use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::NOMAD_CONFIG;

// TODO: Only run create job if run job returns job not found

pub async fn create_job(
	base_job_json: &str,
	region: &backend::region::Region,
) -> GlobalResult<String> {
	let (job_id, job) = build_job(base_job_json, region)?;

	// Submit the job
	submit_job(&job_id, job.clone(), region).await?;

	Ok(job_id)
}

fn override_job_id(job_id: &str, job: &mut nomad_client::models::Job) {
	job.ID = Some(job_id.into());
	job.name = Some(job_id.into());
}

fn build_job(
	base_job_json: &str,
	region: &backend::region::Region,
) -> GlobalResult<(String, nomad_client::models::Job)> {
	let base_job = serde_json::from_str::<nomad_client::models::Job>(base_job_json)?;

	// Modify the job spec
	let mut job = modify_job_spec(base_job, region)?;

	// Derive jobspec hash
	//
	// We serialize the JSON to a canonical string then take a SHA hash of the output.
	let job_cjson_str = match cjson::to_string(&job) {
		Ok(x) => x,
		Err(err) => {
			tracing::error!(?err, "cjson serialization failed");
			internal_panic!("cjson serialization failed")
		}
	};
	let job_hash = Sha256::digest(job_cjson_str.as_bytes());
	let job_hash_str = hex::encode(job_hash);

	// Generate new job ID
	let job_id = format!(
		"job-{hash}:{region}",
		hash = &job_hash_str[0..12],
		region = region.name_id
	);
	override_job_id(&job_id, &mut job);

	Ok((job_id, job))
}

/// Modifies the provided job spec to be compatible with the Rivet job runtime.
fn modify_job_spec(
	mut job: nomad_client::models::Job,
	region: &backend::region::Region,
) -> GlobalResult<nomad_client::models::Job> {
	// Replace all job IDs with a placeholder value in order to create a
	// deterministic job spec.
	override_job_id("__PLACEHOLDER__", &mut job);

	internal_assert_eq!(
		"batch",
		internal_unwrap!(job._type).as_str(),
		"only the batch job type is supported"
	);

	// Update the job's region
	job.region = Some(region.nomad_region.clone());
	job.datacenters = Some(vec![region.nomad_datacenter.clone()]);

	// Validate that the job is parameterized
	let parameters =
		internal_unwrap_owned!(job.parameterized_job.as_mut(), "job not parameterized");

	// Add run parameters
	parameters.meta_required = Some({
		let mut meta_required = parameters.meta_required.clone().unwrap_or_default();
		meta_required.push("job_run_id".into());
		meta_required.push("job_run_token".into());
		meta_required
	});

	// Get task group
	let task_groups = internal_unwrap_owned!(job.task_groups.as_mut());
	internal_assert_eq!(1, task_groups.len(), "must have exactly 1 task group");
	let task_group = internal_unwrap_owned!(task_groups.first_mut());

	// Configure networks
	let networks = internal_unwrap_owned!(task_group.networks.as_mut());
	internal_assert_eq!(1, networks.len(), "must have exactly 1 network");
	let network = internal_unwrap_owned!(networks.first_mut());
	network.DNS = Some(Box::new(nomad_client::models::NetworkDns {
		servers: Some(vec![
			// Cloudflare
			"1.1.1.1".into(),
			"1.0.0.1".into(),
			"2606:4700:4700::1111".into(),
			"2606:4700:4700::1001".into(),
			// Google
			"8.8.8.8".into(),
			"8.8.4.4".into(),
			"2001:4860:4860::8888".into(),
			"2001:4860:4860::8844".into(),
		]),
		..nomad_client::models::NetworkDns::new()
	}));

	// Disable rescheduling, since job-run doesn't support this at the moment
	task_group.reschedule_policy = Some(Box::new(nomad_client::models::ReschedulePolicy {
		attempts: Some(0),
		unlimited: Some(false),
		..nomad_client::models::ReschedulePolicy::new()
	}));

	// Disable restarts. Our Nomad monitoring workflow doesn't support restarts
	// at the moment.
	task_group.restart_policy = Some(Box::new(nomad_client::models::RestartPolicy {
		attempts: Some(0),
		// unlimited: Some(false),
		..nomad_client::models::RestartPolicy::new()
	}));

	// Add cleanup task
	let tasks = internal_unwrap_owned!(task_group.tasks.as_mut());
	tasks.push(gen_cleanup_task());

	Ok(job)
}

fn gen_cleanup_task() -> nomad_client::models::Task {
	use nomad_client::models::*;

	Task {
		name: Some(util_job::RUN_CLEANUP_TASK_NAME.into()),
		lifecycle: Some(Box::new(TaskLifecycle {
			hook: Some("poststop".into()),
			sidecar: Some(false),
		})),
		driver: Some("docker".into()),
		config: Some({
			let mut config = HashMap::new();

			config.insert("image".into(), json!("python:3.10.7-alpine3.16"));
			config.insert(
				"args".into(),
				json!([
					"/bin/sh",
					"-c",
					"apk add --no-cache ca-certificates && python3 /local/cleanup.py"
				]),
			);

			config
		}),
		templates: Some(vec![Template {
			dest_path: Some("local/cleanup.py".into()),
			embedded_tmpl: Some(formatdoc!(
				r#"
					import ssl
					import urllib.request, json, os, mimetypes, sys

					API_JOB_URL = '{api_job_url}'
					BEARER = '{{{{env "NOMAD_META_JOB_RUN_TOKEN"}}}}'

					ctx = ssl.create_default_context()

					def eprint(*args, **kwargs):
    					print(*args, file=sys.stderr, **kwargs)

					def req(method, url, data = None, headers = {{}}):
						request = urllib.request.Request(
							url=url,
							data=data,
							method=method,
							headers=headers
						)

						try:
							res = urllib.request.urlopen(request, context=ctx)
							assert res.status == 200, f"Received non-200 status: {{res.status}}"
							return res
						except urllib.error.HTTPError as err:
							eprint(f"HTTP Error ({{err.code}} {{err.reason}}):\n\nBODY:\n{{err.read().decode()}}\n\nHEADERS:\n{{err.headers}}")

							raise err

					print(f'\n> Cleaning up job')

					res_json = None
					with req('POST', f'{{API_JOB_URL}}/runs/cleanup',
						data = json.dumps({{}}).encode(),
						headers = {{
							'Authorization': f'Bearer {{BEARER}}',
							'Content-Type': 'application/json'
						}}
					) as res:
						res_json = json.load(res)


					print('\n> Finished')
					"#,
				api_job_url = util::env::svc_router_url("api-job"),
			)),
			..Template::new()
		}]),
		resources: Some(Box::new(Resources {
			CPU: Some(util_job::TASK_CLEANUP_CPU),
			memory_mb: Some(util_job::TASK_CLEANUP_MEMORY),
			..Resources::new()
		})),
		log_config: Some(Box::new(LogConfig {
			max_files: Some(4),
			max_file_size_mb: Some(2),
		})),
		..Task::new()
	}
}

#[tracing::instrument]
async fn submit_job(
	job_id: &str,
	job: nomad_client::models::Job,
	region: &backend::region::Region,
) -> GlobalResult<()> {
	tracing::info!("submitting job");

	nomad_client::apis::jobs_api::update_job(
		&NOMAD_CONFIG,
		job_id,
		None,
		Some(&region.nomad_region),
		None,
		None,
		Some(nomad_client::models::RegisterJobRequest {
			job: Some(Box::new(job)),
			enforce_index: None,
			job_modify_index: None,
			policy_override: None,
		}),
	)
	.await?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use chirp_worker::prelude::*;
	use proto::backend;
	use serde_json::json;

	#[test]
	fn job_name_determinism_eq() {
		let region = gen_region();

		// Run multiple times to ensure there's no coincidental hash overlaps
		for _ in 0..16 {
			let x = "Hello, determinism!";
			let base_job_json_a = serde_json::to_string(&gen_job(x)).unwrap();
			let base_job_json_b = serde_json::to_string(&gen_job(x)).unwrap();

			let (job_id_a, _) = super::build_job(&base_job_json_a, &region).unwrap();
			let (job_id_b, _) = super::build_job(&base_job_json_b, &region).unwrap();
			assert_eq!(job_id_a, job_id_b, "job id is not deterministic");
		}
	}

	#[test]
	fn job_name_determinism_ne() {
		let region = gen_region();

		let base_job_json_a = serde_json::to_string(&gen_job("foo")).unwrap();
		let base_job_json_b = serde_json::to_string(&gen_job("bar")).unwrap();

		let (job_id_a, _) = super::build_job(&base_job_json_a, &region).unwrap();
		let (job_id_b, _) = super::build_job(&base_job_json_b, &region).unwrap();
		assert_ne!(job_id_a, job_id_b, "job id is not deterministic");
	}

	fn gen_region() -> backend::region::Region {
		backend::region::Region {
			name_id: "lnd-atl".into(),
			nomad_region: "global".into(),
			nomad_datacenter: "lnd-atl".into(),
			..Default::default()
		}
	}

	fn gen_job(x: &str) -> nomad_client::models::Job {
		use nomad_client::models::*;

		// This job ID will be overridden, so it should not matter what we put
		// here
		let job_id = Uuid::new_v4().to_string();

		Job {
			ID: Some(job_id.clone()),
			name: Some(job_id),
			_type: Some("batch".into()),
			constraints: Some(vec![Constraint {
				l_target: Some("${node.class}".into()),
				r_target: Some("job".into()),
				operand: Some("=".into()),
			}]),
			parameterized_job: Some(Box::new(ParameterizedJobConfig {
				meta_required: Some(vec!["test_id".into()]),
				..ParameterizedJobConfig::new()
			})),
			task_groups: Some(vec![TaskGroup {
				name: Some("test".into()),
				networks: Some(vec![NetworkResource {
					// So we can access it from the test
					mode: Some("bridge".into()),
					dynamic_ports: Some(vec![Port {
						label: Some("http".into()),
						value: None,
						to: Some(80),
					}]),
					..NetworkResource::new()
				}]),
				services: Some(vec![Service {
					provider: Some("nomad".into()),
					ID: Some("test-job".into()),
					name: Some("test-job".into()),
					tags: Some(vec!["test".into()]),
					..Service::new()
				}]),
				tasks: Some(vec![Task {
					name: Some("test".into()),
					driver: Some("docker".into()),
					config: Some({
						let mut config = HashMap::new();
						config.insert("image".into(), json!("alpine:3.14"));
						config.insert("args".into(), json!(["echo", x]));
						config
					}),
					..Task::new()
				}]),
				..TaskGroup::new()
			}]),
			..Job::new()
		}
	}
}
