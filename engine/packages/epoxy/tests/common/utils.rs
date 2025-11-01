use anyhow::*;
use epoxy::ops::propose::ProposalResult;
use epoxy_protocol::protocol;

/// Helper function to execute a command through the ops API
#[allow(dead_code)]
pub async fn execute_command(
	ctx: &gas::prelude::TestCtx,
	command: protocol::CommandKind,
	_wait_for_propagation: bool,
) -> Result<ProposalResult> {
	// Call the propose operation directly
	let result = ctx
		.op(epoxy::ops::propose::Input {
			proposal: protocol::Proposal {
				commands: vec![protocol::Command { kind: command }],
			},
			purge_cache: false,
		})
		.await?;

	Ok(result)
}
