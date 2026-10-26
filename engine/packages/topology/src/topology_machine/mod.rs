//! One state machine owns local leases, storage discovery and endpoint-cache mutations.

pub mod effect;
pub mod event;
pub(crate) mod executor;
mod lease;
mod lookup;
mod publication;
pub mod snapshot;
pub mod state;
pub mod types;
pub mod watch;
use rivet_state_machine::Entropy;
use types::*;

use crate::topology_machine::snapshot::TopologySnapshot;
use crate::topology_machine::types::FenceReason;
pub use effect::{Effect, Effects};
pub use event::Event;
use rivet_env::Clocks;
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};
use state::*;
pub use types::Timer;

/// Drives topology decisions; executors only perform effects and return responses.
///
/// ```text
/// register -> read/claim heartbeat -> renew -> expire or drain
///                   |                  |
///             discover storage -> publish snapshot
///                                      |
///       resolve: snapshot hit / miss -> read exact worker -> publish cache
/// ```
///
/// ## Lease expiration and fencing
/// Serving ends at the confirmed write's send time plus TTL minus the margin.
/// Late responses cannot revive a fenced worker. Consumers check the latest
/// snapshot and clock at admission and reply; notifications also drive cleanup.
/// Observer mode discovers routes without registering or granting a serving lease.
///
/// ## Clean shutdown
/// Drain keeps renewing while dependents finish cleanup. Only then may the caller
/// send `StopRequest`: renewal ends permanently and a conditional Released heartbeat lets
/// a replacement claim immediately. Failed or quarantined cleanup must fence instead;
/// an Active or Draining heartbeat still requires the full expiration wait.
///
/// ## Slots
/// Heartbeat and discovery operations use IDs to reject stale responses; pending
/// work is not superseded. Endpoint reads have per-worker generations; invalidation never affects
/// another worker. All endpoint work is bounded independently of lease I/O.
pub struct TopologyStateMachine;

impl rivet_state_machine::StateMachineState for State {}
impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Event::ResolveEndpointRequest { .. } => "resolve_endpoint_request",
			Event::InvalidateEndpointRequest { .. } => "invalidate_endpoint_request",
			Event::EndpointTimedOut { .. } => "endpoint_timed_out",
			Event::InspectHeartbeatRequest { .. } => "inspect_heartbeat_request",
			Event::WatchRequest { .. } => "watch_request",
			Event::UnwatchRequest { .. } => "unwatch_request",
			Event::ReadWatchedHeartbeatResponse { .. } => "read_watched_heartbeat_response",
			Event::ReadLocalHeartbeatResponse { .. } => "read_local_heartbeat_response",
			Event::WriteLocalHeartbeatResponse { .. } => "write_local_heartbeat_response",
			Event::ListStorageWorkersResponse { .. } => "list_storage_workers_response",
			Event::ListStorageWorkersPartialResponse { .. } => {
				"list_storage_workers_partial_response"
			}
			Event::LookupEndpointResponse { .. } => "lookup_endpoint_response",
			Event::ReadStorageHeartbeatResponse { .. } => "read_storage_heartbeat_response",
			Event::Refresh => "refresh",
			Event::BeginDrain => "begin_drain",
			Event::Fence(..) => "fence",
			Event::StopRequest => "stop_request",
		}
	}
}
impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::ResolveEndpointResponse { .. } => "resolve_endpoint_response",
			Effect::InvalidateEndpointResponse { .. } => "invalidate_endpoint_response",
			Effect::InspectHeartbeatResponse { .. } => "inspect_heartbeat_response",
			Effect::ReadLocalHeartbeatRequest { .. } => "read_local_heartbeat_request",
			Effect::WriteLocalHeartbeatRequest { .. } => "write_local_heartbeat_request",
			Effect::ListStorageWorkersRequest { .. } => "list_storage_workers_request",
			Effect::LookupEndpointRequest { .. } => "lookup_endpoint_request",
			Effect::ReadStorageHeartbeatRequest { .. } => "read_storage_heartbeat_request",
			Effect::ReadWatchedHeartbeatRequest { .. } => "read_watched_heartbeat_request",
			Effect::WatchedProcessGone { .. } => "watched_process_gone",
			Effect::CancelLookups => "cancel_lookups",
			Effect::RecordLookupUsage { .. } => "record_lookup_usage",
			Effect::RecordLookupRejected => "record_lookup_rejected",
			Effect::RecordEndpointTimeout { .. } => "record_endpoint_timeout",
			Effect::RecordIncompatibleVersion { .. } => "record_incompatible_version",
			Effect::RecordFence { .. } => "record_fence",
			Effect::RecordRenewal { .. } => "record_renewal",
			Effect::RecordRenewalLate { .. } => "record_renewal_late",
		}
	}
}
impl rivet_state_machine::Snapshot for TopologySnapshot {}

