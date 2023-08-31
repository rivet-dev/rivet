use anyhow::*;
use rivet_term::console::style;
use std::collections::HashSet;
use tokio::{fs, process::Command};

use crate::{
	config::service::RuntimeKind,
	context::{ProjectContext, RunContext, ServiceContext},
	dep::{
		cloudflare, fly,
		nomad::{self, NomadCtx},
	},
	tasks, utils,
};

struct TestCleanupManager {
	project_ctx: ProjectContext,
	nomad_ctx: NomadCtx,

	/// List of dispatched job names that already before the test starts. Used
	/// to diff with the new dispatched jobs to stop jobs that were dispatched
	/// in a test.
	existing_dispatched_jobs: Vec<nomad::api::JobSummary>,

	/// List of Fly apps that already exist before the test starts. Used to diff with apps created
	/// afterwares to know what apps to clean up.
	existing_fly_apps: Option<HashSet<String>>,
}

impl TestCleanupManager {
	async fn setup(project_ctx: ProjectContext, nomad_ctx: NomadCtx) -> Result<TestCleanupManager> {
		let existing_dispatched_jobs = nomad::api::list_jobs_with_prefix(&nomad_ctx, "job-")
			.await?
			.into_iter()
			.filter(|x| x.id.contains("/dispatch-") || x.id.contains("/periodic-"))
			.collect::<Vec<_>>();

		let existing_fly_apps = if project_ctx.ns().fly.is_some() {
			Some(
				fly::api::list_apps(&project_ctx)
					.await?
					.into_iter()
					.map(|x| x.name)
					.collect::<HashSet<_>>(),
			)
		} else {
			None
		};

		Ok(TestCleanupManager {
			project_ctx,
			nomad_ctx,
			existing_dispatched_jobs,
			existing_fly_apps,
		})
	}

	async fn run(self) -> Result<()> {
		self.cleanup_nomad().await?;
		self.cleanup_fly().await?;

		Ok(())
	}

	async fn cleanup_nomad(&self) -> Result<()> {
		// Fetch list of current dispatched jobs
		let new_dispatched_jobs = nomad::api::list_jobs_with_prefix(&self.nomad_ctx, "job-")
			.await?
			.into_iter()
			.filter(|x| x.id.contains("/dispatch-") || x.id.contains("/periodic-"))
			.collect::<Vec<_>>();

		// Diff the jobs
		let old_ids = self
			.existing_dispatched_jobs
			.iter()
			.map(|x| x.id.as_str())
			.collect::<HashSet<&str>>();
		let current_ids = new_dispatched_jobs
			.iter()
			.map(|x| x.id.as_str())
			.collect::<HashSet<&str>>();
		let created_ids = &current_ids - &old_ids;

		if !created_ids.is_empty() {
			rivet_term::status::info("Cleaning up jobs", format!("{} jobs", created_ids.len()));
			for id in &created_ids {
				nomad::api::stop_job(&self.nomad_ctx, id).await?;
			}
		}

		Ok(())
	}

	async fn cleanup_fly(&self) -> Result<()> {
		let Some(existing_apps) = &self.existing_fly_apps else {
			return Ok(());
		};

		// Fetch list of existing apps
		let current_apps = fly::api::list_apps(&self.project_ctx)
			.await?
			.into_iter()
			.map(|x| x.name)
			.collect::<HashSet<_>>();

		// Diff the apps
		let created_apps = &current_apps - existing_apps;

		if !created_apps.is_empty() {
			rivet_term::status::info("Cleaning up apps", format!("{} apps", created_apps.len()));
			for name in &created_apps {
				// Delete app
				fly::api::delete_app(&self.project_ctx, &name).await?;
			}
		}

		Ok(())
	}
}

pub async fn test_all(
	_ctx: &ProjectContext,
	_test_only: bool,
	_test_name: Option<&str>,
	_force_build: bool,
	_skip_generate: bool,
) -> Result<()> {
	// TODO: Update this to work like test_service
	unimplemented!("TODO: update test_all to work like test_service")

	// if ctx.ns().rivet.test.is_none() {
	// 	bail!("tests are disabled, enable them by setting rivet.test in the namespace config");
	// }

	// // Run all services
	// if !test_only {
	// 	tasks::up::up_all(
	// 		ctx,
	// 		tasks::up::UpOpts {
	// 			skip_build: false,
	// 			skip_dependencies: false,
	// 			force_build,
	// 			skip_generate,
	// 			auto_approve: true,
	// 		},
	// 	)
	// 	.await?;
	// }

	// // Write region config locally
	// let regions_json = serde_json::to_vec(&ctx.ns().regions)?;
	// fs::write(ctx.gen_path().join("region-config.json"), &regions_json).await?;

	// // Check all services
	// let svc_ctxs = ctx.all_services().await;
	// for ctx_svc in svc_ctxs {
	// 	run_test(ctx_svc, test_name).await;
	// }

	// // TODO: Boot dev services if test succeeds

	// Ok(())
}

