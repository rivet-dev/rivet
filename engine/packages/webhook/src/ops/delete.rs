use epoxy::ops::propose::{
	CheckAndSetCommand, Command, CommandKind, ConsensusFailedReason, Proposal, ProposalResult,
};
use epoxy_protocol::protocol::CachingBehavior;
use gas::prelude::*;

use crate::{errors, keys, workflows};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn webhook_config_delete(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let namespace_id = input.namespace_id;
	let name = input.name.clone();

	let global_key = keys::GlobalDataKey::new(namespace_id, name.clone());

	let current = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: namespace::keys::subspace().pack(&global_key),
			caching_behavior: CachingBehavior::Optimistic,
			target_replicas: None,
			save_empty: false,
		})
		.await?
		.value;

	let propose_res = ctx
		.op(epoxy::ops::propose::Input {
			proposal: Proposal {
				commands: vec![Command {
					kind: CommandKind::CheckAndSetCommand(CheckAndSetCommand {
						key: namespace::keys::subspace().pack(&global_key),
						expect_one_of: vec![current], // TODO: verify CAS is implemented. Doesn't work with epoxyV2. Implement(?). Fails tests currently
						new_value: None,
					}),
				}],
			},
			purge_cache: true,
			mutable: true,
			target_replicas: None,
		})
		.await?;

	match propose_res {
		ProposalResult::Committed => {}
		ProposalResult::ConsensusFailed { reason } => match reason {
			ConsensusFailedReason::ExpectedValueDoesNotMatch { .. } => {
				return Err(errors::Webhook::Conflict.build());
			}
			ConsensusFailedReason::PreparePhaseConsensusFailed => {
				bail!("epoxy propose failed: prepare phase consensus failed");
			}
			ConsensusFailedReason::AcceptPhaseConsensusFailed => {
				bail!("epoxy propose failed: accept phase consensus failed");
			}
			ConsensusFailedReason::StaleBallot => {
				bail!("epoxy propose failed: stale ballot");
			}
		},
	}

	ctx.signal(workflows::webhook::Destroy {})
		.to_workflow::<workflows::webhook::Workflow>()
		.tag("namespace_id", input.namespace_id)
		.tag("name", input.name.clone())
		.graceful_not_found()
		.send()
		.await?;

	Ok(())
}
