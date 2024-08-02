use std::{
	collections::{HashMap, HashSet},
	fmt,
	path::{Path, PathBuf},
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::{Duration, Instant},
};

use anyhow::*;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use indexmap::{IndexMap, IndexSet};
use indoc::formatdoc;
use rand::{seq::SliceRandom, thread_rng};
use reqwest::header;
use rivet_term::console::style;
use serde::Deserialize;
use serde_json::json;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{
	config::{ns, service::RuntimeKind},
	context::{ProjectContext, RunContext, ServiceContext},
	dep::{
		self,
		cargo::{
			self,
			cli::{TestBinary, TEST_IMAGE_NAME},
		},
	},
	utils,
};

/// Timeout for tests.
///
/// Default Chirp timeout is 60 seconds, so this is 15 seconds longer to give a buffer for Chirp
/// operations to time out first.
const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(75);

const DEFAULT_PARALLEL_TESTS: usize = 8;

pub struct TestCtx<'a, T: AsRef<str>> {
	pub svc_names: &'a [T],
	pub filters: Vec<String>,
	pub timeout: Option<u64>,
	pub parallel_tests: Option<usize>,
	pub no_purge: bool,
}

pub async fn test_all(
	ctx: &ProjectContext,
	timeout: Option<u64>,
	parallel_tests: Option<usize>,
	no_purge: bool,
) -> Result<()> {
	let all_svc_names = ctx
		.all_services()
		.await
		.iter()
		.map(|svc| svc.name())
		.collect::<Vec<_>>();
	test_services(
		ctx,
		TestCtx {
			svc_names: &all_svc_names,
			filters: Vec::new(),
			timeout,
			parallel_tests,
			no_purge,
		},
	)
	.await?;

	Ok(())
}

