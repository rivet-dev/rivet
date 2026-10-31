//! One deterministic state machine sequences writes and counts disk replica evidence.

pub mod effect;
pub mod event;
pub mod executor;
mod lifecycle;
mod membership;
mod metrics;
mod pipeline;
mod recovery;
mod recovery_only;
mod repair;
mod requests;
mod retained;
pub mod runner;
pub mod snapshot;
pub mod state;
pub mod types;
mod waiting;

pub use self::lifecycle::open_refusal;

use self::{
	effect::VolumeClientEffect,
	event::VolumeClientEvent,
	snapshot::{LoadedVolumeReplicaView, VolumeClientSnapshot},
	state::{VolumeClientPhase, VolumeClientState},
	types::VolumeClientTimer,
};
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};

pub type Effects = rivet_state_machine_ephemeral::Effects<VolumeClientEffect, VolumeClientTimer>;

pub struct VolumeClientMachine;

impl EphemeralStateMachine for VolumeClientMachine {
	type Slot = NoSlot;
	type State = VolumeClientState;
	type Event = VolumeClientEvent;
	type Effect = VolumeClientEffect;
	type Snapshot = VolumeClientSnapshot;
	type Timer = VolumeClientTimer;
	const DEBUG_NAME: &'static str = "volume_client_machine";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			VolumeClientTimer::Waiting(..) => "waiting",
			VolumeClientTimer::OpenIdle => "open_idle",
			VolumeClientTimer::FinishRecovery => "finish_recovery",
			VolumeClientTimer::Quorum(..) => "quorum",
			VolumeClientTimer::Stall => "stall",
			VolumeClientTimer::Control(..) => "control",
			VolumeClientTimer::Append(..) => "append",
			VolumeClientTimer::Resume(..) => "resume",
			VolumeClientTimer::ReplayResume(..) => "replay_resume",
		}
	}

	/// Starts the initial open without admitting writes early.
	fn start(state: &mut Self::State, _: Clocks, entropy: Entropy, effects: &mut Effects) {
		if matches!(state.phase, VolumeClientPhase::Opening { .. }) {
			// A create's reply serves as this open only when it claimed the volume for this
			// client. A create of a volume that already existed claims nothing, so the client
			// opens, and claims, the volume itself.
			if let Some(opened) = state
				.initial_open
				.take()
				.filter(|opened| opened.open_claim.is_some())
			{
				let op = state.operation();
				state.open_attempts.insert(op);
				state.phase = VolumeClientPhase::Opening { op_id: Some(op) };
				let kept = state.initial_open_kept;
				lifecycle::opened(state, op, Ok(opened), kept, entropy, effects);
			} else {
				state.client.client_incarnation = *entropy.rng().uuid_v4().as_bytes();
				lifecycle::open(state, effects);
			}
			effects.arm_timer_after(VolumeClientTimer::OpenIdle, state.config.quorum_timeout_ms);
		}
	}

	/// Apply one completion or admission, then fill every replica's bounded send window.
	fn on_event(
		state: &mut Self::State,
		event: EphemeralStateMachineEvent<Self>,
		effects: &mut Effects,
	) {
		match event.body {
			RuntimeEvent::Input(VolumeClientEvent::OpenRequest { request_id }) => {
				waiting::admit(state, request_id, waiting::Request::Open, effects)
			}
			RuntimeEvent::Timer(VolumeClientTimer::Waiting(request_id)) => {
				waiting::expired(state, request_id, effects)
			}
			RuntimeEvent::Timer(VolumeClientTimer::OpenIdle) => waiting::abandon(state, effects),
			RuntimeEvent::Input(VolumeClientEvent::Fence) => pipeline::fence(state, effects),
			RuntimeEvent::Input(VolumeClientEvent::ReplicaSetObserved { replica_set }) => {
				match state.phase {
					VolumeClientPhase::Appending
					| VolumeClientPhase::Recovering(_)
					| VolumeClientPhase::Draining { .. } => {
						membership::adopt(state, replica_set, effects);
					}
					VolumeClientPhase::Opening { .. }
					| VolumeClientPhase::Closing { .. }
					| VolumeClientPhase::Stopped
					| VolumeClientPhase::Fenced { .. } => {}
				}
			}
			RuntimeEvent::Timer(VolumeClientTimer::FinishRecovery) => {
				recovery_only::finish(state, true, effects)
			}
			RuntimeEvent::Input(VolumeClientEvent::WaitForCommitRequest {
				request_id,
				history,
			}) => requests::barrier(state, request_id, false, history, effects),
			RuntimeEvent::Input(VolumeClientEvent::ConfirmRequest { request_id }) => {
				requests::barrier(state, request_id, true, None, effects)
			}
			RuntimeEvent::Timer(VolumeClientTimer::Quorum(request_id)) => {
				requests::expired(state, request_id, effects);
				waiting::abandon(state, effects);
			}
			RuntimeEvent::Timer(VolumeClientTimer::Stall) => pipeline::stalled(state, effects),
			RuntimeEvent::Input(VolumeClientEvent::ReadVolumeLogEntriesResponse {
				op_id,
				worker_id,
				result,
			}) => recovery::read(state, op_id, &worker_id, result, event.entropy, effects),
			RuntimeEvent::Input(VolumeClientEvent::MatchResponse {
				op_id,
				worker_id,
				result,
			}) => recovery::matched(state, op_id, &worker_id, result, event.entropy, effects),
			RuntimeEvent::Input(VolumeClientEvent::VolumeOpenResponse { op_id, result }) => {
				lifecycle::opened(state, op_id, result, false, event.entropy, effects)
			}
			RuntimeEvent::Input(VolumeClientEvent::ClaimResponse {
				op_id,
				worker_id,
				result,
			}) => lifecycle::claimed(state, op_id, worker_id, result, event.entropy, effects),
			RuntimeEvent::Input(VolumeClientEvent::CloseRequest { request_id }) => {
				lifecycle::close(state, request_id, effects)
			}
			RuntimeEvent::Input(VolumeClientEvent::VolumeCloseResponse { op_id, result }) => {
				lifecycle::closed(state, op_id, result, effects)
			}
			RuntimeEvent::Timer(VolumeClientTimer::Control(op)) => {
				lifecycle::expired(state, op, event.entropy, effects)
			}
			RuntimeEvent::Input(VolumeClientEvent::WriteRequest {
				request_id,
				payload,
				payload_digest,
				retry_of,
				encoded_bytes,
				sync,
			}) => {
				waiting::admit(
					state,
					request_id,
					waiting::Request::Write {
						request_uuid: *event.entropy.rng().uuid_v4().as_bytes(),
						payload,
						encoded_bytes,
						sync,
						payload_digest,
						retry_of,
					},
					effects,
				);
			}
			RuntimeEvent::Input(
				VolumeClientEvent::AppendResponse {
					op_id,
					worker_id,
					result,
				}
				| VolumeClientEvent::AdvanceCommittedResponse {
					op_id,
					worker_id,
					result,
				},
			) => pipeline::answered(state, op_id, &worker_id, result, event.entropy, effects),
			RuntimeEvent::Timer(VolumeClientTimer::Append(op_id)) => {
				pipeline::expired(state, op_id, effects)
			}
			RuntimeEvent::Timer(VolumeClientTimer::ReplayResume(op_id)) => {
				recovery::replay_resumed(state, op_id, effects)
			}
			RuntimeEvent::Timer(VolumeClientTimer::Resume(op_id)) => {
				pipeline::resume(state, op_id, effects)
			}
		}

		retained::release_recovery_payloads(state, effects);
		retained::release_committed_payloads(state, effects);
		// A blocked quorum can form without a new block: a payload release can unmatch a
		// blocked replica, and a membership change can drop one. Check after every event.
		pipeline::fail_blocked(state, effects);
		// Drive repair before sends, then release barriers and finish any pending close.
		recovery::match_replicas(state, effects);
		let matching = state.phase == VolumeClientPhase::Recovering(state::RecoveryPhase::Matching);
		recovery::end_matching(state, effects);
		waiting::dispatch(state, matching, effects);
		pipeline::send(state, effects);
		requests::barriers(state, effects);
		recovery_only::finish(state, false, effects);
		lifecycle::drain(state, effects);
	}

	fn snapshot(state: &Self::State) -> Self::Snapshot {
		VolumeClientSnapshot {
			max_mutation_bytes: state.max_mutation_bytes(),
			volume_log_key: match &state.volume {
				state::VolumeIdentity::Unresolved { .. } => None,
				state::VolumeIdentity::Known(key) => Some(key.clone()),
			},
			purpose: state.purpose,
			cold: state.cold.clone(),
			pending_opens: state.open_attempts.len() as u32,
			phase: state.phase.clone(),
			client: state.client.clone(),
			replica_set: state.replica_set.clone(),
			next_volume_seq: state.next_volume_seq,
			committed: state.commit_publishing.committed,
			claim_entry_volume_seq: state.claim_entry.map(|claim_entry| claim_entry.volume_seq),
			claim_entry_written_at_unix_ms: state
				.claim_entry
				.map(|claim_entry| claim_entry.written_at_unix_ms),
			has_base_chain: state.has_base_chain,
			granted_claim: state.granted_claim,
			close_acknowledged: state.close_acknowledged,
			opened_empty: state.opened_empty,
			created: state.created.clone(),
			read_back: state.read_back.clone(),
			last_client_seq: state.last_client_seq,
			history_blocked: pipeline::wedged(state).map(|(volume_seq, refusal)| {
				state::BlockedAppend {
					volume_seq,
					refusal,
				}
			}),
			pending_writes: (state.writes.len()
				+ state
					.waiting
					.values()
					.filter(|c| matches!(c.request, waiting::Request::Write { .. }))
					.count()) as u32,
			memory_bytes: state.memory_bytes + waiting::reserved_bytes(state),
			dirty_bytes: state.above_cold.bytes(),
			config: state.config.clone(),
			replicas: state
				.replicas
				.iter()
				.map(|(worker_id, r)| LoadedVolumeReplicaView {
					worker_id: worker_id.clone(),
					acked: r.acked,
					claimed: r
						.claim
						.as_ref()
						.is_some_and(|p| p.claimed_client.as_ref() == Some(&state.client)),
					counted: state.claim_quorum.contains(worker_id),
					matched: r.matched,
					replica_incarnation: r.incarnation,
					status: r.claim.as_ref().map(|p| p.status.clone()),
					repair: r.repair.clone(),
					in_flight: r.in_flight.len() as u32,
					blocked: r.blocked,
				})
				.collect(),
		}
	}

	fn debug_validate(state: &Self::State) -> anyhow::Result<()> {
		membership::validate(&state.replica_set)?;
		anyhow::ensure!(
			state
				.replicas
				.keys()
				.cloned()
				.collect::<std::collections::BTreeSet<_>>()
				== membership::destinations(&state.replica_set),
			"replica map disagrees with the configured destinations"
		);
		anyhow::ensure!(
			state.commit_publishing.committed < state.next_volume_seq,
			"commit exceeds assigned history"
		);
		if matches!(
			state.phase,
			VolumeClientPhase::Appending
				| VolumeClientPhase::Recovering(state::RecoveryPhase::CommittingClaimEntry { .. })
		) {
			anyhow::ensure!(
				state
					.above_cold
					.end_seq()
					.is_none_or(|end| end == state.next_volume_seq),
				"the sizes above cold do not end at the next position"
			);
		}
		anyhow::ensure!(
			state.writes.len() <= state.config.max_pending_writes as usize,
			"pending write budget exceeded"
		);
		anyhow::ensure!(
			state.memory_bytes + waiting::reserved_bytes(state) <= state.config.max_memory_bytes,
			"not yet cold byte budget exceeded"
		);
		for r in state.replicas.values() {
			anyhow::ensure!(
				r.in_flight.len() <= state.config.max_in_flight as usize,
				"replica window exceeded"
			);
			anyhow::ensure!(
				r.refused.is_none() || r.in_flight.is_empty(),
				"refused replica has sends outstanding"
			);
			anyhow::ensure!(
				r.blocked
					.is_none_or(|blocked| r.in_flight.values().all(|p| p.end < blocked.volume_seq)),
				"blocked replica has a refused entry in flight"
			);
			anyhow::ensure!(
				r.append_limit_bytes <= state.config.max_append_bytes,
				"replica append limit above the configured maximum"
			);
		}
		Ok(())
	}
}