impl EphemeralStateMachine for TopologyStateMachine {
	type Slot = Slot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Snapshot = TopologySnapshot;
	type Timer = Timer;
	const DEBUG_NAME: &'static str = "topology";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			Timer::Registration => "registration",
			Timer::Renew => "renew",
			Timer::Refresh => "refresh",
			Timer::WorkerLease => "worker_lease",
			Timer::RenewalLate => "renewal_late",
			Timer::ReleaseDeadline => "release_deadline",
			Timer::Retry => "retry",
			Timer::Takeover => "takeover",
			Timer::LookupExpiry => "lookup_expiry",
			Timer::WatchPoll => "watch_poll",
			Timer::RenewalAttempt => "renewal_attempt",
			Timer::Io(..) => "io",
			Timer::Publication(..) => "publication",
		}
	}

	fn start(state: &mut State, now: Clocks, _: Entropy, out: &mut Effects) {
		if let LeaseState::Registering {
			registration_deadline_boot_ms,
			..
		} = &mut state.lease
		{
			*registration_deadline_boot_ms = now
				.boot_ms
				.saturating_add(state.config.lease.boot_timeout_ms);
			let deadline_boot_ms = *registration_deadline_boot_ms;

			out.arm_timer_at(Timer::Registration, deadline_boot_ms);
			lease::read_local_heartbeat(state, out);
		} else if matches!(state.lease, LeaseState::Observer) {
			lease::list_storage_workers(state, "start", now.boot_ms, out);
		}
	}

	fn on_event(state: &mut State, event: EphemeralStateMachineEvent<Self>, out: &mut Effects) {
		let now_boot_ms = event.now.boot_ms;

		// Sharing the cache root makes any mutation during this event observable through
		// `ptr_eq`, since a shared root is replaced rather than changed in place.
		let entries_before = state.lookups.entries.clone();
		let reads_before = (
			state.lookups.pending.len(),
			state.lookups.heartbeats.len(),
			state.lookups.next_generation,
		);
		let stopped_before =
			state.lease.terminal() || matches!(state.lease, LeaseState::Releasing { .. });
		let next_poll_before = (state.watches.polls.next_at(), state.watches.reading);
		let poll_timer = matches!(event.body, RuntimeEvent::Timer(Timer::WatchPoll));

		// Due alarms are ordered by deadline, so each alarm checks lease authority before work.
		if matches!(event.body, RuntimeEvent::Timer(_)) {
			if matches!(state.lease, LeaseState::Live { .. })
				&& state.worker_lease.on_timer(out) == rivet_state_machine_util::LeaseChange::Fenced
			{
				lease::fence(state, FenceReason::Expired, out);
			}
			if matches!(state.lease, LeaseState::Registering { registration_deadline_boot_ms, .. } if now_boot_ms >= registration_deadline_boot_ms)
			{
				lease::fence(state, FenceReason::RegistrationFailed, out);
			}
			if matches!(state.lease, LeaseState::Releasing { deadline_boot_ms, .. } if now_boot_ms >= deadline_boot_ms)
			{
				lease::fence(state, FenceReason::Expired, out);
			}
		}

		if state.lease.terminal() || matches!(state.lease, LeaseState::Releasing { .. }) {
			match event.body {
				RuntimeEvent::Input(Event::StopRequest) => {}
				RuntimeEvent::Input(Event::Fence(reason)) => lease::fence(state, reason, out),
				RuntimeEvent::Input(
					event @ (Event::ReadLocalHeartbeatResponse { .. }
					| Event::WriteLocalHeartbeatResponse { .. }),
				) => publication::response(state, event, now_boot_ms, out),
				RuntimeEvent::Timer(timer) => publication::timer(state, timer, now_boot_ms, out),
				RuntimeEvent::Input(event) => lookup::reject(event, out),
			}
		} else {
			match event.body {
				RuntimeEvent::Input(Event::InspectHeartbeatRequest { request_id, worker }) => {
					lookup::handle_inspect_heartbeat_request(
						state,
						request_id,
						worker,
						now_boot_ms,
						out,
					)
				}
				RuntimeEvent::Input(Event::ReadStorageHeartbeatResponse {
					worker,
					generation,
					result,
				}) => lookup::handle_read_storage_heartbeat_response(
					state,
					worker,
					generation,
					result,
					now_boot_ms,
					out,
				),
				RuntimeEvent::Input(Event::ResolveEndpointRequest { request_id, worker }) => {
					lookup::handle_resolve_endpoint_request(
						state,
						request_id,
						worker,
						now_boot_ms,
						out,
					)
				}
				RuntimeEvent::Input(Event::LookupEndpointResponse {
					worker,
					generation,
					result,
					heartbeat_modified_unix_ms,
				}) => lookup::handle_lookup_endpoint_response(
					state,
					worker,
					generation,
					result,
					heartbeat_modified_unix_ms,
					now_boot_ms,
					out,
				),
				RuntimeEvent::Input(Event::InvalidateEndpointRequest { request_id, worker }) => {
					lookup::handle_invalidate_endpoint_request(
						state,
						request_id,
						worker,
						now_boot_ms,
						out,
					)
				}
				RuntimeEvent::Input(Event::EndpointTimedOut { worker }) => {
					lookup::handle_endpoint_timed_out(state, worker, now_boot_ms, out)
				}
				RuntimeEvent::Input(Event::WatchRequest {
					process,
					subscription,
				}) => watch::handle_watch_request(state, process, subscription, now_boot_ms),
				RuntimeEvent::Input(Event::UnwatchRequest {
					process,
					subscription,
				}) => watch::handle_unwatch_request(state, process, subscription),
				RuntimeEvent::Input(Event::ReadWatchedHeartbeatResponse {
					process,
					generation,
					result,
				}) => watch::handle_read_watched_heartbeat_response(
					state,
					process,
					generation,
					result,
					now_boot_ms,
					out,
				),
				RuntimeEvent::Input(Event::StopRequest) => {
					lease::begin_release(state, now_boot_ms, out);
				}
				RuntimeEvent::Input(Event::Fence(reason)) => lease::fence(state, reason, out),
				RuntimeEvent::Input(Event::BeginDrain) => {
					match &mut state.lease {
						LeaseState::Registering { participation, .. }
						| LeaseState::Live { participation, .. } => *participation = Participation::Draining,
						LeaseState::Observer
						| LeaseState::Releasing { .. }
						| LeaseState::Fenced { .. }
						| LeaseState::Stopped { .. } => {}
					}

					if matches!(state.lease, LeaseState::Live { .. })
						&& state.pending(Slot::Heartbeat).is_none()
					{
						lease::write_renewal(state, now_boot_ms, out);
					}
				}
				RuntimeEvent::Input(Event::Refresh) if state.discovering() => {
					lease::hint_refresh(state, "request", now_boot_ms, out);
				}
				RuntimeEvent::Input(input) => {
					lease::handle_response(state, input, now_boot_ms, out)
				}
				RuntimeEvent::Timer(Timer::LookupExpiry) => lookup::expire(state, now_boot_ms, out),
				RuntimeEvent::Timer(Timer::WatchPoll) => watch::polls_due(state, now_boot_ms, out),
				RuntimeEvent::Timer(timer) => lease::timer(state, timer, now_boot_ms, out),
			}
		}
		state.refresh_discovery(now_boot_ms);
		if poll_timer || (state.watches.polls.next_at(), state.watches.reading) != next_poll_before
		{
			watch::arm(state, out);
		}
		let reads_after = (
			state.lookups.pending.len(),
			state.lookups.heartbeats.len(),
			state.lookups.next_generation,
		);
		let stopped = state.lease.terminal() || matches!(state.lease, LeaseState::Releasing { .. });
		if !entries_before.ptr_eq(&state.lookups.entries)
			|| reads_before != reads_after
			|| (!stopped_before && stopped)
		{
			lookup::finish_event(state, out);
		}
	}

	fn snapshot(state: &State) -> TopologySnapshot {
		TopologySnapshot {
			calls: state.pending_calls(),
			lookups: state
				.lookups
				.pending
				.iter()
				.map(|(id, call)| {
					(
						id.clone(),
						rivet_state_machine_util::IoCall {
							op: call.generation,
							deadline_boot_ms: call.deadline_boot_ms,
						},
					)
				})
				.collect(),
			inspections: state
				.lookups
				.heartbeats
				.iter()
				.map(|(id, call)| {
					(
						id.clone(),
						rivet_state_machine_util::IoCall {
							op: call.generation,
							deadline_boot_ms: call.deadline_boot_ms,
						},
					)
				})
				.collect(),
			worker_registration: state.lease.worker_registration().cloned(),
			status: state.lease.status(&state.config.lease),
			heartbeat_state: state.lease.heartbeat_state(),
			storage_workers: state
				.current_storage_entries()
				.map(crate::heartbeat::Observation::worker)
				.collect(),
			valid_until_boot_ms: state.discovery.valid_until_boot_ms,
			store_clock: state.discovery.store_clock.clone(),
			read_floor: state.discovery.read_floor.clone(),
			discovery_round: state.discovery.round,
			discovery_ok: state.discovery.ok,
			discovery_partial: state.discovery.partial,
			endpoints: state.lookups.entries.clone(),
			invalidated: state.lookups.invalidated.clone(),
			watched_processes: state.watches.watches.len(),
		}
	}

	fn debug_validate(state: &State) -> anyhow::Result<()> {
		state.config.validate()?;

		if let LeaseState::Live {
			confirmed_write,
			unconfirmed_writes,
			..
		} = &state.lease
		{
			anyhow::ensure!(
				unconfirmed_writes.len() + 1
					<= (state.config.lease.ttl_ms / state.config.lease.renew_interval_ms
						+ state.config.lease.ttl_ms / state.config.lease.io_timeout_ms
						+ 4) as usize,
				"unbounded heartbeat history"
			);
			anyhow::ensure!(
				unconfirmed_writes
					.windows(2)
					.all(|w| w[0].write_sequence < w[1].write_sequence),
				"unordered heartbeat write history"
			);
			anyhow::ensure!(
				unconfirmed_writes
					.first()
					.is_none_or(|write| confirmed_write.write.write_sequence < write.write_sequence),
				"unconfirmed heartbeat must follow the confirmed write"
			);
		}

		if state.lease.terminal() {
			anyhow::ensure!(
				state.lookups.pending.is_empty() && state.lookups.heartbeats.is_empty(),
				"terminal topology retains work"
			);
		}
		anyhow::ensure!(
			state.lookups.entries.len() <= state.config.endpoint_capacity,
			"endpoint cache exceeds cap"
		);
		lookup::validate(state)?;
		watch::validate(state)?;
		if state.lease.terminal() {
			anyhow::ensure!(
				state.watches.watches.is_empty(),
				"terminal topology retains watches"
			);
		}
		anyhow::ensure!(
			state.lookups.pending.len() + state.lookups.heartbeats.len()
				<= state.config.lookup_concurrency,
			"too many endpoint reads"
		);

		Ok(())
	}
}






