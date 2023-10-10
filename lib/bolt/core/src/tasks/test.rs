use anyhow::*;
use futures_util::{StreamExt, TryStreamExt};
use rand::{seq::SliceRandom, thread_rng};
use rivet_term::console::style;

use std::sync::{
	atomic::{AtomicUsize, Ordering},
	Arc,
};
use std::{
	collections::{HashMap, HashSet},
	path::Path,
	time::{Duration, Instant},
};
use tokio::process::Command;

use crate::{
	config::{ns, service::RuntimeKind},
	context::{ProjectContext, RunContext},
	dep::{
		self,
		cargo::{self, cli::TestBinary},
		k8s::gen::{ExecServiceContext, ExecServiceDriver},
	},
};

/// Timeout for tests.
///
/// Default Chirp timeout is 60 seconds, so this is 15 seconds longer to give a buffer for Chirp
/// operations to time out first.
const TEST_TIMEOUT: Duration = Duration::from_secs(75);

const PARALLEL_TESTS: usize = 8;

pub async fn test_all(ctx: &ProjectContext) -> Result<()> {
	let all_svc_names = ctx
		.all_services()
		.await
		.iter()
		.map(|svc| svc.name())
		.collect::<Vec<_>>();
	test_services(ctx, &all_svc_names, Vec::new()).await?;

	Ok(())
}

pub async fn test_services<T: AsRef<str>>(
	ctx: &ProjectContext,
	svc_names: &[T],
	filters: Vec<String>,
) -> Result<()> {
	if ctx.ns().rivet.test.is_none() {
		bail!("tests are disabled, enable them by setting rivet.test in the namespace config");
	}

	// TODO: implement tests for distributed clusters (must upload test build to docker)
	match ctx.ns().cluster.kind {
		ns::ClusterKind::SingleNode { .. } => {}
		ns::ClusterKind::Distributed { .. } => {
			bail!("tests not implemented for distributed clusters")
		}
	}

	// Resolve services
	let svc_names = svc_names
		.iter()
		.map(|x| x.as_ref().to_string())
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();
	let all_svcs = ctx.services_with_patterns(&svc_names).await;

	// Find all services that are executables
	let rust_svcs = all_svcs
		.iter()
		.filter(|svc_ctx| matches!(svc_ctx.config().runtime, RuntimeKind::Rust {}))
		.collect::<Vec<_>>();
	eprintln!();
	rivet_term::status::progress("Preparing", format!("{} services", rust_svcs.len()));

	// Run batch commands for all given services
	eprintln!();
	rivet_term::status::progress("Building", "(batch)");
	let test_binaries = {
		// Collect rust services by their workspace root
		let mut svcs_by_workspace = HashMap::new();
		for svc in rust_svcs {
			let workspace = svcs_by_workspace
				.entry(svc.workspace_path())
				.or_insert_with(Vec::new);
			workspace.push(svc.cargo_name().expect("no cargo name"));
		}
		ensure!(!svcs_by_workspace.is_empty(), "no matching services");

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
				jobs: ctx.config_local().rust.num_jobs,
				test_filters: &filters,
			},
		)
		.await
		.unwrap();

		test_binaries
	};

	// Run tests
	eprintln!();
	rivet_term::status::progress("Running tests", "");
	let tests_complete = Arc::new(AtomicUsize::new(0));
	let test_count = test_binaries.len();
	let test_results = futures_util::stream::iter(test_binaries.into_iter().map(|test_binary| {
		let ctx = ctx.clone();
		let tests_complete = tests_complete.clone();
		let filters = filters.clone();
		async move {
			run_test(
				&ctx,
				test_binary,
				tests_complete.clone(),
				test_count,
				filters,
			)
			.await
		}
	}))
	.buffer_unordered(PARALLEL_TESTS)
	.try_collect::<Vec<_>>()
	.await?;

	// Print results
	print_results(&test_results);

	let all_succeeded = test_results
		.iter()
		.all(|res| matches!(res.status, TestStatus::Pass));
	if !all_succeeded {
		bail!("at least one test failure occurred");
	}

	Ok(())
}