pub async fn test_services<T: AsRef<str>>(
	ctx: &ProjectContext,
	test_ctx: TestCtx<'_, T>,
) -> Result<()> {
	if ctx.ns().rivet.test.is_none() {
		bail!("tests are disabled, enable them by setting rivet.test in the namespace config");
	}

	let run_load_tests = ctx
		.ns()
		.rivet
		.test
		.as_ref()
		.map(|test| test.load_tests)
		.unwrap_or_default();

	// Resolve services
	let svc_names = test_ctx
		.svc_names
		.iter()
		.map(|x| x.as_ref().to_string())
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();
	let all_svcs = ctx.services_with_patterns(&svc_names).await;

	let rust_svcs = all_svcs
		.iter()
		// Find all services that are executables
		.filter(|svc_ctx| matches!(svc_ctx.config().runtime, RuntimeKind::Rust {}))
		// Filter/include load tests
		.filter(|svc_ctx| run_load_tests || !svc_ctx.config().service.load_test)
		.collect::<Vec<_>>();
	eprintln!();
	rivet_term::status::progress("Preparing", format!("{} services", rust_svcs.len()));

	// Telemetry
	let mut event = utils::telemetry::build_event(ctx, "bolt_test").await?;
	event.insert_prop(
		"svc_names",
		&rust_svcs.iter().map(|x| x.name()).collect::<Vec<_>>(),
	)?;
	event.insert_prop("filters", &test_ctx.filters)?;
	utils::telemetry::capture_event(ctx, event).await?;

	// Run batch commands for all given services
	eprintln!();
	rivet_term::status::progress("Building", "(batch)");
	let test_binaries = {
		// Collect rust services by their workspace root
		let mut svcs_by_workspace = HashMap::new();
		for svc in &rust_svcs {
			let workspace = svcs_by_workspace
				.entry(svc.workspace_path())
				.or_insert_with(Vec::new);
			workspace.push(svc.cargo_name().expect("no cargo name"));
		}
		ensure!(
			!svcs_by_workspace.is_empty(),
			"no matching services (to run load tests set `rivet.test.load_tests = true`)"
		);

		// Run build
		let test_binaries = cargo::cli::build_tests(
			ctx,
			cargo::cli::BuildTestOpts {
				build_calls: svcs_by_workspace
					.iter()
					.map(|(workspace_path, svc_names)| cargo::cli::BuildTestCall {
						path: workspace_path.strip_prefix(ctx.path()).unwrap(),
						packages: &svc_names,
					})
					.collect::<Vec<_>>(),
				release: false,
				jobs: ctx.config_local().rust.num_jobs,
				test_filters: &test_ctx.filters,
			},
		)
		.await
		.unwrap();

		test_binaries
	};

	// Choose image based on build config
	let image = if ctx.build_svcs_locally() {
		"ghcr.io/rivet-gg/rivet-local-binary-artifact-runner:07e8de0".to_string()
	} else {
		let (push_repo, pull_repo) = ctx.docker_repos().await;
		let source_hash = ctx.source_hash();
		let image_tag = format!("{push_repo}{TEST_IMAGE_NAME}:{source_hash}");

		eprintln!();
		rivet_term::status::progress("Uploading build", "");

		let mut cmd = Command::new("docker");
		cmd.arg("push");
		cmd.arg(image_tag);

		let status = cmd.status().await?;
		eprintln!();

		ensure!(status.success());

		// Return pull repo tag
		format!("{pull_repo}{TEST_IMAGE_NAME}:{source_hash}")
	};

	// Generate test ID
	let test_suite_id = gen_test_id();
	let purge = !test_ctx.no_purge;

	// Build exec ctx
	let run_context = RunContext::Test {
		test_id: test_suite_id.clone(),
	};
	let k8s_svc_name = format!("test-{test_suite_id}");

	// Apply pod
	let specs = gen_spec(
		ctx,
		&run_context,
		&rust_svcs,
		&image,
		&k8s_svc_name,
		ctx.build_svcs_locally(),
	)
	.await;
	dep::k8s::cli::apply_specs(ctx, specs).await?;

	// Wait for pod to start
	eprintln!();
	rivet_term::status::progress("Waiting for pod start", "");
	let label = format!("app.kubernetes.io/name={k8s_svc_name}");
	let status = Command::new("kubectl")
		.args([
			"wait",
			"--for=condition=Ready",
			"pod",
			"--selector",
			&label,
			"-n",
			"rivet-service",
		])
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.stdout(std::process::Stdio::null())
		.status()
		.await?;
	if !status.success() {
		bail!("failed to check pod readiness");
	}

	// Install CA
	rivet_term::status::progress("Installing CA", "");
	let status = Command::new("kubectl")
		.args([
			"exec",
			&format!("job/{k8s_svc_name}"),
			"-n",
			"rivet-service",
			"--",
			"/usr/bin/install_ca.sh",
		])
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.stdout(std::process::Stdio::null())
		.status()
		.await?;
	if !status.success() {
		bail!("failed to check pod readiness");
	}

	// Run tests
	let test_suite_start_time = Instant::now();

	eprintln!();
	rivet_term::status::progress("Running tests", &test_suite_id);
	eprintln!();
	let tests_complete = Arc::new(AtomicUsize::new(0));
	let test_count = test_binaries.len();
	let test_results = futures_util::stream::iter(test_binaries.into_iter().map(|test_binary| {
		let ctx = ctx.clone();
		let test_suite_id = test_suite_id.clone();
		let k8s_svc_name = k8s_svc_name.clone();
		let tests_complete = tests_complete.clone();
		let timeout = test_ctx.timeout;

		async move {
			run_test(
				&ctx,
				test_suite_id,
				k8s_svc_name,
				test_binary,
				tests_complete.clone(),
				test_count,
				timeout,
				purge,
			)
			.await
		}
	}))
	.buffer_unordered(test_ctx.parallel_tests.unwrap_or(DEFAULT_PARALLEL_TESTS))
	.try_collect::<Vec<_>>()
	.await?;

	// Delete job
	Command::new("kubectl")
		.args(["delete", "job", &k8s_svc_name, "-n", "rivet-service"])
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.output()
		.await?;

	// Print results
	print_results(&test_results, test_suite_start_time);

	cleanup_nomad(ctx, purge).await?;
	cleanup_servers(ctx).await?;

	// Error on failure
	let all_succeeded = test_results
		.iter()
		.all(|res| matches!(res.status, TestStatus::Pass));
	if !all_succeeded {
		eprintln!();
		bail!("at least one test failure occurred");
	}

	Ok(())
}

