use std::{collections::HashMap, path::Path, process::Command, sync::Arc};

use rivet_term::console::style;
use tokio::fs;

use crate::{
	context::ServiceContextData,
	context::{ProjectContext, ProjectContextData},
	dep, tasks,
	utils::{self, command_helper::CommandHelper},
};

pub async fn check_all(
	ctx: &ProjectContext,
	skip_build: bool,
	skip_generate: bool,
	skip_tests: bool,
	skip_config_sync_check: bool,
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
		skip_config_sync_check,
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
	skip_config_sync_check: bool,
	validate_format: bool,
) {
	use crate::config::service::RuntimeKind;

	let all_svcs = ctx.services_with_patterns(&svc_names).await;
	assert!(!all_svcs.is_empty(), "input matched no services");

	// Generate configs
	if !skip_generate {
		tasks::gen::generate_project(ctx, skip_config_sync_check).await;
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
			cmd.env("RUSTFLAGS", "--cfg tokio_unstable");
			cmd.env("CARGO_TARGET_DIR", ctx.path().join("target"));
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

/// Checks if the config and secrets files are in sync with 1Password.
pub async fn check_config_sync(ctx: &ProjectContext) {
	if std::env::var("BOLT_SKIP_CONFIG_SYNC")
		.ok()
		.map_or(false, |x| x == "1")
	{
		return;
	}

	// Check if 1Password service token is available
	if ctx.ns().secrets._1password.is_some() && ctx.config_local()._1password.is_none() {
		eprintln!();
		rivet_term::status::warn(
			"Warning",
			format!(
				r#"Cannot validate that config is synchronized without configuring the 1Password service token in Bolt.local.toml. See docs/libraries/bolt/CONFIG_SYNC.md for details or use `{}` to suppress this message."#,
				style("BOLT_SKIP_CONFIG_SYNC=1").bold()
			),
		);
		return;
	}

	let (Some(local_op), Some(ns_op)) = (
		ctx.config_local()._1password.as_ref(),
		ctx.ns().secrets._1password.as_ref(),
	) else {
		return;
	};

	let op_service_account_token = Some(local_op.service_account_token.clone());
	let op_namespace_path = &ns_op.namespace_path;
	let op_secrets_path = &ns_op.secrets_path;

	// Fetch and parse configs from 1Password
	let op_namespace_str =
		dep::one_password::cli::read(op_service_account_token.as_deref(), op_namespace_path).await;
	let op_namespace = toml::from_str::<serde_json::Value>(&op_namespace_str)
		.expect("failed to parse op namespace config");
	let op_secrets_str =
		dep::one_password::cli::read(op_service_account_token.as_deref(), op_secrets_path).await;
	let op_secrets =
		toml::from_str::<serde_json::Value>(&op_secrets_str).expect("failed to parse op secrets");

	// Fetch local configs
	let ns_id = ctx.ns_id();
	let namespace_path = ctx.ns_path().join(format!("{ns_id}.toml"));
	let secrets_path = ctx.secrets_path();

	let local_namespace_str = fs::read_to_string(&namespace_path).await.unwrap();
	let namespace = toml::from_str::<serde_json::Value>(&local_namespace_str)
		.expect("failed to read namespace config");
	let secrets = ProjectContextData::read_secrets(Some(ctx.ns()), ctx.path(), &ns_id).await;

	let ns_patches = json_patch::diff(&op_namespace, &namespace);
	if !ns_patches.is_empty() {
		rivet_term::status::error("Error",
			format!("Diff detected between local namespace file ({}) and 1Password namespace reference ({}):",
			namespace_path.display(),
			op_namespace_path
		));
		utils::render_diff(2, &ns_patches);
	}

	let secrets_patches = json_patch::diff(&op_secrets, &secrets);
	if !secrets_patches.is_empty() {
		if !ns_patches.is_empty() {
			eprintln!();
		}

		rivet_term::status::error("Error",
			format!("Diff detected between local secrets file ({}) and 1Password secrets reference ({}):",
			secrets_path.display(),
			op_secrets_path,
		));
		utils::render_diff(2, &secrets_patches);
	}

	if !ns_patches.is_empty() || !secrets_patches.is_empty() {
		eprintln!(
			"\nUse `{}` to resolve this conflict or `{}` to suppress this message.",
			style("bolt config push/pull").bold(),
			style("BOLT_SKIP_CONFIG_SYNC=1").bold()
		);
		std::process::exit(1);
	}
}
