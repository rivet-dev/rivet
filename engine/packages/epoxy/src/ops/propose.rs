use anyhow::{Context, Result, bail};
use epoxy_protocol::protocol::{self, CommittedValue, ReplicaId};
use futures_util::{StreamExt, stream::FuturesUnordered};
use gas::prelude::*;
use rand::Rng;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::{
	http_client, metrics,
	replica::{
		ballot::{self, Ballot, BallotSelection},
		commit_kv::{self, CommitKvOutcome},
	},
	utils,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct Proposal {
	pub commands: Vec<Command>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct Command {
	pub kind: CommandKind,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub enum CommandKind {
	SetCommand(SetCommand),
	CheckAndSetCommand(CheckAndSetCommand),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct SetCommand {
	pub key: Vec<u8>,
	pub value: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct CheckAndSetCommand {
	pub key: Vec<u8>,
	pub expect_one_of: Vec<Option<Vec<u8>>>,
	pub new_value: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProposalResult {
	Committed,
	ConsensusFailed { reason: ConsensusFailedReason },
}

impl ProposalResult {
	/// Errors if the result is not `Committed`
	pub fn resolve(&self) -> Result<()> {
		match self {
			ProposalResult::Committed => Ok(()),
			ProposalResult::ConsensusFailed { reason } => match reason {
				ConsensusFailedReason::PreparePhaseConsensusFailed => {
					bail!("proposal failed due to prepare phase consensus failure")
				}
				ConsensusFailedReason::AcceptPhaseConsensusFailed => {
					bail!("proposal failed due to accept phase consensus failure")
				}
				ConsensusFailedReason::StaleBallot => bail!("proposal failed due to stale ballot"),
				ConsensusFailedReason::ExpectedValueDoesNotMatch { .. } => {
					bail!("proposal failed due to value mismatch")
				}
			},
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConsensusFailedReason {
	PreparePhaseConsensusFailed,
	AcceptPhaseConsensusFailed,
	StaleBallot,
	ExpectedValueDoesNotMatch { current_value: Option<Vec<u8>> },
}

#[derive(Debug)]
pub struct Input {
	pub proposal: Proposal,
	pub mutable: bool,
	pub purge_cache: bool,
	/// Optional active-replica scope for this proposal.
	///
	/// Epoxy only validates that the supplied replicas are active and include the local
	/// replica. Callers are responsible for ensuring a given key stays on a stable scope over
	/// time, or that any scope change is handled as an explicit reconfiguration at a higher
	/// layer.
	pub target_replicas: Option<Vec<ReplicaId>>,
}

#[derive(Debug, Clone)]
struct SetProposal {
	key: Vec<u8>,
	value: Option<Vec<u8>>,
	mutable: bool,
}

impl SetProposal {
	fn from_proposal(proposal: &Proposal, mutable: bool) -> Result<Self> {
		if proposal.commands.len() != 1 {
			bail!("epoxy v2 only supports single-command proposals");
		}

		let command = &proposal.commands[0];
		match &command.kind {
			CommandKind::SetCommand(SetCommand { key, value }) => Ok(Self {
				key: key.clone(),
				value: value.clone(),
				mutable,
			}),
			CommandKind::CheckAndSetCommand(CheckAndSetCommand {
				key,
				expect_one_of,
				new_value,
			}) => {
				if expect_one_of.len() != 1 || !matches!(expect_one_of.first(), Some(None)) {
					bail!(
						"epoxy v2 does not support multiple `expect_one_of` values for `CheckAndSet` or `expect_one_of` values that are not `None`"
					)
				}

				Ok(Self {
					key: key.clone(),
					value: new_value.clone(),
					mutable,
				})
			}
		}
	}

	fn result_for_committed_value(&self, current_value: Option<Vec<u8>>) -> ProposalResult {
		if self.mutable {
			if current_value == self.value {
				ProposalResult::Committed
			} else {
				ProposalResult::ConsensusFailed {
					reason: ConsensusFailedReason::ExpectedValueDoesNotMatch { current_value },
				}
			}
		} else if current_value == self.value {
			ProposalResult::Committed
		} else {
			ProposalResult::ConsensusFailed {
				reason: ConsensusFailedReason::ExpectedValueDoesNotMatch { current_value },
			}
		}
	}
}

#[derive(Debug)]
enum PreparePhaseOutcome {
	Prepared {
		ballot: protocol::Ballot,
		value: CommittedValue,
	},
	AlreadyCommitted(Option<Vec<u8>>),
	ConsensusFailed,
}

#[derive(Debug)]
enum PrepareRoundOutcome {
	Promised {
		accepted_value: Option<(Ballot, CommittedValue)>,
	},
	AlreadyCommitted(Option<Vec<u8>>),
	Retry {
		next_ballot: Ballot,
	},
	ConsensusFailed,
}

#[derive(Debug, PartialEq, Eq)]
enum AcceptPhaseOutcome {
	Accepted,
	AlreadyCommitted(Option<Vec<u8>>),
	ConsensusFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AcceptObservation {
	Ok,
	AlreadyCommitted(Option<Vec<u8>>),
	HigherBallot,
	Failed,
}

#[derive(Debug, Clone, Copy)]
struct AcceptRoundState {
	target: usize,
	ok_responses: usize,
	remaining: usize,
}

fn apply_accept_observation(
	state: &mut AcceptRoundState,
	observation: AcceptObservation,
) -> Option<AcceptPhaseOutcome> {
	state.remaining = state.remaining.saturating_sub(1);

	match observation {
		AcceptObservation::Ok => {
			state.ok_responses += 1;
			if state.ok_responses >= state.target {
				return Some(AcceptPhaseOutcome::Accepted);
			}
		}
		AcceptObservation::AlreadyCommitted(value) => {
			return Some(AcceptPhaseOutcome::AlreadyCommitted(value));
		}
		AcceptObservation::HigherBallot | AcceptObservation::Failed => {}
	}

	if state.ok_responses + state.remaining < state.target {
		Some(AcceptPhaseOutcome::ConsensusFailed)
	} else {
		None
	}
}

const PREPARE_RETRY_INITIAL_DELAY_MS: u64 = 10;
const PREPARE_RETRY_MAX_DELAY_MS: u64 = 1_000;
const PREPARE_RETRY_MAX_ATTEMPTS: usize = 10;

#[operation]
pub async fn epoxy_propose(ctx: &OperationCtx, input: &Input) -> Result<ProposalResult> {
	let start = Instant::now();
	let replica_id = ctx.config().epoxy_replica_id();
	let proposal = SetProposal::from_proposal(&input.proposal, input.mutable)?;
	let mut used_slow_path = false;

	let config = ctx
		.udb()?
		.run(|tx| async move { utils::read_config(&tx, replica_id).await })
		.custom_instrument(tracing::info_span!("read_config_tx"))
		.await
		.context("failed reading config")?;

	let quorum_members = utils::resolve_active_quorum_members(
		&config,
		replica_id,
		input.target_replicas.as_deref(),
	)?;
	tracing::debug!(
		?quorum_members,
		quorum_size = quorum_members.len(),
		scoped = input.target_replicas.is_some(),
		"resolved quorum members for proposal"
	);

	let result = match ctx
		.udb()?
		.run(|tx| {
			let key = proposal.key.clone();
			let mutable = proposal.mutable;
			async move { ballot::ballot_selection(&tx, replica_id, key, mutable).await }
		})
		.custom_instrument(tracing::info_span!("ballot_selection_tx"))
		.await
		.context("failed selecting ballot")?
	{
		BallotSelection::AlreadyCommitted(value) => proposal.result_for_committed_value(value),
		BallotSelection::AlreadyCommittedMutable {
			value: committed_value,
			ballot,
		} => {
			let version = committed_value
				.version
				.checked_add(1)
				.context("epoxy mutable key version overflow")?;

			used_slow_path = true;
			metrics::SLOW_PATH_TOTAL.inc();
			metrics::PREPARE_TOTAL.inc();
			match run_prepare_phase(
				ctx,
				&config,
				replica_id,
				&quorum_members,
				proposal.key.clone(),
				CommittedValue {
					value: proposal.value.clone(),
					version,
					mutable: true,
				},
				ballot,
			)
			.await?
			{
				PreparePhaseOutcome::Prepared { ballot, value } => {
					run_slow_path(
						ctx,
						&config,
						replica_id,
						&quorum_members,
						&proposal,
						ballot,
						value,
						input.purge_cache,
					)
					.await?
				}
				PreparePhaseOutcome::AlreadyCommitted(value) => {
					proposal.result_for_committed_value(value)
				}
				PreparePhaseOutcome::ConsensusFailed => ProposalResult::ConsensusFailed {
					reason: ConsensusFailedReason::PreparePhaseConsensusFailed,
				},
			}
		}
		BallotSelection::FreshBallot(ballot) => {
			metrics::FAST_PATH_TOTAL.inc();
			run_fast_path(
				ctx,
				&config,
				replica_id,
				&quorum_members,
				&proposal,
				ballot.into(),
				CommittedValue {
					value: proposal.value.clone(),
					version: 1,
					mutable: proposal.mutable,
				},
				input.purge_cache,
			)
			.await?
		}
		BallotSelection::NeedsPrepare { ballot } => {
			used_slow_path = true;
			metrics::SLOW_PATH_TOTAL.inc();
			metrics::PREPARE_TOTAL.inc();
			match run_prepare_phase(
				ctx,
				&config,
				replica_id,
				&quorum_members,
				proposal.key.clone(),
				CommittedValue {
					value: proposal.value.clone(),
					version: 1,
					mutable: proposal.mutable,
				},
				ballot,
			)
			.await?
			{
				PreparePhaseOutcome::Prepared { ballot, value } => {
					run_slow_path(
						ctx,
						&config,
						replica_id,
						&quorum_members,
						&proposal,
						ballot,
						value,
						input.purge_cache,
					)
					.await?
				}
				PreparePhaseOutcome::AlreadyCommitted(value) => {
					proposal.result_for_committed_value(value)
				}
				PreparePhaseOutcome::ConsensusFailed => ProposalResult::ConsensusFailed {
					reason: ConsensusFailedReason::PreparePhaseConsensusFailed,
				},
			}
		}
	};

	metrics::PROPOSAL_DURATION.observe(start.elapsed().as_secs_f64());
	metrics::record_proposal_result(match (&result, used_slow_path) {
		(ProposalResult::Committed, true) => "slow_path",
		(ProposalResult::Committed, false) => "committed",
		_ => "failed",
	});

	Ok(result)
}

async fn run_fast_path(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	proposal: &SetProposal,
	ballot: protocol::Ballot,
	chosen_value: CommittedValue,
	purge_cache: bool,
) -> Result<ProposalResult> {
	run_accept_path(
		ctx,
		config,
		replica_id,
		quorum_members,
		proposal,
		ballot,
		chosen_value,
		purge_cache,
		utils::QuorumType::Fast,
	)
	.await
}

async fn run_slow_path(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	proposal: &SetProposal,
	ballot: protocol::Ballot,
	chosen_value: CommittedValue,
	purge_cache: bool,
) -> Result<ProposalResult> {
	run_accept_path(
		ctx,
		config,
		replica_id,
		quorum_members,
		proposal,
		ballot,
		chosen_value,
		purge_cache,
		utils::QuorumType::Slow,
	)
	.await
}

async fn run_accept_path(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	proposal: &SetProposal,
	ballot: protocol::Ballot,
	chosen_value: CommittedValue,
	purge_cache: bool,
	accept_quorum: utils::QuorumType,
) -> Result<ProposalResult> {
	match send_accept_round(
		ctx,
		config,
		replica_id,
		quorum_members,
		proposal.key.clone(),
		chosen_value.clone(),
		ballot.clone(),
		accept_quorum,
	)
	.await?
	{
		AcceptPhaseOutcome::Accepted => {}
		AcceptPhaseOutcome::AlreadyCommitted(value) => {
			return Ok(proposal.result_for_committed_value(value));
		}
		AcceptPhaseOutcome::ConsensusFailed => {
			return Ok(ProposalResult::ConsensusFailed {
				reason: ConsensusFailedReason::AcceptPhaseConsensusFailed,
			});
		}
	}

	let commit_result = ctx
		.udb()?
		.run(|tx| {
			let key = proposal.key.clone();
			let value = chosen_value.clone();
			let ballot = ballot.clone();
			async move {
				let CommittedValue {
					value,
					version,
					mutable,
				} = value;
				commit_kv::commit_kv(&tx, replica_id, key, value, ballot, mutable, version).await
			}
		})
		.custom_instrument(tracing::info_span!("commit_kv_tx"))
		.await
		.context("failed committing locally")?;

	match commit_result {
		CommitKvOutcome::Committed => {
			// Broadcast is fire-and-forget. The local commit already succeeded, so
			// propagation failures should not fail the proposal.
			tokio::spawn({
				let ctx = ctx.clone();
				let config = config.clone();
				let key = proposal.key.clone();
				let chosen_value = chosen_value.clone();
				let ballot = ballot;
				let purge_cache = purge_cache && proposal.mutable;
				async move {
					if let Err(err) = broadcast_commits(
						&ctx,
						&config,
						replica_id,
						key.clone(),
						chosen_value.clone(),
						ballot,
					)
					.await
					{
						tracing::warn!(?err, "commit broadcast failed after local commit");
					}

					if purge_cache {
						if let Err(err) = broadcast_cache_purges(
							&ctx,
							&config,
							replica_id,
							vec![protocol::KvPurgeCacheEntry {
								key,
								version: chosen_value.version,
							}],
						)
						.await
						{
							tracing::warn!(?err, "cache purge broadcast failed after local commit");
						}
					}
				}
			});
			Ok(ProposalResult::Committed)
		}
		CommitKvOutcome::AlreadyCommitted { value, .. } => {
			Ok(proposal.result_for_committed_value(value))
		}
		CommitKvOutcome::StaleBallot { .. } => Ok(ProposalResult::ConsensusFailed {
			reason: ConsensusFailedReason::StaleBallot,
		}),
	}
}

async fn run_prepare_phase(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	replica_id: ReplicaId,
	quorum_members: &[ReplicaId],
	key: Vec<u8>,
	proposed_value: CommittedValue,
	initial_ballot: Ballot,
) -> Result<PreparePhaseOutcome> {
	let mut request_ballot = initial_ballot;
	let mut retry_count = 0;

	loop {
		match send_prepare_round(
			ctx,
			config,
			replica_id,
			quorum_members,
			key.clone(),
			proposed_value.clone(),
			request_ballot.into(),
		)
		.await?
		{
			PrepareRoundOutcome::Promised { accepted_value } => {
				let value = accepted_value
					.map(|(_, value)| value)
					.unwrap_or_else(|| proposed_value.clone());
				return Ok(PreparePhaseOutcome::Prepared {
					ballot: request_ballot.into(),
					value,
				});
			}
			PrepareRoundOutcome::AlreadyCommitted(value) => {
				return Ok(PreparePhaseOutcome::AlreadyCommitted(value));
			}
			PrepareRoundOutcome::Retry { next_ballot } => {
				store_prepare_ballot(ctx, replica_id, key.clone(), next_ballot).await?;
				let Some(retry_delay) =
					next_prepare_retry_delay(retry_count, &mut rand::thread_rng())
				else {
					tracing::warn!(
						%replica_id,
						key=hex::encode(&key),
						retry_count,
						"prepare phase exceeded retry limit"
					);
					return Ok(PreparePhaseOutcome::ConsensusFailed);
				};

				metrics::record_prepare_retry();
				tracing::info!(
					%replica_id,
					key=hex::encode(&key),
					retry_count,
					retry_delay_ms = retry_delay.as_millis() as u64,
					next_ballot_counter = next_ballot.counter,
					next_ballot_replica_id = next_ballot.replica_id,
					"retrying prepare phase after contention"
				);
				tokio::time::sleep(retry_delay).await;
				request_ballot = next_ballot;
				retry_count += 1;
			}
			PrepareRoundOutcome::ConsensusFailed => {
				return Ok(PreparePhaseOutcome::ConsensusFailed);
			}
		}
	}
}

async fn send_prepare_round(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	key: Vec<u8>,
	proposed_value: CommittedValue,
	ballot: protocol::Ballot,
) -> Result<PrepareRoundOutcome> {
	let target = utils::calculate_quorum(replica_ids.len(), utils::QuorumType::Slow);
	let mut pending =
		futures_util::stream::iter(replica_ids.iter().copied().map(|to_replica_id| {
			let key = key.clone();
			let proposed_value = proposed_value.clone();
			let ballot = ballot.clone();
			async move {
				(
					to_replica_id,
					tokio::time::timeout(
						crate::consts::REQUEST_TIMEOUT,
						send_prepare_request(
							ctx,
							config,
							from_replica_id,
							to_replica_id,
							key,
							proposed_value,
							ballot,
						),
					)
					.await,
				)
			}
		}))
		.collect::<FuturesUnordered<_>>()
		.await;

	let mut ok_responses = 0;
	let mut remaining = replica_ids.len();
	let mut highest_accepted: Option<(Ballot, CommittedValue)> = None;
	let mut highest_rejected_ballot: Option<Ballot> = None;

	while let Some((to_replica_id, response)) = pending.next().await {
		remaining -= 1;

		match response {
			Ok(Ok(protocol::PrepareResponse::PrepareResponseOk(ok))) => {
				ok_responses += 1;

				match (ok.accepted_value, ok.accepted_ballot) {
					(Some(value), Some(ballot)) => {
						let protocol::CommittedValue {
							value,
							version,
							mutable,
						} = value;
						let ballot = Ballot::from(ballot);
						let replace = highest_accepted
							.as_ref()
							.map(|(current_ballot, _)| ballot > *current_ballot)
							.unwrap_or(true);
						if replace {
							highest_accepted = Some((
								ballot,
								CommittedValue {
									value,
									version,
									mutable,
								},
							));
						}
					}
					(None, None) => {}
					_ => {
						bail!(
							"prepare response from replica {to_replica_id} returned partial accepted state"
						);
					}
				}

				if ok_responses >= target {
					return Ok(PrepareRoundOutcome::Promised {
						accepted_value: highest_accepted,
					});
				}
			}
			Ok(Ok(protocol::PrepareResponse::PrepareResponseAlreadyCommitted(committed))) => {
				return Ok(PrepareRoundOutcome::AlreadyCommitted(committed.value));
			}
			Ok(Ok(protocol::PrepareResponse::PrepareResponseHigherBallot(rejected))) => {
				let rejected_ballot = Ballot::from(rejected.ballot);
				let replace = highest_rejected_ballot
					.map(|current| rejected_ballot > current)
					.unwrap_or(true);
				if replace {
					highest_rejected_ballot = Some(rejected_ballot);
				}
			}
			Ok(Err(err)) => {
				tracing::warn!(?err, %to_replica_id, "prepare request failed");
			}
			Err(err) => {
				tracing::warn!(?err, %to_replica_id, "prepare request timed out");
			}
		}

		if ok_responses + remaining < target {
			return if let Some(rejected_ballot) = highest_rejected_ballot {
				Ok(PrepareRoundOutcome::Retry {
					next_ballot: next_ballot(rejected_ballot, from_replica_id)?,
				})
			} else {
				Ok(PrepareRoundOutcome::ConsensusFailed)
			};
		}
	}

	if ok_responses >= target {
		Ok(PrepareRoundOutcome::Promised {
			accepted_value: highest_accepted,
		})
	} else if let Some(rejected_ballot) = highest_rejected_ballot {
		Ok(PrepareRoundOutcome::Retry {
			next_ballot: next_ballot(rejected_ballot, from_replica_id)?,
		})
	} else {
		Ok(PrepareRoundOutcome::ConsensusFailed)
	}
}

async fn send_accept_round(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	replica_ids: &[ReplicaId],
	key: Vec<u8>,
	value: CommittedValue,
	ballot: protocol::Ballot,
	accept_quorum: utils::QuorumType,
) -> Result<AcceptPhaseOutcome> {
	let target = utils::calculate_quorum(replica_ids.len(), accept_quorum);
	let mut pending =
		futures_util::stream::iter(replica_ids.iter().copied().map(|to_replica_id| {
			let key = key.clone();
			let value = value.clone();
			let ballot = ballot.clone();
			async move {
				(
					to_replica_id,
					tokio::time::timeout(
						crate::consts::REQUEST_TIMEOUT,
						send_accept_request(
							ctx,
							config,
							from_replica_id,
							to_replica_id,
							key,
							value,
							ballot,
						),
					)
					.await,
				)
			}
		}))
		.collect::<FuturesUnordered<_>>()
		.await;

	let mut state = AcceptRoundState {
		target,
		ok_responses: 0,
		remaining: replica_ids.len(),
	};

	while let Some((to_replica_id, response)) = pending.next().await {
		let observation = match response {
			Ok(Ok(protocol::AcceptResponse::AcceptResponseOk(_))) => AcceptObservation::Ok,
			Ok(Ok(protocol::AcceptResponse::AcceptResponseAlreadyCommitted(committed))) => {
				AcceptObservation::AlreadyCommitted(committed.value)
			}
			Ok(Ok(protocol::AcceptResponse::AcceptResponseHigherBallot(_))) => {
				AcceptObservation::HigherBallot
			}
			Ok(Err(err)) => {
				tracing::warn!(?err, %to_replica_id, "accept request failed");
				AcceptObservation::Failed
			}
			Err(err) => {
				tracing::warn!(?err, %to_replica_id, "accept request timed out");
				AcceptObservation::Failed
			}
		};

		if let Some(outcome) = apply_accept_observation(&mut state, observation) {
			return Ok(outcome);
		}
	}

	if state.ok_responses >= target {
		Ok(AcceptPhaseOutcome::Accepted)
	} else {
		Ok(AcceptPhaseOutcome::ConsensusFailed)
	}
}

async fn send_prepare_request(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	to_replica_id: ReplicaId,
	key: Vec<u8>,
	proposed_value: CommittedValue,
	ballot: protocol::Ballot,
) -> Result<protocol::PrepareResponse> {
	let response = http_client::send_message(
		&ApiCtx::new_from_operation(ctx)?,
		config,
		protocol::Request {
			from_replica_id,
			to_replica_id,
			kind: protocol::RequestKind::PrepareRequest(protocol::PrepareRequest {
				key,
				ballot,
				mutable: proposed_value.mutable,
				version: proposed_value.version,
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

	Ok(response)
}

async fn send_accept_request(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	to_replica_id: ReplicaId,
	key: Vec<u8>,
	value: CommittedValue,
	ballot: protocol::Ballot,
) -> Result<protocol::AcceptResponse> {
	let CommittedValue {
		value,
		version,
		mutable,
	} = value;
	let response = http_client::send_message(
		&ApiCtx::new_from_operation(ctx)?,
		config,
		protocol::Request {
			from_replica_id,
			to_replica_id,
			kind: protocol::RequestKind::AcceptRequest(protocol::AcceptRequest {
				key,
				value,
				ballot,
				mutable,
				version,
			}),
		},
	)
	.await?;

	let protocol::Response {
		kind: protocol::ResponseKind::AcceptResponse(response),
	} = response
	else {
		bail!("wrong response type");
	};

	Ok(response)
}

async fn broadcast_commits(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	key: Vec<u8>,
	value: CommittedValue,
	ballot: protocol::Ballot,
) -> Result<()> {
	let all_replicas = utils::get_all_replicas(config);

	http_client::fanout_to_replicas(
		from_replica_id,
		&all_replicas,
		utils::QuorumType::All,
		|to_replica_id| {
			let config = config.clone();
			let key = key.clone();
			let value = value.clone();
			let ballot = ballot.clone();
			async move {
				let CommittedValue {
					value,
					version,
					mutable,
				} = value;
				let response = http_client::send_message(
					&ApiCtx::new_from_operation(ctx)?,
					&config,
					protocol::Request {
						from_replica_id,
						to_replica_id,
						kind: protocol::RequestKind::CommitRequest(protocol::CommitRequest {
							key,
							value,
							ballot,
							mutable,
							version,
						}),
					},
				)
				.await?;

				let protocol::Response {
					kind: protocol::ResponseKind::CommitResponse(_),
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

async fn broadcast_cache_purges(
	ctx: &OperationCtx,
	config: &protocol::ClusterConfig,
	from_replica_id: ReplicaId,
	entries: Vec<protocol::KvPurgeCacheEntry>,
) -> Result<()> {
	let all_replicas = utils::get_all_replicas(config);

	http_client::fanout_to_replicas(
		from_replica_id,
		&all_replicas,
		utils::QuorumType::All,
		|to_replica_id| {
			let config = config.clone();
			let entries = entries.clone();
			async move {
				let response = http_client::send_message(
					&ApiCtx::new_from_operation(ctx)?,
					&config,
					protocol::Request {
						from_replica_id,
						to_replica_id,
						kind: protocol::RequestKind::KvPurgeCacheRequest(
							protocol::KvPurgeCacheRequest { entries },
						),
					},
				)
				.await?;

				let protocol::Response {
					kind: protocol::ResponseKind::KvPurgeCacheResponse,
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

fn next_ballot(ballot: Ballot, replica_id: ReplicaId) -> Result<Ballot> {
	metrics::record_ballot_bump();

	let counter = ballot
		.counter
		.checked_add(1)
		.context("ballot counter overflow")?;
	Ok(Ballot::new(counter, replica_id))
}

async fn store_prepare_ballot(
	ctx: &OperationCtx,
	replica_id: ReplicaId,
	key: Vec<u8>,
	ballot: Ballot,
) -> Result<()> {
	ctx.udb()?
		.run(|tx| {
			let key = key.clone();
			async move { ballot::store_ballot(&tx, replica_id, key, ballot) }
		})
		.custom_instrument(tracing::info_span!("store_prepare_ballot_tx"))
		.await
}

fn next_prepare_retry_delay<R>(retry_count: usize, rng: &mut R) -> Option<Duration>
where
	R: Rng + ?Sized,
{
	if retry_count >= PREPARE_RETRY_MAX_ATTEMPTS {
		return None;
	}

	let base_delay_ms = prepare_retry_base_delay_ms(retry_count);
	let half = base_delay_ms / 2;
	let jitter_ms = rng.gen_range(0..=base_delay_ms);
	// Delay range is [base/2, base*1.5] for better decorrelation between competing proposers.
	Some(Duration::from_millis(half.saturating_add(jitter_ms)))
}

fn prepare_retry_base_delay_ms(retry_count: usize) -> u64 {
	let multiplier = 1_u64
		.checked_shl(retry_count.min(63) as u32)
		.unwrap_or(u64::MAX);
	PREPARE_RETRY_INITIAL_DELAY_MS
		.saturating_mul(multiplier)
		.min(PREPARE_RETRY_MAX_DELAY_MS)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rand::{SeedableRng, rngs::StdRng};

	#[test]
	fn parses_set_command_as_set_proposal() {
		let proposal = Proposal {
			commands: vec![Command {
				kind: CommandKind::SetCommand(SetCommand {
					key: b"key".to_vec(),
					value: Some(b"value".to_vec()),
				}),
			}],
		};

		let parsed = SetProposal::from_proposal(&proposal, false).unwrap();
		assert_eq!(parsed.key, b"key".to_vec());
		assert_eq!(parsed.value, Some(b"value".to_vec()));
		assert!(!parsed.mutable);
	}

	#[test]
	fn rejects_multi_command_proposals() {
		let proposal = Proposal {
			commands: vec![
				Command {
					kind: CommandKind::SetCommand(SetCommand {
						key: b"key".to_vec(),
						value: Some(b"value".to_vec()),
					}),
				},
				Command {
					kind: CommandKind::SetCommand(SetCommand {
						key: b"key2".to_vec(),
						value: Some(b"value2".to_vec()),
					}),
				},
			],
		};

		assert!(SetProposal::from_proposal(&proposal, false).is_err());
	}

	#[test]
	fn prepare_retry_base_delay_doubles_and_caps() {
		assert_eq!(prepare_retry_base_delay_ms(0), 10);
		assert_eq!(prepare_retry_base_delay_ms(1), 20);
		assert_eq!(prepare_retry_base_delay_ms(2), 40);
		assert_eq!(prepare_retry_base_delay_ms(3), 80);
		assert_eq!(prepare_retry_base_delay_ms(4), 160);
		assert_eq!(prepare_retry_base_delay_ms(5), 320);
		assert_eq!(prepare_retry_base_delay_ms(6), 640);
		assert_eq!(prepare_retry_base_delay_ms(7), 1_000);
		assert_eq!(prepare_retry_base_delay_ms(20), 1_000);
	}

	#[test]
	fn prepare_retry_delay_applies_bounded_jitter() {
		let mut rng = StdRng::seed_from_u64(7);

		for retry_count in 0..PREPARE_RETRY_MAX_ATTEMPTS {
			let base_delay_ms = prepare_retry_base_delay_ms(retry_count);
			let delay = next_prepare_retry_delay(retry_count, &mut rng).unwrap();
			let delay_ms = delay.as_millis() as u64;

			// Delay range is [base/2, base*1.5].
			assert!(delay_ms >= base_delay_ms / 2);
			assert!(delay_ms <= base_delay_ms + (base_delay_ms / 2));
		}
	}

	#[test]
	fn prepare_retry_delay_stops_after_maximum_attempts() {
		let mut rng = StdRng::seed_from_u64(11);
		assert!(next_prepare_retry_delay(PREPARE_RETRY_MAX_ATTEMPTS, &mut rng).is_none());
	}

	#[test]
	fn accept_round_tolerates_single_higher_ballot_if_quorum_still_reachable() {
		let mut state = AcceptRoundState {
			target: 2,
			ok_responses: 0,
			remaining: 3,
		};

		assert_eq!(
			apply_accept_observation(&mut state, AcceptObservation::HigherBallot),
			None
		);
		assert_eq!(
			apply_accept_observation(&mut state, AcceptObservation::Ok),
			None
		);
		assert_eq!(
			apply_accept_observation(&mut state, AcceptObservation::Ok),
			Some(AcceptPhaseOutcome::Accepted)
		);
	}

	#[test]
	fn accept_round_fails_once_quorum_becomes_unreachable() {
		let mut state = AcceptRoundState {
			target: 2,
			ok_responses: 0,
			remaining: 3,
		};

		assert_eq!(
			apply_accept_observation(&mut state, AcceptObservation::HigherBallot),
			None
		);
		assert_eq!(
			apply_accept_observation(&mut state, AcceptObservation::Failed),
			Some(AcceptPhaseOutcome::ConsensusFailed)
		);
	}
}