#[derive(Debug)]
enum TestStatus {
	Pass,
	TestFailed,
	Timeout,
	UnknownExitCode(i32),
	UnknownError(String),
}

#[derive(Debug)]
struct TestResult {
	status: TestStatus,
}

async fn run_test(
	ctx: &ProjectContext,
	test_suite_id: String,
	k8s_svc_name: String,
	test_binary: TestBinary,
	tests_complete: Arc<AtomicUsize>,
	test_count: usize,
	timeout: Option<u64>,
	purge_nomad_jobs: bool,
) -> Result<TestResult> {
	let svc_ctx = ctx
		.all_services()
		.await
		.into_iter()
		.find(|x| x.cargo_name() == Some(&test_binary.package))
		.context("svc not found for package")?;
	let display_name = format!("{}::{}", svc_ctx.name(), test_binary.test_name);

	let test_id = gen_test_id();

	// Build path to logs
	let logs_dir = Path::new("/tmp")
		.join(test_suite_id)
		.join(svc_ctx.name())
		.join(&test_binary.target);
	tokio::fs::create_dir_all(&logs_dir).await?;
	let logs_path = logs_dir.join(format!("{}.log", test_binary.test_name));

	rivet_term::status::info(
		"Running",
		format!(
			"{display_name} [{logs_path}]",
			logs_path = logs_path.display()
		),
	);

	let test_start_time = Instant::now();
	let timeout = timeout
		.map(Duration::from_secs)
		.unwrap_or(DEFAULT_TEST_TIMEOUT);
	let status = match tokio::time::timeout(
		timeout,
		exec_test(
			ctx,
			&k8s_svc_name,
			&test_binary,
			&test_id,
			logs_path.clone(),
		),
	)
	.await
	{
		Result::Ok(Result::Ok(x)) => x,
		Result::Ok(Err(err)) => TestStatus::UnknownError(err.to_string()),
		Err(_) => TestStatus::Timeout,
	};

	// Print status
	let test_duration = test_start_time.elapsed().as_secs_f32();
	let complete_count = tests_complete.fetch_add(1, Ordering::SeqCst) + 1;
	let run_info = format!(
		"{display_name} ({complete_count}/{test_count}) [{logs_path}] [{test_duration:.1}s]",
		logs_path = logs_path.display(),
	);
	match &status {
		TestStatus::Pass => {
			rivet_term::status::success("Passed", &run_info);
		}
		TestStatus::TestFailed => {
			rivet_term::status::error("Failed", &run_info);
		}
		TestStatus::Timeout => {
			rivet_term::status::error("Timeout", &run_info);
		}
		TestStatus::UnknownExitCode(code) => {
			rivet_term::status::error(format!("Unknown exit code {}", code), &run_info);
		}
		TestStatus::UnknownError(err) => {
			rivet_term::status::error(format!("Unknown error: {}", err), &run_info);
		}
	}

	cleanup_nomad_test(ctx, &test_id, purge_nomad_jobs).await?;

	Ok(TestResult { status })
}

/// Follow the pod logs and write them to a file.
async fn exec_test(
	ctx: &ProjectContext,
	k8s_svc_name: &str,
	test_binary: &TestBinary,
	test_id: &str,
	logs_path: PathBuf,
) -> Result<TestStatus> {
	let container_path = Path::new("/target").join(&test_binary.path);

	let command = format!(
		"RIVET_TEST_ID={test_id} {} --exact {}",
		&container_path.display(),
		&test_binary.test_name,
	);

	// Write logs to file
	let file = tokio::task::block_in_place(|| std::fs::File::create(&logs_path))?;
	let mut logs_child = Command::new("kubectl")
		.args([
			"exec",
			&format!("job/{k8s_svc_name}"),
			"-n",
			"rivet-service",
			"--",
			"sh",
			"-c",
			&command,
		])
		.env("KUBECONFIG", ctx.gen_kubeconfig_path())
		.stdout(file)
		.stderr(std::process::Stdio::null())
		.kill_on_drop(true)
		.spawn()?;
	let status = logs_child.wait().await?;

	// Write end of file
	let mut file = tokio::fs::OpenOptions::new()
		.append(true)
		.open(&logs_path)
		.await?;
	file.write_all(b"\n=== TEST FINISHED ===\n").await?;

	match status.code() {
		Some(0) => Ok(TestStatus::Pass),
		Some(101) => Ok(TestStatus::TestFailed),
		Some(x) => Ok(TestStatus::UnknownExitCode(x)),
		None => Ok(TestStatus::UnknownError("no status code".to_string())),
	}
}

