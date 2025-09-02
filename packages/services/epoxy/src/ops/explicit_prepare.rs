use anyhow::*;
use epoxy_protocol::protocol::{self, ReplicaId};
use gas::prelude::*;
use universaldb::FdbBindingError;

use crate::{http_client, replica, types, utils};

#[derive(Debug)]
pub struct Input {
	pub instance: protocol::Instance,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExplicitPrepareResult {
	/// Command was successfully committed through explicit prepare
	Committed,
	/// Explicit prepare failed (couldn't reach quorum)
	Failed,
	/// Command error occurred during execution
	CommandError(crate::ops::propose::CommandError),
}

#[operation]
pub async fn explicit_prepare(ctx: &OperationCtx, input: &Input) -> Result<ExplicitPrepareResult> {
	let replica_id = ctx.config().epoxy_replica_id();
	let instance = &input.instance;

	tracing::info!(
		?instance,
		"starting explicit prepare for potentially failed replica"
	);

	// Read config
	let config = ctx
		.udb()?
		.run(move |tx, _| async move {
			utils::read_config(&tx, replica_id)
				.await
				.map_err(|x| FdbBindingError::CustomError(x.into()))
		})
		.await?;

	// EPaxos Step 25: Increment ballot number
	let new_ballot = ctx
		.udb()?
		.run(move |tx, _| async move {
			replica::ballot::increment_ballot(&tx, replica_id)
				.await
				.map_err(|x| FdbBindingError::CustomError(x.into()))
		})
		.await?;

	// Get quorum members
	let quorum_members = utils::get_quorum_members(&config);

	// EPaxos Step 26: Send Prepare to all replicas and wait for quorum
	let prepare_responses =
		send_prepares(&config, replica_id, &quorum_members, &new_ballot, instance).await?;

	// Check if we got enough responses for a quorum
	let required_quorum = utils::calculate_quorum(quorum_members.len(), utils::QuorumType::Slow);
	if prepare_responses.len() < required_quorum {
		tracing::warn!(
			received_responses = prepare_responses.len(),
			required_quorum = required_quorum,
			"failed to get quorum for explicit prepare"
		);
		return Ok(ExplicitPrepareResult::Failed);
	}

	// EPaxos Step 27: Find replies with highest ballot number
	let highest_ballot_responses = find_highest_ballot_responses(&prepare_responses);

	// Decide what to do based on the responses
	let result = match analyze_prepare_responses(&highest_ballot_responses, instance) {
		PrepareDecision::Commit(payload) => {
			// EPaxos Step 29: Run Commit phase
			let result =
				crate::ops::propose::commit(ctx, &config, replica_id, &quorum_members, payload)
					.await?;
			convert_proposal_result(result)
		}
		PrepareDecision::Accept(payload) => {
			// EPaxos Step 31: Run Paxos-Accept phase
			let result = crate::ops::propose::run_paxos_accept(
				ctx,
				&config,
				replica_id,
				&quorum_members,
				payload,
			)
			.await?;
			convert_proposal_result(result)
		}
		PrepareDecision::RestartPhase1(commands) => {
			// EPaxos Steps 35-37: Start Phase 1, avoid fast path
			restart_phase1(
				ctx,
				&config,
				replica_id,
				&quorum_members,
				instance,
				commands,
			)
			.await?
		}
	};

	Ok(result)
}

fn convert_proposal_result(result: crate::ops::propose::ProposalResult) -> ExplicitPrepareResult {
	match result {
		crate::ops::propose::ProposalResult::Committed => ExplicitPrepareResult::Committed,
		crate::ops::propose::ProposalResult::ConsensusFailed => ExplicitPrepareResult::Failed,
		crate::ops::propose::ProposalResult::CommandError(cmd_err) => {
			ExplicitPrepareResult::CommandError(cmd_err)
		}
	}
}

enum PrepareDecision {
	Commit(protocol::Payload),
	Accept(protocol::Payload),
	RestartPhase1(Option<Vec<protocol::Command>>),
}

fn analyze_prepare_responses(
	responses: &[&protocol::PrepareOk],
	instance: &protocol::Instance,
) -> PrepareDecision {
	// EPaxos Step 28: Check if any response contains committed command
	for response in responses {
		if let Some(data) = &response.data {
			if data.state == protocol::State::Committed {
				let payload = protocol::Payload {
					proposal: protocol::Proposal {
						commands: data.commands.clone(),
					},
					seq: data.seq,
					deps: data.deps.clone(),
					instance: instance.clone(),
				};
				return PrepareDecision::Commit(payload);
			}
		}
	}

	// EPaxos Step 30: Check if any response contains accepted command
	for response in responses {
		if let Some(data) = &response.data {
			if data.state == protocol::State::Accepted {
				let payload = protocol::Payload {
					proposal: protocol::Proposal {
						commands: data.commands.clone(),
					},
					seq: data.seq,
					deps: data.deps.clone(),
					instance: instance.clone(),
				};
				return PrepareDecision::Accept(payload);
			}
		}
	}

	// EPaxos Step 32: Check for majority of identical pre-accepted replies from default ballot
	let pre_accepted_responses: Vec<_> = responses
		.iter()
		.filter_map(|r| {
			r.data.as_ref().filter(|data| {
				data.state == protocol::State::PreAccepted
					&& data.ballot.epoch == 0
					&& data.ballot.ballot == 0
					&& data.ballot.replica_id == instance.replica_id
			})
		})
		.collect();

	if pre_accepted_responses.len() >= responses.len() / 2 + 1 {
		// Check if all pre-accepted responses are identical and none from original replica
		if let Some(first) = pre_accepted_responses.first() {
			let all_identical = pre_accepted_responses.iter().all(|r| {
				r.commands == first.commands && r.seq == first.seq && r.deps == first.deps
			});

			let none_from_original = responses
				.iter()
				.all(|r| r.previous_ballot.replica_id != instance.replica_id);

			if all_identical && none_from_original {
				let payload = protocol::Payload {
					proposal: protocol::Proposal {
						commands: first.commands.clone(),
					},
					seq: first.seq,
					deps: first.deps.clone(),
					instance: instance.clone(),
				};
				// EPaxos Step 33
				return PrepareDecision::Accept(payload);
			}
		}
	}

	// EPaxos Step 34: Check if any response contains pre-accepted command
	for response in responses {
		if let Some(data) = &response.data {
			if data.state == protocol::State::PreAccepted {
				// EPaxos Step 35
				return PrepareDecision::RestartPhase1(Some(data.commands.clone()));
			}
		}
	}

	// EPaxos Step 36: No responses or no valid commands - restart with no-op
	// EPaxos Step 37: Start Phase 1 for no-op at L.i, avoid fast path
	PrepareDecision::RestartPhase1(None)
}

fn find_highest_ballot_responses(responses: &[protocol::PrepareOk]) -> Vec<&protocol::PrepareOk> {
	if responses.is_empty() {
		return Vec::new();
	}

	let highest_ballot = responses
		.iter()
		.map(|r| &r.previous_ballot)
		.max_by(|a, b| compare_ballots(a, b))
		.unwrap();

	responses
		.iter()
		.filter(|r| {
			compare_ballots(&r.previous_ballot, highest_ballot) == std::cmp::Ordering::Equal
		})
		.collect()
}

fn compare_ballots(a: &protocol::Ballot, b: &protocol::Ballot) -> std::cmp::Ordering {
	match a.epoch.cmp(&b.epoch) {
		std::cmp::Ordering::Equal => match a.ballot.cmp(&b.ballot) {
			std::cmp::Ordering::Equal => a.replica_id.cmp(&b.replica_id),
			other => other,
		},
		other => other,
	}
}

async fn send_prepares(
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	ballot: &protocol::Ballot,
	instance: &protocol::Instance,
) -> Result<Vec<protocol::PrepareOk>> {
	let responses = http_client::fanout_to_replicas(
		from_replica_id,
		replica_ids,
		utils::QuorumType::Slow,
		|to_replica_id| {
			let config = config.clone();
			let ballot = ballot.clone();
			let instance = instance.clone();
			async move {
				let response = http_client::send_message(
					&config,
					to_replica_id,
					protocol::Request {
						from_replica_id,
						to_replica_id,
						kind: protocol::RequestKind::PrepareRequest(protocol::PrepareRequest {
							ballot,
							instance,
						}),
					},
				)
				.await?;

				let protocol::Response {
					kind: protocol::ResponseKind::PrepareResponse(response),
				} = response
				else {
					bail!("wrong response type");
				};

				match response {
					protocol::PrepareResponse::PrepareOk(ok) => Ok(ok),
					protocol::PrepareResponse::PrepareNack(_nack) => {
						bail!("received NACK for prepare request");
					}
				}
			}
		},
	)
	.await?;

	Ok(responses)
}

async fn restart_phase1(
	ctx: &OperationCtx,
	_config: &protocol::ClusterConfig,
	_replica_id: ReplicaId,
	_quorum_members: &[ReplicaId],
	instance: &protocol::Instance,
	commands: Option<Vec<protocol::Command>>,
) -> Result<ExplicitPrepareResult> {
	// Create proposal with provided commands or no-op
	let proposal = protocol::Proposal {
		commands: commands.unwrap_or_else(|| vec![]), // Empty vec for no-op
	};

	tracing::info!(
		?instance,
		commands_count = proposal.commands.len(),
		"restarting phase1 with propose operation"
	);

	// Call the propose operation to restart consensus from Phase 1
	let result = ctx.op(crate::ops::propose::Input { proposal }).await?;

	// Convert ProposalResult to ExplicitPrepareResult
	match result {
		crate::ops::propose::ProposalResult::Committed => Ok(ExplicitPrepareResult::Committed),
		crate::ops::propose::ProposalResult::ConsensusFailed => Ok(ExplicitPrepareResult::Failed),
		crate::ops::propose::ProposalResult::CommandError(cmd_err) => {
			Ok(ExplicitPrepareResult::CommandError(cmd_err))
		}
	}
}
