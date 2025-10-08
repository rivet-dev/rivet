use epoxy_protocol::protocol;
use gas::prelude::*;
use rivet_types::runner_configs::RunnerConfig;
use universaldb::prelude::FormalKey;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub config: RunnerConfig,
}

#[operation]
pub async fn namespace_runner_config_upsert(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let key = keys::runner_config::DefaultKey::new(input.namespace_id, input.name.clone());
	let key_packed = keys::subspace().pack(&key);
	let runner_config_packed = key.serialize(input.config.clone())?;

	// Propagate default runner config over epoxy
	let result = ctx
		.op(epoxy::ops::propose::Input {
			proposal: protocol::Proposal {
				commands: vec![protocol::Command {
					kind: protocol::CommandKind::SetCommand(protocol::SetCommand {
						key: key_packed,
						value: Some(runner_config_packed),
					}),
				}],
			},
		})
		.await?;

	ensure!(
		matches!(result, epoxy::ops::propose::ProposalResult::Committed),
		"proposal failed"
	);

	// Bump autoscaler in all dcs
	ctx.op(internal::ops::bump_serverless_autoscaler_global::Input {})
		.await?;

	Ok(())
}