fn print_results(test_results: &[TestResult], start_time: Instant) {
	let test_duration = start_time.elapsed().as_secs_f32();

	eprintln!();
	rivet_term::status::success("Complete", format!("{test_duration:.1}s"));

	let passed_count = test_results
		.iter()
		.filter(|test_result| matches!(test_result.status, TestStatus::Pass))
		.count();
	if passed_count > 0 {
		eprintln!(
			"  {}: {}/{}",
			style("PASS").italic().green(),
			passed_count,
			test_results.len()
		);
	}

	let failed_count = test_results
		.iter()
		.filter(|test_result| matches!(test_result.status, TestStatus::TestFailed))
		.count();
	if failed_count > 0 {
		eprintln!(
			"  {}: {}/{}",
			style("FAIL").italic().red(),
			failed_count,
			test_results.len()
		);
	}

	let timeout_count = test_results
		.iter()
		.filter(|test_result| matches!(test_result.status, TestStatus::Timeout))
		.count();
	if timeout_count > 0 {
		eprintln!(
			"  {}: {}/{}",
			style("TIMEOUT").italic().red(),
			timeout_count,
			test_results.len()
		);
	}

	let unknown_count = test_results
		.iter()
		.filter(|test_result| {
			matches!(
				test_result.status,
				TestStatus::UnknownExitCode(_) | TestStatus::UnknownError(_)
			)
		})
		.count();
	if unknown_count > 0 {
		eprintln!(
			"  {}: {}/{}",
			style("UNKNOWN").italic().red(),
			unknown_count,
			test_results.len()
		);
	}
}

#[derive(Deserialize)]
struct ApiErrorResponse {
	errors: Vec<ApiError>,
}

impl fmt::Display for ApiErrorResponse {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for error in &self.errors {
			if let Some(field) = &error.field {
				write!(f, "{:?}: ", field)?;
			}

			writeln!(f, "{}", error.reason)?;
		}

		std::result::Result::Ok(())
	}
}

#[derive(Deserialize)]
struct ApiError {
	field: Option<String>,
	reason: String,
}

#[derive(Debug, Deserialize)]
struct TaggedObjectsListResponse {
	data: Vec<TaggedObject>,
}

#[derive(Debug, Deserialize)]
struct TaggedObject {
	#[serde(rename = "type")]
	_type: String,
	data: TaggedObjectData,
}