pub async fn test_service<T: AsRef<str>>(
	ctx: &ProjectContext,
	svc_names: &[T],
	test_only: bool,
	test_name: Option<&str>,
	skip_deps: bool,
	force_build: bool,
	skip_generate: bool,
) -> Result<()> {
	if ctx.ns().rivet.test.is_none() {
		bail!("tests are disabled, enable them by setting rivet.test in the namespace config");
	}

	// Get contexts
	let svc_ctxs = ctx.services_with_patterns(&svc_names).await;

	// Run the services. Already ran gen in `check`.
	if !test_only {
		tasks::up::up_services(
			ctx,
			svc_names,
			tasks::up::UpOpts {
				skip_build: false,
				skip_dependencies: skip_deps,
				force_build,
				skip_generate,
				auto_approve: true,
			},
		)
		.await?;
	}

	// HACK: Write region config locally to mimic the file mounted to the task dir
	let regions_json = serde_json::to_vec(&ctx.ns().regions)?;
	fs::write(ctx.gen_path().join("region-config.json"), &regions_json).await?;

	// Test service
	let mut passed = Vec::new();
	let mut failed = Vec::new();
	for ctx in &svc_ctxs {
		match run_test(ctx, test_name).await {
			TestResult::Success => {
				passed.push(ctx);
			}
			TestResult::Failure => {
				failed.push(ctx);
			}
			TestResult::Cancel => {
				break;
			}
		}
	}

	eprintln!();
	rivet_term::status::success(
		"Finished",
		format!(
			"{}/{} passed",
			(svc_ctxs.len() - failed.len()),
			svc_ctxs.len()
		),
	);

	for svc in &passed {
		eprintln!("  * {}: {}", svc.name(), style("PASS").italic().green());
	}

	for svc in &failed {
		eprintln!("  * {}: {}", svc.name(), style("FAIL").italic().red());
	}

	Ok(())
}

enum TestResult {
	Success,
	Failure,
	Cancel,
}

async fn run_test(svc_ctx: &ServiceContext, test_name: Option<&str>) -> TestResult {
	eprintln!();
	eprintln!();
	rivet_term::status::info("Testing", svc_ctx.name());

	let project_ctx = svc_ctx.project().await;

	// *Really* make sure we don't run a test in production
	if project_ctx.ns().rivet.test.is_none() {
		unreachable!();
	}

	let nomad_ctx = NomadCtx::remote(&project_ctx).await.unwrap();

	let cleanup = TestCleanupManager::setup(project_ctx.clone(), nomad_ctx.clone())
		.await
		.unwrap();

	// Render env
	let (mut env, forward_configs) = svc_ctx.env(RunContext::Test).await.unwrap();
	let (secret_env, secret_forward_configs) = svc_ctx.secret_env(RunContext::Test).await.unwrap();
	env.extend(secret_env);

	// Forward services
	let forwards = forward_configs
		.into_iter()
		.chain(secret_forward_configs)
		.map(|c| {
			utils::kubectl_port_forward(c.service_name, c.namespace, (c.local_port, c.remote_port))
		})
		.collect::<Result<Vec<_>>>()
		.unwrap();

	// Wait for port forwards to open and check if successful
	tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	forwards
		.iter()
		.map(|h| h.check())
		.collect::<Result<Vec<_>>>()
		.unwrap();

	// Run tests
	let res = async {
		match &svc_ctx.config().runtime {
			RuntimeKind::Rust {} => {
				let mut cmd = Command::new("cargo");
				cmd.current_dir(svc_ctx.path());
				cmd.env("RUSTFLAGS", "--cfg tokio_unstable");
				cmd.arg("test");
				if let Some(jobs) = project_ctx.config_local().rust.num_jobs {
					cmd.arg("--jobs").arg(jobs.to_string());
				}
				if let Some(test_name) = test_name {
					cmd.arg(test_name);
				}
				// Only run one test at a time
				cmd.args(["--", "--test-threads", "1"]);
				cmd.envs(env);
				ensure!(cmd.status().await.unwrap().success(), "test failed");

				Result::Ok(())
			}
			RuntimeKind::CRDB { .. }
			| RuntimeKind::ClickHouse { .. }
			| RuntimeKind::Redis { .. }
			| RuntimeKind::S3 { .. }
			| RuntimeKind::Nats { .. } => {
				rivet_term::status::info("No tests to run", "");
				Result::Ok(())
			}
		}
	};

	let test_result = tokio::select! {
		res = res => {
			match res {
				Result::Ok(_) => {
					rivet_term::status::success("Passed", svc_ctx.name());
					TestResult::Success
				}
				Result::Err(err) => {
					rivet_term::status::error("Failed", format!("{err:?}"));
					TestResult::Failure
				}
			}
		}
		_ = tokio::signal::ctrl_c() => {
			rivet_term::status::warn("Cancelled", "");
			TestResult::Cancel
		}
	};

	cleanup.run().await.unwrap();

	test_result
}
