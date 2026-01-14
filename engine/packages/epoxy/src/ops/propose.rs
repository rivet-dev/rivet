use anyhow::*;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use epoxy_protocol::protocol::{self, Path, Payload, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::prelude::*;
use std::time::Instant;

use crate::{http_client, metrics, replica, utils};

#[derive(Debug, Serialize, Deserialize)]
pub enum ProposalResult {
	Committed,
	ConsensusFailed,
	CommandError(CommandError),
}

/// Command errors indicate that a proposal succeeded but the command did not apply.
///
/// Proposals that have command errors are still written to the log but have no effect.
#[derive(Debug, Serialize, Deserialize)]
pub enum CommandError {
	ExpectedValueDoesNotMatch { current_value: Option<Vec<u8>> },
}

#[derive(Debug)]
pub struct Input {
	pub proposal: protocol::Proposal,
	/// Only works in non-workflow contexts.
	pub purge_cache: bool,
}

#[operation]
pub async fn epoxy_propose(ctx: &OperationCtx, input: &Input) -> Result<ProposalResult> {
	let start = Instant::now();
	let replica_id = ctx.config().epoxy_replica_id();

	// Read config
	let config = ctx
		.udb()?
		.run(|tx| async move { utils::read_config(&tx, replica_id).await })
		.custom_instrument(tracing::info_span!("read_config_tx"))
		.await
		.context("failed reading config")?;

	// Lead consensus
	let payload = ctx
		.udb()?
		.run(|tx| {
			let proposal = input.proposal.clone();
			async move { replica::lead_consensus::lead_consensus(&*tx, replica_id, proposal).await }
		})
		.custom_instrument(tracing::info_span!("lead_consensus_tx"))
		.await
		.context("failed leading consensus")?;

	// Get quorum members (only active replicas for voting)
	let quorum_members = utils::get_quorum_members(&config);

	// EPaxos Step 5
	let pre_accept_oks =
		send_pre_accepts(ctx, &config, replica_id, &quorum_members, &payload).await?;

	// Decide path
	let path = ctx
		.udb()?
		.run(|tx| {
			let pre_accept_oks = pre_accept_oks.clone();
			let payload = payload.clone();
			async move { replica::decide_path::decide_path(&*tx, pre_accept_oks, &payload) }
		})
		.custom_instrument(tracing::info_span!("decide_path_tx"))
		.await
		.context("failed deciding path")?;

	let res = match path {
		Path::PathFast(protocol::PathFast { payload }) => {
			commit(ctx, &config, replica_id, payload, input.purge_cache).await?
		}
		Path::PathSlow(protocol::PathSlow { payload }) => {
			run_paxos_accept(
				ctx,
				&config,
				replica_id,
				&quorum_members,
				payload,
				input.purge_cache,
			)
			.await?
		}
	};

	metrics::PROPOSAL_DURATION.observe(start.elapsed().as_secs_f64());
	metrics::PROPOSALS_TOTAL
		.with_label_values(&[if let ProposalResult::Committed = res {
			"ok"
		} else {
			"err"
		}])
		.inc();

	Ok(res)
}

#[tracing::instrument(skip_all)]
pub async fn run_paxos_accept(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	payload: Payload,
	purge_cache: bool,
) -> Result<ProposalResult> {
	// Clone payload for use after the closure
	let payload_for_accepts = payload.clone();

	// Mark as accepted
	ctx.udb()?
		.run(|tx| {
			let payload = payload.clone();
			async move { replica::messages::accepted(&*tx, replica_id, payload).await }
		})
		.custom_instrument(tracing::info_span!("accept_tx"))
		.await
		.context("failed accepting")?;

	// EPaxos Step 17
	let quorum = send_accepts(
		ctx,
		&config,
		replica_id,
		&quorum_members,
		&payload_for_accepts,
	)
	.await?;

	// EPaxos Step 20
	if quorum >= utils::calculate_quorum(quorum_members.len(), utils::QuorumType::Slow) {
		commit(ctx, &config, replica_id, payload_for_accepts, purge_cache).await
	} else {
		Ok(ProposalResult::ConsensusFailed)
	}
}

#[tracing::instrument(skip_all)]
pub async fn commit(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	payload: Payload,
	purge_cache: bool,
) -> Result<ProposalResult> {
	// Commit locally
	//
	// Receives command error after committing to KV. Proposals are still committed even if there
	// is a command error since command errors are purely feedback to the client that the command
	// was not applied.
	let cmd_err = {
		let payload = payload.clone();
		ctx.udb()?
			.run(|tx| {
				let payload = payload.clone();
				async move {
					let cmd_err = replica::messages::committed(&*tx, replica_id, &payload).await?;

					Result::Ok(cmd_err)
				}
			})
			.custom_instrument(tracing::info_span!("committed_tx"))
			.await
			.context("failed committing")?
	};

	// EPaxos Step 23
	// Send commits to all replicas (not just quorum members)
	let all_replicas = utils::get_all_replicas(config);
	tokio::spawn({
		let ctx = ctx.clone();
		let config = config.clone();
		let replica_id = replica_id;
		let all_replicas = all_replicas.to_vec();
		let payload = payload.clone();

		async move {
			let _ = send_commits(&ctx, &config, replica_id, &all_replicas, &payload).await;
		}
	});

	if purge_cache {
		let keys = payload
			.proposal
			.commands
			.iter()
			.map(replica::utils::extract_key_from_command)
			.flatten()
			.map(|key| BASE64.encode(key))
			.collect::<Vec<_>>();

		// Purge optimistic cache for all dcs
		if !keys.is_empty() {
			let ctx = ctx.clone();
			tokio::spawn(async move {
				if let Err(err) = purge_optimistic_cache(ctx, keys).await {
					tracing::error!(?err, "failed purging optimistic cache");
				}
			});
		}
	}

	if let Some(cmd_err) = cmd_err {
		Ok(ProposalResult::CommandError(cmd_err))
	} else {
		Ok(ProposalResult::Committed)
	}
}

#[tracing::instrument(skip_all)]
async fn send_pre_accepts(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	payload: &Payload,
) -> Result<Vec<Payload>> {
	let responses = http_client::fanout_to_replicas(
		from_replica_id,
		replica_ids,
		utils::QuorumType::Fast,
		|to_replica_id| {
			let config = config.clone();
			let payload = payload.clone();
			async move {
				let response = http_client::send_message(
					&ApiCtx::new_from_operation(&ctx)?,
					&config,
					protocol::Request {
						from_replica_id,
						to_replica_id,
						kind: protocol::RequestKind::PreAcceptRequest(protocol::PreAcceptRequest {
							payload,
						}),
					},
				)
				.await?;

				let protocol::Response {
					kind: protocol::ResponseKind::PreAcceptResponse(response),
				} = response
				else {
					bail!("wrong response type");
				};

				Ok(response.payload)
			}
		},
	)
	.await?;

	Ok(responses)
}

#[tracing::instrument(skip_all)]
async fn send_accepts(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	payload: &Payload,
) -> Result<usize> {
	let responses = http_client::fanout_to_replicas(
		from_replica_id,
		replica_ids,
		utils::QuorumType::Slow,
		|to_replica_id| {
			let config = config.clone();
			let payload = payload.clone();
			async move {
				let response = http_client::send_message(
					&ApiCtx::new_from_operation(&ctx)?,
					&config,
					protocol::Request {
						from_replica_id,
						to_replica_id,
						kind: protocol::RequestKind::AcceptRequest(protocol::AcceptRequest {
							payload,
						}),
					},
				)
				.await?;

				let protocol::Response {
					kind: protocol::ResponseKind::AcceptResponse(_),
				} = response
				else {
					bail!("wrong response type");
				};

				Ok(())
			}
		},
	)
	.await?;

	// Add 1 to indicate this node has accepted it
	Ok(responses.len() + 1)
}

#[tracing::instrument(skip_all)]
async fn send_commits(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	payload: &Payload,
) -> Result<()> {
	http_client::fanout_to_replicas(
		from_replica_id,
		replica_ids,
		utils::QuorumType::All,
		|to_replica_id| {
			let config = config.clone();
			let payload = payload.clone();
			async move {
				let response = http_client::send_message(
					&ApiCtx::new_from_operation(&ctx)?,
					&config,
					protocol::Request {
						from_replica_id,
						to_replica_id,
						kind: protocol::RequestKind::CommitRequest(protocol::CommitRequest {
							payload,
						}),
					},
				)
				.await?;

				let protocol::Response {
					kind: protocol::ResponseKind::CommitResponse,
				} = response
				else {
					bail!("wrong response type");
				};

				Ok(())
			}
		},
	)
	.await?;

	Ok(())
}

async fn purge_optimistic_cache(ctx: OperationCtx, keys: Vec<String>) -> Result<()> {
	for dc in &ctx.config().topology().datacenters {
		let workflow_id = ctx
			.workflow(crate::workflows::purger::Input {
				replica_id: dc.datacenter_label as u64,
			})
			.tag("replica_id", dc.datacenter_label as u64)
			.unique()
			.dispatch()
			.await?;
		ctx.signal(crate::workflows::purger::Purge { keys: keys.clone() })
			// This is ok because double purging is idempotent
			.bypass_signal_from_workflow_I_KNOW_WHAT_IM_DOING()
			.to_workflow_id(workflow_id)
			.send()
			.await?;
	}

	Ok(())
}
