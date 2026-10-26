//! Manual tablet placement and grants, acknowledged only after conditional manifest writes.

pub mod effect;
pub mod event;
pub mod placement;
#[cfg(not(target_arch = "wasm32"))]
pub mod runner;
pub mod state;
pub mod store;
pub mod types;

use effect::Effect;
use event::Event;
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::{
	TabletCoordinatorRejectResponse, TabletCoordinatorRequest,
};
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEvent,
};
use serde::{Deserialize, Serialize};
use state::{Phase, ReadContext, State};
use types::{Command, Timer};

pub type Effects = rivet_state_machine_ephemeral::Effects<Effect, Timer>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Snapshot {
	pub placement: Option<state::balancing::Status>,
	pub ready: bool,
	pub stopped: bool,
	pub manifest: Option<rivet_data::generated::depot_tablet_manifest_v1::TabletManifest>,
}

impl rivet_state_machine::Snapshot for Snapshot {}

pub struct TabletCoordinatorMachine;

impl EphemeralStateMachine for TabletCoordinatorMachine {
	type Slot = NoSlot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Snapshot = Snapshot;
	type Timer = Timer;
	const DEBUG_NAME: &'static str = "tablet_coordinator_machine";

	fn timer_name(timer: &Self::Timer) -> &'static str {
		match timer {
			Timer::Operation(..) => "operation",
			Timer::Retry => "retry",
			Timer::Balance => "balance",
			Timer::LearningDeadline => "learning_deadline",
			Timer::DiskProbe(..) => "disk_probe",
			Timer::HandoffRetry(..) => "handoff_retry",
		}
	}

	fn start(state: &mut State, _: Clocks, _: Entropy, effects: &mut Effects) {
		state.read(ReadContext::Takeover, effects);
	}

	fn on_event(
		state: &mut State,
		mut event: EphemeralStateMachineEvent<Self>,
		effects: &mut Effects,
	) {
		if let RuntimeEvent::Input(Event::AlarmCommand {
			request_id,
			request,
		}) = event.body
		{
			let (request, special) = state::alarms::normalize(request);
			if state.queued.len() < state.max_requests && !matches!(state.phase, Phase::Stopped) {
				if let Some(special) = special {
					state.alarms.commands.insert(request_id, special);
				}
			}
			event.body = RuntimeEvent::Input(Event::Command {
				request_id,
				request,
			});
		}
		match event.body {
			RuntimeEvent::Input(Event::DiskProbeResponse {
				worker,
				operation,
				result,
			}) => {
				let fresh = state
					.discovery
					.as_ref()
					.is_some_and(|topology| event.now.boot_ms < topology.valid_until_boot_ms);
				if let Some(balance) = state.balancing.as_mut() {
					balance.probe_response(
						worker,
						operation,
						if fresh {
							result
						} else {
							Err("topology expired".into())
						},
						effects,
					);
				}
			}
			RuntimeEvent::Input(Event::RequestStop {
				request_id,
				request,
				topology,
			}) => {
				if matches!(state.phase, Phase::Stopped) {
					state::reject(
						request_id,
						TabletCoordinatorRejectResponse::NotCoordinator,
						effects,
					);
				} else if state.queued.len() >= state.max_requests {
					state::reject(
						request_id,
						TabletCoordinatorRejectResponse::Overloaded,
						effects,
					);
				} else {
					state.queued.push_back(Command {
						request_id,
						request: TabletCoordinatorRequest::RequestStop(request),
						topology: Some(topology),
						tablet_id: [0; 16],
						split: None,
						requested_at: event.now.unix_ms,
					});
				}
			}
			RuntimeEvent::Input(Event::HandoffReadinessResponse {
				operation,
				worker,
				results,
			}) => state.handoff_readiness(operation, worker, results, effects),
			RuntimeEvent::Input(Event::CancelCommand { request_id }) => {
				state.cancel_command(request_id, effects)
			}
			RuntimeEvent::Input(Event::StopReadinessResponse {
				operation,
				worker,
				results,
			}) => state.stop_readiness(operation, worker, results),
			RuntimeEvent::Input(Event::Command {
				request_id,
				request,
			}) => {
				if matches!(state.phase, Phase::Stopped) {
					state::reject(
						request_id,
						TabletCoordinatorRejectResponse::NotCoordinator,
						effects,
					);
				} else if state.queued.len() >= state.max_requests {
					state::reject(
						request_id,
						TabletCoordinatorRejectResponse::Overloaded,
						effects,
					);
				} else {
					let mut rng = event.entropy.clone().rng();
					let tablet_id = *rng.uuid_v4().as_bytes();
					let split = if let rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorRequest::SplitTablet(split) = &request {
						Some(rivet_data::generated::depot_tablet_manifest_v1::TabletSplit {
							split_key_hash: split.split_key_hash,
							left_tablet_id: *rng.uuid_v4().as_bytes(),
							right_tablet_id: *rng.uuid_v4().as_bytes(),
							checkpoint_id: *rng.uuid_v4().as_bytes(),
						})
					} else { None };
					state.queued.push_back(Command {
						topology: None,
						request_id,
						request,
						tablet_id,
						split,
						requested_at: event.now.unix_ms,
					});
				}
			}
			RuntimeEvent::Input(Event::AlarmCommand { .. }) => {
				unreachable!("normalized coordinator command")
			}
			RuntimeEvent::Input(Event::ReadManifestResponse { operation, result }) => {
				state.read_response(operation, result, effects)
			}
			RuntimeEvent::Input(Event::WriteManifestResponse { operation, result }) => {
				state.write_response(operation, result, effects)
			}
			RuntimeEvent::Input(Event::StorageWorkers { topology }) => {
				if !matches!(state.phase, Phase::Stopped) {
					if let Some(balance) = state.balancing.as_mut() {
						balance.observed(&topology, effects);
					}
					state.discovery = Some(topology);
				}
			}
			RuntimeEvent::Input(Event::BalancingEnabled { enabled }) => {
				if !matches!(state.phase, Phase::Stopped)
					&& let Some(balance) = state.balancing.as_mut()
					&& balance.set_enabled(enabled, effects)
				{
					tracing::info!(
						enabled,
						"automatic tablet balancing switched by dynamic config"
					);
				}
			}
			RuntimeEvent::Input(Event::Stop) => state.stop(effects),
			RuntimeEvent::Timer(Timer::Operation(operation)) => state.timeout(operation, effects),
			RuntimeEvent::Timer(Timer::Retry) => {
				if let Phase::Waiting { context } = &state.phase {
					state.read(context.clone(), effects);
				}
			}
			RuntimeEvent::Timer(Timer::HandoffRetry(generation)) => {
				if let Phase::WaitingHandoff { pending } = &state.phase {
					if pending.generation == generation {
						state.check_handoff(pending.clone(), effects);
					}
				}
			}

			RuntimeEvent::Timer(Timer::Balance) => {
				if let Some(balance) = state.balancing.as_mut() {
					balance.timer();
				}
			}
			// The drive below aborts every Learning phase whose deadline passed.
			RuntimeEvent::Timer(Timer::LearningDeadline) => {}
			RuntimeEvent::Timer(Timer::DiskProbe(worker, operation)) => {
				if let Some(balance) = state.balancing.as_mut() {
					balance.probe_response(
						worker,
						operation,
						Err("disk probe timed out".into()),
						effects,
					);
				}
			}
		}
		state.drive(effects, &event.entropy);
	}

	fn debug_validate(state: &State) -> anyhow::Result<()> {
		anyhow::ensure!(
			state.queued.len() <= state.max_requests,
			"coordinator queue limit exceeded"
		);
		if let Phase::Ready(stored) = &state.phase {
			state::validate(&stored.manifest)?;
		}
		Ok(())
	}

	fn snapshot(state: &State) -> Snapshot {
		Snapshot {
			placement: match &state.phase {
				Phase::Ready(stored) => state
					.balancing
					.as_ref()
					.and_then(|b| b.status(&stored.manifest)),
				_ => None,
			},
			ready: matches!(state.phase, Phase::Ready(_)),
			stopped: matches!(state.phase, Phase::Stopped),
			manifest: match &state.phase {
				Phase::Ready(stored) => Some(stored.manifest.clone()),
				Phase::Reading { .. }
				| Phase::Waiting { .. }
				| Phase::CheckingHandoff { .. }
				| Phase::WaitingHandoff { .. }
				| Phase::Writing { .. }
				| Phase::Stopped => None,
			},
		}
	}
}