#[derive(Debug, Deserialize)]
struct TaggedObjectData {
	id: u64,
	tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SshKeysListResponse {
	data: Vec<SshKey>,
}

#[derive(Debug, Deserialize)]
struct SshKey {
	id: u64,
}

async fn cleanup_servers(ctx: &ProjectContext) -> Result<()> {
	if ctx.ns().rivet.provisioning.is_none() {
		return Ok(());
	}

	eprintln!();
	rivet_term::status::progress("Cleaning up servers", "");

	let ns = ctx.ns_id();
	let ns_full = format!("rivet-{ns}");

	// Create client
	let api_token = ctx.read_secret(&["linode", "token"]).await?;
	let auth = format!("Bearer {}", api_token);
	let mut headers = header::HeaderMap::new();
	headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&auth)?);
	let client = reqwest::Client::builder()
		.default_headers(headers)
		.build()?;

	let test_tag = "test";
	let (objects_res, ssh_keys_res) = tokio::try_join!(
		async {
			// Fetch all objects with the tag "test"
			let res = client
				.get(format!("https://api.linode.com/v4/tags/{test_tag}"))
				.send()
				.await?;

			if !res.status().is_success() {
				bail!(
					"api request failed ({}):\n{}",
					res.status(),
					res.json::<ApiErrorResponse>().await?
				);
			}

			// Deserialize
			Ok(res.json::<TaggedObjectsListResponse>().await?)
		},
		async {
			// Fetch all ssh keys with "test-" in their label and are in this namespace
			let res = client
				.get("https://api.linode.com/v4/profile/sshkeys".to_string())
				.header(
					"X-Filter",
					format!(r#"{{ "label": {{ "+contains": "{test_tag}-{ns}-" }} }}"#),
				)
				.send()
				.await?;

			if !res.status().is_success() {
				bail!(
					"api request failed ({}):\n{}",
					res.status(),
					res.json::<ApiErrorResponse>().await?
				);
			}

			// Deserialize
			Ok(res.json::<SshKeysListResponse>().await?)
		}
	)?;

	let deletions = objects_res
		.data
		.into_iter()
		// Only delete test objects from this namespace
		.filter(|object| {
			object
				.data
				.tags
				.iter()
				.any(|tag| tag == ns || tag == &ns_full)
		})
		.map(|object| {
			let client = client.clone();
			let obj_type = object._type;
			let id = object.data.id;

			async move {
				eprintln!("destroying {obj_type} {id}");

				// Destroy resource
				let res = match obj_type.as_str() {
					"linode" => {
						client
							.delete(format!("https://api.linode.com/v4/linode/instances/{id}"))
							.send()
							.await?
					}
					"firewall" => {
						client
							.delete(format!(
								"https://api.linode.com/v4/networking/firewalls/{id}"
							))
							.send()
							.await?
					}
					_ => {
						eprintln!("unknown type tagged with \"test\": {obj_type}");
						return Ok(());
					}
				};

				if !res.status().is_success() {
					// Resource does not exist to be deleted, not an error
					if res.status() == reqwest::StatusCode::NOT_FOUND {
						eprintln!("{obj_type} {id} doesn't exist, skipping");
						return Ok(());
					}

					bail!(
						"api request failed ({}):\n{}",
						res.status(),
						res.json::<ApiErrorResponse>().await?
					);
				}

				Ok(())
			}
			.boxed()
		})
		.chain(ssh_keys_res.data.into_iter().map(|key| {
			let client = client.clone();
			let id = key.id;

			async move {
				eprintln!("destroying key {id}");

				let res = client
					.delete(format!("https://api.linode.com/v4/profile/sshkeys/{id}"))
					.send()
					.await?;

				if !res.status().is_success() {
					// Resource does not exist to be deleted, not an error
					if res.status() == reqwest::StatusCode::NOT_FOUND {
						eprintln!("key {id} doesn't exist, skipping");
						return Ok(());
					}

					bail!(
						"api request failed ({}):\n{}",
						res.status(),
						res.json::<ApiErrorResponse>().await?
					);
				}

				Ok(())
			}
			.boxed()
		}));

	futures_util::stream::iter(deletions)
		.buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

fn gen_test_id() -> String {
	let mut rng = thread_rng();
	(0..8)
		.map(|_| {
			let mut chars = ('a'..='z').chain('0'..='9').collect::<Vec<_>>();
			chars.shuffle(&mut rng);
			chars[0]
		})
		.collect()
}

// Cleans up all nomad jobs from a specific test
async fn cleanup_nomad_test(ctx: &ProjectContext, test_id: &str, purge: bool) -> Result<()> {
	// Fetch all jobs from this test
	let fetch_cmd = format!(
		r#"nomad operator api -filter 'Meta.rivet_test_id == "{test_id}"' /v1/jobs?meta=true"#
	);

	let mut cmd = Command::new("kubectl");
	cmd.args([
		"exec",
		"service/nomad-server",
		"-n",
		"nomad",
		"--container",
		"nomad-instance",
		"--",
		"sh",
		"-c",
		&fetch_cmd,
	]);
	cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());

	let output = cmd.output().await?;
	ensure!(output.status.success());

	if output.stdout == b"No cluster leader" {
		panic!("no cluster leader");
	}

	let jobs: Vec<NomadJob> = serde_json::from_slice(&output.stdout)?;

	// Cleanup jobs
	let purge = if purge { "-purge" } else { "" };
	let cleanup_cmd = jobs
		.iter()
		.map(|job| format!("nomad job stop {purge} -detach {}", job.id))
		.collect::<Vec<_>>()
		.join("\n");

	let mut cmd = Command::new("kubectl");
	cmd.args([
		"exec",
		"service/nomad-server",
		"-n",
		"nomad",
		"--container",
		"nomad-instance",
		"--",
		"sh",
		"-c",
		&cleanup_cmd,
	]);
	cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());

	let output = cmd.output().await?;
	ensure!(output.status.success());

	Ok(())
}

