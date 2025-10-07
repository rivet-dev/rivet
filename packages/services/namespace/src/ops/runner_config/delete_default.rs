use epoxy_protocol::protocol;
use gas::prelude::*;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn namespace_runner_config_delete_default(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<()> {
	let key = keys::runner_config::DefaultKey::new(input.namespace_id, input.name.clone());
	let key_packed = keys::subspace().pack(&key);

	let result = ctx
		.op(epoxy::ops::propose::Input {
			proposal: protocol::Proposal {
				commands: vec![protocol::Command {
					kind: protocol::CommandKind::SetCommand(protocol::SetCommand {
						key: key_packed,
						value: None,
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
