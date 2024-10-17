use std::{collections::HashMap, path::Path, process::Command, sync::Arc};

use rivet_term::console::style;
use tokio::fs;

use crate::{
	context::{ProjectContext, ProjectContextData, ServiceContextData},
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
	let secrets =
		ProjectContextData::read_secrets(Some(&ctx.ns().secrets), ctx.path(), &ns_id).await;

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
