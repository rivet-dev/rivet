use std::{collections::HashMap, path::Path, process::Command, sync::Arc};

use crate::{
	context::ProjectContext, context::ServiceContextData, tasks,
	utils::command_helper::CommandHelper,
};

pub async fn check_all(
	ctx: &ProjectContext,
	skip_build: bool,
	skip_generate: bool,
	skip_tests: bool,
	validate_format: bool,
) {
	let all_svc_names = ctx
		.all_services()
		.await
		.iter()
		.map(|svc| svc.name())
		.collect::<Vec<_>>();
	check_service(
		ctx,
		&all_svc_names,
		skip_build,
		skip_generate,
		skip_tests,
		validate_format,
	)
	.await;
}

pub async fn check_service<T: AsRef<str>>(
	ctx: &ProjectContext,
	svc_names: &[T],
	_skip_build: bool,
	skip_generate: bool,
	skip_tests: bool,
	validate_format: bool,
) {
	use crate::config::service::RuntimeKind;

	let all_svcs = ctx.services_with_patterns(&svc_names).await;
	assert!(!all_svcs.is_empty(), "input matched no services");

	// Generate configs
	if !skip_generate {
		tasks::gen::generate_project(ctx).await;
		tasks::gen::generate_all_services(ctx).await;
		tasks::artifact::generate_all_services(ctx).await;
	}

	rivet_term::status::progress("Checking services", "(batch)");
	{
		let rust_svcs = all_svcs
			.iter()
			.filter(|svc_ctx| match svc_ctx.config().runtime {
				RuntimeKind::Rust {} => true,
				_ => false,
			});

		// Collect rust services by their workspace root
		let mut svcs_by_workspace = HashMap::new();
		for svc in rust_svcs {
			let workspace = svcs_by_workspace
				.entry(svc.workspace_path())
				.or_insert_with(Vec::new);
			workspace.push(svc);
		}

		for (workspace_path, svcs) in svcs_by_workspace {
			check_svcs(ctx, validate_format, skip_tests, workspace_path, svcs).await;
		}
	}

	// println!("\n> Checking services [individual]");
	// for svc_ctx in &all_svcs {
	// 	match svc_ctx.config().runtime {
	// 		RuntimeKind::Rust { .. } => {}
	// 		RuntimeKind::CRDB { .. }
	// 		| RuntimeKind::ClickHouse { .. }
	// 		| RuntimeKind::Redis { .. }
	// 		| RuntimeKind::S3 { .. }
	// 		| RuntimeKind::Nats { .. } => {
	// 			// Do nothing
	// 		}
	// 	}
	// }

	eprintln!();
	rivet_term::status::success("Valid", "")
}

async fn check_svcs(
	ctx: &ProjectContext,
	validate_format: bool,
	skip_tests: bool,
	path: &Path,
	svcs: Vec<&Arc<ServiceContextData>>,
) {
	if !svcs.is_empty() {
		// cargo fmt
		{
			// TODO: Figure out how to check lib dependencies, `--all`
			// checks all packages in the project

			let mut cmd = Command::new("cargo");
			cmd.current_dir(path);
			cmd.arg("fmt");
			if validate_format {
				cmd.arg("--check");
			}
			for svc_ctx in &svcs {
				cmd.arg("--package")
					.arg(svc_ctx.cargo_name().expect("no cargo name"));
			}
			cmd.exec().await.unwrap();
		}

		// cargo clippy
		{
			// `cargo clippy` runs `cargo check` under the hood.
			let mut cmd = Command::new("cargo");
			cmd.current_dir(path);
			cmd.arg("clippy");

			// Check tests, which will also check the main module. Using
			// `--all-targets` will cause duplicate errors.
			if !skip_tests {
				cmd.arg("--tests");
			}

			if let Some(jobs) = ctx.config_local().rust.num_jobs {
				cmd.arg("--jobs").arg(jobs.to_string());
			}

			for svc_ctx in &svcs {
				cmd.arg("--package")
					.arg(svc_ctx.cargo_name().expect("no cargo name"));
			}

			if let Some(fmt) = &ctx.config_local().rust.message_format {
				cmd.arg(format!("--message-format={fmt}"));
			}

			// Enable more warnings. See
			// https://zhauniarovich.com/post/2021/2021-09-pedantic-clippy/#command-line-based-approach
			//
			// clippy::cargo is disabled since we don't care about this for
			// our auto-generated services.
			cmd.arg("--")
				// .arg("-W")
				// .arg("clippy::pedantic")
				// .arg("-W")
				// .arg("clippy::nursery")
				.arg("-W")
				.arg("clippy::unnecessary_cast")
				.arg("-A")
				.arg("clippy::wildcard_imports")
				.arg("-A")
				.arg("clippy::module_name_repetitions")
				.arg("-W")
				.arg("clippy::cast_possible_truncation")
				.arg("-A")
				.arg("clippy::missing_errors_doc")
				.arg("-A")
				.arg("clippy::missing_const_for_fn")
				.arg("-A")
				.arg("clippy::default_trait_access")
				.arg("-A")
				.arg("clippy::unwrap_in_result")
				.arg("-A")
				.arg("clippy::unwrap_used");

			cmd.exec().await.unwrap();
		}
	}
}