fn print_results(test_results: &[TestResult]) {
	eprintln!();
	rivet_term::status::success("Complete", "");

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
	test_binary: TestBinary,
	tests_complete: Arc<AtomicUsize>,
	test_count: usize,
	filters: Vec<String>,
) -> Result<TestResult> {
	let svc_ctx = ctx
		.all_services()
		.await
		.into_iter()
		.find(|x| x.cargo_name() == Some(&test_binary.package))
		.context("svc not found for package")?;
	let display_name = format!("{}::{}", svc_ctx.name(), test_binary.target);

	// Convert path relative to project
	let relative_path = test_binary
		.path
		.strip_prefix(ctx.path())
		.context("path not in project")?;
	let container_path = Path::new("/rivet-src").join(relative_path);

	// Build exec ctx
	let exec_ctx = ExecServiceContext {
		svc_ctx: svc_ctx.clone(),
		run_context: RunContext::Test {
			test_id: gen_test_id(),
		},
		driver: ExecServiceDriver::LocalBinaryArtifact {
			exec_path: container_path,
			// Limit test running in parallel & filter the tests that get ran
			args: [vec!["--test-threads".to_string(), "2".to_string()], filters].concat(),
		},
	};

	// Build specs
	let specs = dep::k8s::gen::gen_svc(&exec_ctx).await;
	let svc_name = dep::k8s::gen::k8s_svc_name(&exec_ctx);

	// Apply pod
	dep::k8s::cli::apply_specs(ctx, specs).await?;

	// Tail pod
	rivet_term::status::info("Running", format!("{display_name} [job/{svc_name}]"));
	let test_start_time = Instant::now();
	let status = match tokio::time::timeout(TEST_TIMEOUT, tail_pod(ctx, &svc_name)).await {
		Result::Ok(Result::Ok(x)) => x,
		Result::Ok(Err(err)) => TestStatus::UnknownError(err.to_string()),
		Err(_) => {
			Command::new("kubectl")
				.args(&["delete", "job", &svc_name, "-n", "rivet-service"])
				.env("KUBECONFIG", ctx.gen_kubeconfig_path())
				.output()
				.await?;

			TestStatus::Timeout
		}
	};

	// Print status
	let test_duration = test_start_time.elapsed();
	let complete_count = tests_complete.fetch_add(1, Ordering::SeqCst) + 1;
	let run_info = format!(
		"{display_name} ({complete_count}/{test_count}) [job/{svc_name}] [{td:.1}s]",
		td = test_duration.as_secs_f32()
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
			rivet_term::status::error(&format!("Unknown exit code {}", code), &run_info);
		}
		TestStatus::UnknownError(err) => {
			rivet_term::status::error(&format!("Unknown error: {}", err), &run_info);
		}
	}

	Ok(TestResult { status })
}

async fn tail_pod(ctx: &ProjectContext, svc_name: &str) -> Result<TestStatus> {
	let label = format!("app.kubernetes.io/name={svc_name}");

	loop {
		// TODO: Use --wait for better performance
		let output = Command::new("kubectl")
			.args(&[
				"get",
				"pod",
				"--selector",
				&label,
				"-n",
				"rivet-service",
				"-o",
				"jsonpath={.items[0].status.phase}",
			])
			.env("KUBECONFIG", ctx.gen_kubeconfig_path())
			.output()
			.await?;

		let output_str = String::from_utf8_lossy(&output.stdout);
		let output_str = output_str.trim();
		match output_str {
			"Pending" | "Running" | "" => {
				// Continue
				tokio::time::sleep(Duration::from_millis(500)).await;
			}
			"Succeeded" | "Failed" => {
				// Get the exit code of the pod
				let output = Command::new("kubectl")
					.args(&[
						"get",
						"pod",
						"--selector",
						&label,
						"-n",
						"rivet-service",
						"-o",
						"jsonpath={.items[0].status.containerStatuses[0].state.terminated.exitCode}",
					])
					.env("KUBECONFIG", ctx.gen_kubeconfig_path())
					.output()
					.await?;

				let exit_code_str = String::from_utf8_lossy(&output.stdout);
				let exit_code: i32 = exit_code_str.trim().parse()?;

				let test_status = match exit_code {
					0 => TestStatus::Pass,
					101 => TestStatus::TestFailed,
					x => TestStatus::UnknownExitCode(x),
				};

				return Ok(test_status);
			}
			_ => bail!("unexpected pod status: {}", output_str),
		}
	}
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
