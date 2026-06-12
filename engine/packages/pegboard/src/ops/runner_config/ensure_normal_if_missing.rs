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
	let pool_res = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.name.clone())],
			bypass_cache: true,
		})
		.await?;

	if pool_res.is_empty() {
		ctx.op(crate::ops::runner_config::upsert::Input {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			config: rivet_types::runner_configs::RunnerConfig {
				kind: rivet_types::runner_configs::RunnerConfigKind::Normal {
					drain_on_version_upgrade: false,
					actor_eviction_delay: 0,
					actor_eviction_period: 0,
					actor_eviction_rate: 1.0,
				},
				metadata: None,
			},
		})
		.await?;
	}

	Ok(())
}