// Cleans up all nomad jobs
async fn cleanup_nomad(ctx: &ProjectContext, purge: bool) -> Result<()> {
	eprintln!();
	rivet_term::status::progress("Cleaning up jobs", "");

	let purge = if purge { "-purge" } else { "" };
	let cleanup_cmd = formatdoc!(
		r#"
		nomad job status |
			grep -v -e "ID" -e "No running jobs" |
			cut -f1 -d ' ' |
			xargs -I {{}} nomad job stop {purge} -detach {{}}
		"#
	);

	let mut cmd = Command::new("kubectl");
	cmd.args([
		"exec",
		"service/nomad-server",
		"-n",
		"nomad",
		"--container",
		"nomad-instance",
		"--",
		"sh",
		"-c",
		&cleanup_cmd,
	]);
	cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());

	let status = cmd.status().await?;
	ensure!(status.success());

	Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct NomadJob {
	#[serde(rename = "ID")]
	id: String,
}

/// Generates the k8s spec for the main test pod.
pub async fn gen_spec(
	ctx: &ProjectContext,
	run_context: &RunContext,
	svcs: &[&ServiceContext],
	image: &str,
	k8s_svc_name: &str,
	build_svcs_locally: bool,
) -> Vec<serde_json::Value> {
	let mut specs = Vec::new();

	// Render env
	let mut env = IndexMap::new();
	let mut secret_env = IndexMap::new();

	for svc_ctx in svcs {
		env.extend(svc_ctx.env(run_context).await.unwrap());
		secret_env.extend(svc_ctx.secret_env(run_context).await.unwrap());
	}

	let env = dep::k8s::gen::generate_k8s_variables()
		.into_iter()
		.chain(
			env.into_iter()
				.map(|(k, v)| json!({ "name": k, "value": v })),
		)
		.collect::<Vec<_>>();

	// Create secret env vars
	let secret_env_name = format!("{}-secret-env", k8s_svc_name);
	let secret_data = secret_env
		.into_iter()
		.map(|(k, v)| (k, base64::encode(v)))
		.collect::<HashMap<_, _>>();
	specs.push(json!({
		"apiVersion": "v1",
		"kind": "Secret",
		"metadata": {
			"name": secret_env_name,
			"namespace": "rivet-service"
		},
		"data": secret_data
	}));

	let (volumes, volume_mounts) = build_volumes(ctx, run_context, svcs, build_svcs_locally).await;

	let metadata = json!({
		"name": k8s_svc_name,
		"namespace": "rivet-service",
		"labels": {
			"app.kubernetes.io/name": k8s_svc_name
		}
	});

	let pod_spec = json!({
		"restartPolicy": "Never",
		"terminationGracePeriodSeconds": 0,
		"imagePullSecrets": [{
			"name": "docker-auth"
		}],
		"containers": [{
			"name": "service",
			"image": image,
			"imagePullPolicy": "IfNotPresent",
			"command": ["/bin/sh"],
			"args": ["-c", "sleep 100000"],
			"env": env,
			"envFrom": [{
				"secretRef": {
					"name": secret_env_name
				}
			}],
			"volumeMounts": volume_mounts,
		}],
		"volumes": volumes
	});
	let pod_template = json!({
		"metadata": {
			"labels": {
				"app.kubernetes.io/name": k8s_svc_name
			},
		},
		"spec": pod_spec,
	});

	specs.push(json!({
		"apiVersion": "batch/v1",
		"kind": "Job",
		"metadata": metadata,
		"spec": {
			"ttlSecondsAfterFinished": 3,
			"completions": 1,
			"backoffLimit": 0,
			"template": pod_template
		}
	}));

	specs
}

