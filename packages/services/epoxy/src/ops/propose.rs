use anyhow::*;
use epoxy_protocol::protocol::{self, Path, Payload, ReplicaId};
use gas::prelude::*;
use rivet_api_builder::prelude::*;
use rivet_config::Config;
use universaldb::FdbBindingError;

use crate::{http_client, replica, utils};

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
}

#[operation]
pub async fn propose(ctx: &OperationCtx, input: &Input) -> Result<ProposalResult> {
	let replica_id = ctx.config().epoxy_replica_id();

	// Read config
	let config = ctx
		.udb()?
		.run(move |tx, _| async move {
			utils::read_config(&tx, replica_id)
				.await
				.map_err(|x| FdbBindingError::CustomError(x.into()))
		})
		.await?;

	// Lead consensus
	let payload = ctx
		.udb()?
		.run(move |tx, _| {
			let proposal = input.proposal.clone();
			async move {
				replica::lead_consensus::lead_consensus(&*tx, replica_id, proposal)
					.await
					.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
			}
		})
		.await?;

	// Get quorum members (only active replicas for voting)
	let quorum_members = utils::get_quorum_members(&config);

	// EPaxos Step 5
	let pre_accept_oks = send_pre_accepts(&config, replica_id, &quorum_members, &payload).await?;

	// Decide path
	let path = ctx
		.udb()?
		.run(move |tx, _| {
			let pre_accept_oks = pre_accept_oks.clone();
			let payload = payload.clone();
			async move {
				replica::decide_path::decide_path(&*tx, pre_accept_oks, &payload)
					.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
			}
		})
		.await?;

	match path {
		Path::PathFast(protocol::PathFast { payload }) => {
			commit(ctx, &config, replica_id, &quorum_members, payload).await
		}
		Path::PathSlow(protocol::PathSlow { payload }) => {
			run_paxos_accept(ctx, &config, replica_id, &quorum_members, payload).await
		}
	}
}

pub async fn run_paxos_accept(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	payload: Payload,
) -> Result<ProposalResult> {
	// Clone payload for use after the closure
	let payload_for_accepts = payload.clone();

	// Mark as accepted
	ctx.udb()?
		.run(move |tx, _| {
			let payload = payload.clone();
			async move {
				replica::messages::accepted(&*tx, replica_id, payload)
					.await
					.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))
			}
		})
		.await?;

	// EPaxos Step 17
	let quorum = send_accepts(&config, replica_id, &quorum_members, &payload_for_accepts).await?;

	// EPaxos Step 20
	if quorum >= utils::calculate_quorum(quorum_members.len(), utils::QuorumType::Slow) {
		commit(
			ctx,
			&config,
			replica_id,
			&quorum_members,
			payload_for_accepts,
		)
		.await
	} else {
		Ok(ProposalResult::ConsensusFailed)
	}
}

pub async fn commit(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	payload: Payload,
) -> Result<ProposalResult> {
	// Commit locally
	//
	// Receives command error aftoer committing to KV. Proposals are still committed even if there
	// is a command error since command errors are purely feedback to the client that the command
	// was not applied.
	let cmd_err = {
		let payload = payload.clone();
		ctx.udb()?
			.run(move |tx, _| {
				let payload = payload.clone();
				async move {
					let cmd_err = replica::messages::committed(&*tx, replica_id, &payload)
						.await
						.map_err(|e| universaldb::FdbBindingError::CustomError(e.into()))?;

					Result::Ok(cmd_err)
				}
			})
			.await?
	};

	// EPaxos Step 23
	// Send commits to all replicas (not just quorum members)
	let all_replicas = utils::get_all_replicas(config);
	tokio::spawn({
		let config = config.clone();
		let replica_id = replica_id;
		let all_replicas = all_replicas.to_vec();
		let payload = payload.clone();
		async move {
			let _ = send_commits(&config, replica_id, &all_replicas, &payload).await;
		}
	});

	if let Some(cmd_err) = cmd_err {
		Ok(ProposalResult::CommandError(cmd_err))
	} else {
		Ok(ProposalResult::Committed)
	}
}

async fn send_pre_accepts(
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
					&config,
					to_replica_id,
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

async fn send_accepts(
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
					&config,
					to_replica_id,
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

async fn send_commits(
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
					&config,
					to_replica_id,
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
