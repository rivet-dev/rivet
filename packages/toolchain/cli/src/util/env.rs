use anyhow::*;
use toolchain::{
	meta, paths,
	tasks::{self},
};
use uuid::Uuid;

use crate::util::task::{run_task, TaskOutputStyle};

/// Select an environment or use the provided env.
///
/// Returns the env's slug.
pub async fn get_or_select(
	ctx: &toolchain::ToolchainCtx,
	slug: Option<impl ToString>,
) -> Result<String> {
	// Return configured env
	if let Some(slug) = slug {
		return Ok(slug.to_string());
	};

	// Read the selected env
	let selected_env_id =
		meta::try_read_project(&paths::data_dir()?, |p| Ok(p.cloud()?.selected_environment))
			.await?;
	if let Some(env) = ctx
		.project
		.namespaces
		.iter()
		.find(|x| Some(x.namespace_id) == selected_env_id)
	{
		return Ok(env.name_id.clone());
	}

	// Prompt user for selection
	select(ctx, false).await
}

/// Select an environment.
///
/// Forcing selection will prompt the user for selection, even if there's only 1 item.
pub async fn select(ctx: &toolchain::ToolchainCtx, force_select: bool) -> Result<String> {
	// Build selections
	let mut envs = ctx
		.project
		.namespaces
		.iter()
		.map(|n| EnvWrapper {
			id: n.namespace_id.clone(),
			slug: n.name_id.clone(),
			name: n.display_name.clone(),
		})
		.collect::<Vec<_>>();
	envs.sort_by_key(|e| e.name.clone());

	// If only one env, don't prompt
	if !force_select && envs.len() == 1 {
		let env = envs.into_iter().next().expect("should have 1 env value");
		return Ok(env.slug);
	}

	// Choose starting index
	let start_env_id =
		meta::try_read_project(&paths::data_dir()?, |p| Ok(p.cloud()?.selected_environment))
			.await?;
	let start_idx = envs
		.iter()
		.position(|e| Some(e.id) == start_env_id)
		.unwrap_or(0);

	// Prompt
	let selected = tokio::task::block_in_place(|| {
		inquire::Select::new("Environment", envs)
			.with_starting_cursor(start_idx)
			.prompt()
	})?;

	// Update settings
	run_task::<tasks::env::select::Task>(
		TaskOutputStyle::None,
		tasks::env::select::Input {
			environment_id: selected.id,
		},
	)
	.await?;

	Ok(selected.slug.clone())
}

/// Struct used for wrapping data in selector.
struct EnvWrapper {
	id: Uuid,
	slug: String,
	name: String,
}

impl std::fmt::Display for EnvWrapper {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{} ({})", self.name, self.slug)
	}
}