pub async fn build_volumes(
	project_ctx: &ProjectContext,
	run_context: &RunContext,
	svcs: &[&ServiceContext],
	build_svcs_locally: bool,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
	// Shared data between containers
	let mut volumes = Vec::<serde_json::Value>::new();
	let mut volume_mounts = Vec::<serde_json::Value>::new();

	match &project_ctx.ns().cluster.kind {
		ns::ClusterKind::SingleNode { .. } => {
			if build_svcs_locally {
				// Volumes
				volumes.push(json!({
					"name": "target",
					"hostPath": {
						"path": "/target",
						"type": "Directory"
					}
				}));
				volumes.push(json!({
					"name": "nix-store",
					"hostPath": {
						"path": "/nix/store",
						"type": "Directory"
					}
				}));

				// Mounts
				volume_mounts.push(json!({
					"name": "target",
					"mountPath": "/target",
					"readOnly": true
				}));
				volume_mounts.push(json!({
					"name": "nix-store",
					"mountPath": "/nix/store",
					"readOnly": true
				}));
			}
		}
		ns::ClusterKind::Distributed { .. } => {
			// Those volumes only exist on single node setups
		}
	}

	// Add Redis CA
	match project_ctx.ns().redis.provider {
		ns::RedisProvider::Kubernetes {} => {
			let mut redis_deps = IndexSet::with_capacity(2);

			for svc in svcs {
				let svc_redis_deps =
					svc.redis_dependencies(run_context)
						.await
						.into_iter()
						.map(|redis_dep| {
							if let RuntimeKind::Redis { persistent } = redis_dep.config().runtime {
								if persistent {
									"persistent"
								} else {
									"ephemeral"
								}
							} else {
								unreachable!();
							}
						});

				redis_deps.extend(svc_redis_deps);
			}

			volumes.extend(redis_deps.iter().map(|db| {
				json!({
					"name": format!("redis-{}-ca", db),
					"configMap": {
						"name": format!("redis-{}-ca", db),
						"defaultMode": 420,
						"items": [
							{
								"key": "ca.crt",
								"path": format!("redis-{}-ca.crt", db)
							}
						]
					}
				})
			}));
			volume_mounts.extend(redis_deps.iter().map(|db| {
				json!({
					"name": format!("redis-{}-ca", db),
					"mountPath": format!("/usr/local/share/ca-certificates/redis-{}-ca.crt", db),
					"subPath": format!("redis-{}-ca.crt", db)
				})
			}));
		}
		ns::RedisProvider::Aws { .. } | ns::RedisProvider::Aiven { .. } => {
			// Uses publicly signed cert
		}
	}

	// Add CRDB CA
	match project_ctx.ns().cockroachdb.provider {
		ns::CockroachDBProvider::Kubernetes {} => {
			volumes.push(json!({
				"name": "crdb-ca",
				"configMap": {
					"name": "crdb-ca",
					"defaultMode": 420,
					"items": [
						{
							"key": "ca.crt",
							"path": "crdb-ca.crt"
						}
					]
				}
			}));
			volume_mounts.push(json!({
				"name": "crdb-ca",
				"mountPath": "/usr/local/share/ca-certificates/crdb-ca.crt",
				"subPath": "crdb-ca.crt"
			}));
		}
		ns::CockroachDBProvider::Managed { .. } => {
			// Uses publicly signed cert
		}
	}

	// Add ClickHouse CA
	if let Some(clickhouse) = &project_ctx.ns().clickhouse {
		match clickhouse.provider {
			ns::ClickHouseProvider::Kubernetes {} => {
				volumes.push(json!({
					"name": "clickhouse-ca",
					"configMap": {
						"name": "clickhouse-ca",
						"defaultMode": 420,
						"items": [
							{
								"key": "ca.crt",
								"path": "clickhouse-ca.crt"
							}
						]
					}
				}));
				volume_mounts.push(json!({
					"name": "clickhouse-ca",
					"mountPath": "/usr/local/share/ca-certificates/clickhouse-ca.crt",
					"subPath": "clickhouse-ca.crt"
				}));
			}
			ns::ClickHouseProvider::Managed { .. } => {
				// Uses publicly signed cert
			}
		}
	}

	(volumes, volume_mounts)
}
