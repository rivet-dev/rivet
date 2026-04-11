use gas::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

/// Creates a default normal runner config for this namespace and runner name if one does not
/// already exist. Returns `true` when the config was created.
#[operation]
pub async fn pegboard_runner_config_ensure_normal_if_missing(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<()> {
	ctx.op(crate::ops::runner_config::upsert::Input {
		namespace_id: input.namespace_id,
		name: input.name.clone(),
		config: rivet_types::runner_configs::RunnerConfig {
			kind: rivet_types::runner_configs::RunnerConfigKind::Normal {},
			metadata: None,
			drain_on_version_upgrade: false,
		},
	})
	.await?;

	Ok(())
}
