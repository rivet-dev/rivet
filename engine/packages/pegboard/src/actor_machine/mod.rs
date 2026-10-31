use crate::wire;
use anyhow::Result;
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_durable::{DurableStateMachine, Effects, RuntimeEvent};
use serde::{Deserialize, Serialize};
use vbare::Versioned;

pub mod executor;

const EFFECT_TIMEOUT_MS: u64 = 15 * 1000;
const EFFECT_MAX_ATTEMPTS: u32 = 10;
/// How long a stop command goes unanswered before it is sent again. A stop never times out
/// into the next epoch: it is resent until the envoy reports the epoch stopped or the
/// executor proves the envoy gone, which it can do about one lost threshold after the
/// envoy's connection closed. The interval bounds how late after that point the proof comes.
const STOP_RESEND_MS: u64 = 5 * 1000;
/// The most a resend is delayed past `STOP_RESEND_MS`, drawn per resend, so the actors of one
/// envoy that were stopped together do not resend and read its record in lockstep.
const STOP_RESEND_JITTER_MS: u64 = 2 * 1000;
/// How long a due alarm that found no envoy to start its actor on waits before it tries again.
/// Envoys reconnect about a second after the engine starts, so a short wait keeps the alarm
/// close to its deadline.
const ALARM_NO_ENVOY_RETRY_MS: u64 = 1000;

pub struct ActorStateMachine;

impl DurableStateMachine for ActorStateMachine {
	type Slot = NoSlot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Timer = wire::ActorTimer;
	type Snapshot = Snapshot;
	type Preparation = Option<wire::ActorAllocation>;
	type WriteEffect = WriteEffect;

	const DEBUG_NAME: &'static str = "actor";

	fn start(
		_state: &mut Self::State,
		_clocks: Clocks,
		_entropy: Entropy,
		_effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	) {
	}

	fn on_event(
		state: &mut Self::State,
		raw_event: rivet_state_machine::Event<
			rivet_state_machine_durable::RuntimeEvent<Event, wire::ActorTimer>,
		>,
		preparation: Option<wire::ActorAllocation>,
		effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	) {
		let State(state) = state;
		let now_unix_ms = raw_event.now.unix_ms;
		let entropy = raw_event.entropy.clone();
		let event = match raw_event.body {
			RuntimeEvent::Input(Event(input)) => RuntimeEvent::Input(input),
			RuntimeEvent::Timer(timer) => RuntimeEvent::Timer(timer),
		};

		// Process event
		match state.phase {
			wire::ActorPhase::Initializing(ref mut init) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(state, effects, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::ReserveEpoxyState(effect_state)) => {
					walk_reserve_epoxy(
						&mut init.reserve_epoxy,
						state.create_unix_ms,
						effects,
						Some(effect_state),
					);
					try_complete_initialization(state, effects, Dispatch::Nondurable, now_unix_ms);
				}
				RuntimeEvent::Timer(wire::ActorTimer::ReserveEpoxyTimeout(timeout)) => {
					walk_reserve_epoxy(
						&mut init.reserve_epoxy,
						state.create_unix_ms,
						effects,
						Some(wire::ReserveEpoxyState {
							phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
					try_complete_initialization(state, effects, Dispatch::Nondurable, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingSleep,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						set_pending_action(
							&mut state.pending_action,
							wire::PendingAction::PendingReallocate,
						);
						store_start_waiter(state, event.waiter, now_unix_ms);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::WriteIndexState(effect_state)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(effect_state),
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::WriteIndexTimeout(timeout)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(wire::WriteIndexState {
							phase: wire::WriteIndexPhase::WriteIndexPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Destroy) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingDestroy,
					);
				}
				_ => {}
			},
			// Start command sent to `allocation`, waiting for the envoy's running report.
			wire::ActorPhase::Starting(ref mut phase) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(state, effects, preparation, now_unix_ms);
				}
				// The start command did not reach the envoy, or the envoy never reported
				// running. Either way the envoy may hold the epoch, so it is only stopped, like
				// a lost report: the epoch ends on the stopped report or once the envoy is
				// proven gone, so the next start never runs beside this one.
				RuntimeEvent::Input(wire::ActorEvent::StartFailed(res)) => {
					if res.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						effects.cancel_timer(wire::ActorTimer::StartTimeout(wire::StartTimeout {
							epoch: state.epoch,
						}));
						stop_actor(state, effects, allocation);
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::StartTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						stop_actor(state, effects, allocation);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::ToEnvoyEvent(event)) => {
					if event.epoch == state.epoch {
						match event.kind {
							wire::ToEnvoyEventKind::ToEnvoyEventKindRunning => {
								let allocation = phase.allocation.clone();
								effects.cancel_timer(wire::ActorTimer::StartTimeout(
									wire::StartTimeout { epoch: state.epoch },
								));

								if !state.start_waiters.is_empty() {
									// Prune start waiters
									state
										.start_waiters
										.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);

									effects.execute(Effect::NotifyStartWaiters {
										waiters: std::mem::take(&mut state.start_waiters),
										kind: wire::ActorStartNoticeKind::ActorStartNoticeRunning(
											wire::ActorStartNoticeRunning {
												epoch: state.epoch,
												allocation: allocation.clone(),
											},
										),
									});
								}

								state.phase = wire::ActorPhase::Running(wire::Running {
									allocation: allocation.clone(),
								});

								match &state.pending_action {
									wire::PendingAction::PendingNone => {}
									wire::PendingAction::PendingSleep
									| wire::PendingAction::PendingReallocate
									| wire::PendingAction::PendingDestroy => {
										stop_actor(state, effects, allocation);
									}
								}
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_) => {
								handle_stopped(state, effects, preparation, now_unix_ms);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
								set_pending_action(
									&mut state.pending_action,
									wire::PendingAction::PendingSleep,
								);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
								set_pending_action(
									&mut state.pending_action,
									wire::PendingAction::PendingDestroy,
								);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(alarm) => {
								set_alarm(effects, alarm.alarm_unix_ms, now_unix_ms);
							}
						}
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingSleep,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::UpdateRoute(event)) => {
					if event.epoch == state.epoch && phase.allocation == event.from {
						phase.allocation = event.to;
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						set_pending_action(
							&mut state.pending_action,
							wire::PendingAction::PendingReallocate,
						);

						// A lost report may be wrong while the envoy still hosts the epoch, so
						// the epoch is only stopped. It ends on the envoy's stopped report or
						// once the envoy is proven gone, so the next start never runs beside
						// this one.
						if event.lost {
							let allocation = phase.allocation.clone();
							effects.cancel_timer(wire::ActorTimer::StartTimeout(
								wire::StartTimeout { epoch: state.epoch },
							));
							stop_actor(state, effects, allocation);
						}
						store_start_waiter(state, event.waiter, now_unix_ms);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::WriteIndexState(effect_state)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(effect_state),
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::WriteIndexTimeout(timeout)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(wire::WriteIndexState {
							phase: wire::WriteIndexPhase::WriteIndexPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Destroy) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingDestroy,
					);
				}
				_ => {}
			},
			wire::ActorPhase::Running(ref mut phase) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(state, effects, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::ToEnvoyEvent(event)) => {
					if event.epoch == state.epoch {
						match event.kind {
							wire::ToEnvoyEventKind::ToEnvoyEventKindRunning => {}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_) => {
								handle_stopped(state, effects, preparation, now_unix_ms);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
								let allocation = phase.allocation.clone();
								stop_actor(state, effects, allocation);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
								set_pending_action(
									&mut state.pending_action,
									wire::PendingAction::PendingDestroy,
								);

								let allocation = phase.allocation.clone();
								stop_actor(state, effects, allocation);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(alarm) => {
								set_alarm(effects, alarm.alarm_unix_ms, now_unix_ms);
							}
						}
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::UpdateRoute(event)) => {
					if event.epoch == state.epoch && phase.allocation == event.from {
						phase.allocation = event.to;
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						set_pending_action(
							&mut state.pending_action,
							wire::PendingAction::PendingReallocate,
						);

						// The epoch is stopped whether its envoy is lost or draining, and ends
						// on the stopped report or once the envoy is proven gone.
						let allocation = phase.allocation.clone();
						stop_actor(state, effects, allocation);
						store_start_waiter(state, event.waiter, now_unix_ms);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {
					let allocation = phase.allocation.clone();
					stop_actor(state, effects, allocation);
				}
				RuntimeEvent::Input(wire::ActorEvent::WriteIndexState(effect_state)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(effect_state),
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::WriteIndexTimeout(timeout)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(wire::WriteIndexState {
							phase: wire::WriteIndexPhase::WriteIndexPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Destroy) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingDestroy,
					);

					let allocation = phase.allocation.clone();
					stop_actor(state, effects, allocation);
				}
				_ => {}
			},
			wire::ActorPhase::Stopping(ref mut phase) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(state, effects, preparation, now_unix_ms);
				}
				// The stop is still unanswered. It may have been lost with a connection the
				// envoy has since replaced while it keeps the epoch running, so silence proves
				// nothing: the same stop is sent again and the timer re-armed.
				RuntimeEvent::Timer(wire::ActorTimer::StopTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						let jitter_ms = entropy.rng().gen_range(0..STOP_RESEND_JITTER_MS);
						resend_stop(state, effects, allocation, jitter_ms);
					}
				}
				// The executor found the envoy disconnected for longer than its lost
				// threshold, past which an envoy stops every actor it holds, so the epoch can
				// end without its stopped report. A late report of an earlier epoch is ignored.
				RuntimeEvent::Input(wire::ActorEvent::EnvoyGone(gone)) => {
					if gone.epoch == state.epoch {
						handle_stopped(state, effects, preparation, now_unix_ms);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::ToEnvoyEvent(event)) => {
					if event.epoch == state.epoch {
						match event.kind {
							wire::ToEnvoyEventKind::ToEnvoyEventKindRunning => {}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_) => {
								handle_stopped(state, effects, preparation, now_unix_ms);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
								set_pending_action(
									&mut state.pending_action,
									wire::PendingAction::PendingSleep,
								);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
								set_pending_action(
									&mut state.pending_action,
									wire::PendingAction::PendingDestroy,
								);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(alarm) => {
								set_alarm(effects, alarm.alarm_unix_ms, now_unix_ms);
							}
						}
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::UpdateRoute(event)) => {
					if event.epoch == state.epoch && phase.allocation == event.from {
						phase.allocation = event.to;
					}
				}
				// A lost report, even a repeated one, does not end the epoch: it may be wrong
				// while the envoy still hosts it. The stopped report or the envoy's proven
				// absence ends it.
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						set_pending_action(
							&mut state.pending_action,
							wire::PendingAction::PendingReallocate,
						);
						store_start_waiter(state, event.waiter, now_unix_ms);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingSleep,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::WriteIndexState(effect_state)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(effect_state),
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::WriteIndexTimeout(timeout)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(wire::WriteIndexState {
							phase: wire::WriteIndexPhase::WriteIndexPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Destroy) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingDestroy,
					);
				}
				_ => {}
			},
			wire::ActorPhase::Sleeping => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(state, effects, event.waiter, preparation, now_unix_ms);
				}
				// With no envoy to start on, as right after the engine starts and before its
				// envoys reconnect, the alarm is armed again so the wake is not lost.
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => match preparation {
					Some(_) => wake(state, effects, preparation, now_unix_ms),
					None => rearm_alarm(effects, entropy),
				},
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						allocate_actor(state, effects, preparation, Dispatch::Durable, now_unix_ms);
						store_start_waiter(state, event.waiter, now_unix_ms);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {}
				RuntimeEvent::Input(wire::ActorEvent::WriteIndexState(effect_state)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(effect_state),
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::WriteIndexTimeout(timeout)) => {
					walk_write_index(
						&mut state.write_index,
						state.index_bucket_count,
						state.create_unix_ms,
						effects,
						Some(wire::WriteIndexState {
							phase: wire::WriteIndexPhase::WriteIndexPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Destroy) => {
					destroy_actor(state, effects, None, now_unix_ms);
				}
				_ => {}
			},
			// TODO: A pending write index effect can re-create the index entry after destroy
			wire::ActorPhase::Destroying(ref mut phase) => match event {
				// Nothing starts an actor being destroyed: a try start or create stores no
				// waiter and the command reports the destroy, and the alarm was cancelled.
				RuntimeEvent::Input(wire::ActorEvent::TryStart(_))
				| RuntimeEvent::Input(wire::ActorEvent::Create(_))
				| RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {}
				RuntimeEvent::Input(wire::ActorEvent::DestroyState(effect_state)) => {
					walk_destroy(
						&mut phase.destroy,
						state.index_bucket_count,
						effects,
						Some(effect_state),
					);

					if let wire::DestroyPhase::DestroyPhaseSuccess = phase.destroy.state.phase {
						state.phase = wire::ActorPhase::Destroyed;
						state.pending_action = wire::PendingAction::PendingNone;
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::DestroyTimeout(timeout)) => {
					walk_destroy(
						&mut phase.destroy,
						state.index_bucket_count,
						effects,
						Some(wire::DestroyState {
							phase: wire::DestroyPhase::DestroyPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
				}
				_ => {}
			},
			// The resting state of a destroyed actor, and the default state of a new one: a
			// create starts the next incarnation, and nothing else moves it.
			wire::ActorPhase::Destroyed => match event {
				RuntimeEvent::Input(wire::ActorEvent::Create(create)) => {
					initialize(
						state,
						create.init,
						create.waiter,
						preparation,
						effects,
						now_unix_ms,
					);
				}
				// A try start stores no waiter; the command reports the actor destroyed.
				RuntimeEvent::Input(wire::ActorEvent::TryStart(_))
				| RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {}
				_ => {}
			},
		}
	}

	fn snapshot(_state: &Self::State) -> Self::Snapshot {
		Snapshot
	}

	fn debug_validate(_state: &Self::State) -> Result<()> {
		Ok(())
	}

	fn serialize_state(state: &Self::State) -> Result<Vec<u8>> {
		rivet_data::versioned::ActorState::serialize_with_embedded_version(
			&state.0,
			rivet_data::generated::PEGBOARD_ACTOR_VERSION,
		)
	}

	fn deserialize_state(bytes: &[u8]) -> Result<Self::State> {
		rivet_data::versioned::ActorState::deserialize_with_embedded_version(bytes).map(State)
	}

	fn serialize_timer(timer: &Self::Timer) -> Result<Vec<u8>> {
		rivet_data::versioned::ActorTimer::serialize_with_embedded_version(
			timer,
			rivet_data::generated::PEGBOARD_ACTOR_VERSION,
		)
	}

	fn deserialize_timer(bytes: &[u8]) -> Result<Self::Timer> {
		rivet_data::versioned::ActorTimer::deserialize_with_embedded_version(bytes)
	}
}

/// Leaves `Initializing` once the reservation settled: on to the start when it is reserved, to
/// a destroy when it ran out of attempts, and to a destroy that tells the waiters who owns the
/// key when another datacenter holds it. Called from the create and after each reservation
/// result or timeout; it does nothing while the reservation is pending. `dispatch` says where
/// the allocation chosen at create time is committed: in this transition when called from the
/// create, so the start waits for the commit, or in an earlier one when called from a
/// reservation result, so the start can go out before this commit. A failed commit then
/// resends the same allocation, epoch and command index, which the envoy drops as a replay.
fn try_complete_initialization(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	dispatch: Dispatch,
	now_unix_ms: i64,
) {
	let wire::ActorPhase::Initializing(init) = &state.phase else {
		return;
	};
	match &init.reserve_epoxy.state.phase {
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => {
			let allocation = init.allocation.clone();
			allocate_actor(state, effects, allocation, dispatch, now_unix_ms);
		}
		wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending => {}
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(already_reserved) => {
			// Another datacenter holds the key. The waiters learn the owner now; no start will
			// ever notify them.
			let reservation_id = already_reserved.reservation_id.clone();
			state
				.start_waiters
				.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
			if !state.start_waiters.is_empty() {
				effects.execute(Effect::NotifyStartWaiters {
					waiters: std::mem::take(&mut state.start_waiters),
					kind: wire::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(
						wire::ActorStartNoticeEpoxyAlreadyReserved {
							reservation_id: reservation_id.clone(),
						},
					),
				});
			}
			destroy_actor(state, effects, Some(reservation_id), now_unix_ms);
		}
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed
		| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => {
			destroy_actor(state, effects, None, now_unix_ms);
		}
	}
}

/// Starts a new incarnation of the actor from `init`: a brand-new actor, or a destroyed keyed
/// actor created again under its id. The incarnation is identified by `create_unix_ms`, which
/// is strictly greater than the previous incarnation's even if the clock went backwards, so
/// the previous incarnation's late results, timers, index entry and database volume can never
/// be mistaken for this one's. The epoch is never reset. The data record is written in this
/// transition's own transaction, so it is durable with the state. A keyed actor then waits for
/// its reservation; an unkeyed one starts at once, with its start sent after the commit since
/// the allocation is committed only now.
fn initialize(
	state: &mut wire::ActorState,
	init: wire::ActorCreateData,
	waiter: Option<wire::StartWaiter>,
	allocation: Option<wire::ActorAllocation>,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	now_unix_ms: i64,
) {
	let create_unix_ms = now_unix_ms.max(state.create_unix_ms.saturating_add(1));
	state.create_unix_ms = create_unix_ms;
	state.command_index = 0;
	state.pending_action = wire::PendingAction::PendingNone;
	state.start_waiters = waiter.into_iter().collect();

	state.write_index = wire::WriteIndexEffectData {
		namespace: init.namespace.clone(),
		name: init.name.clone(),
		create_unix_ms,
		state: wire::WriteIndexState {
			phase: wire::WriteIndexPhase::WriteIndexPhasePending,
			attempt: 0,
			create_unix_ms,
		},
	};
	let mut init_phase = wire::Initializing {
		reserve_epoxy: wire::ReserveEpoxyEffectData {
			namespace: init.namespace.clone(),
			name: init.name.clone(),
			key: init.key.clone(),
			state: wire::ReserveEpoxyState {
				phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
				attempt: 0,
				create_unix_ms,
			},
		},
		allocation,
	};

	effects.write(WriteEffect::WriteData {
		data: wire::ActorData {
			actor_id: init.actor_id,
			namespace: init.namespace,
			pool_name: init.pool_name,
			name: init.name,
			key: init.key,
			input: init.input,
			create_unix_ms,
		},
	});
	walk_reserve_epoxy(&mut init_phase.reserve_epoxy, create_unix_ms, effects, None);
	walk_write_index(
		&mut state.write_index,
		state.index_bucket_count,
		create_unix_ms,
		effects,
		None,
	);

	state.phase = wire::ActorPhase::Initializing(init_phase);
	try_complete_initialization(state, effects, Dispatch::Durable, now_unix_ms);
}

/// The `walk_*` helpers advance one effect's attempt state. `new_state` is the executor's
/// result or the attempt's timeout; it is ignored when it names another incarnation by
/// `create_unix_ms` or an attempt older than the current one. Without it, the first attempt
/// is started.
fn walk_reserve_epoxy(
	effect_data: &mut wire::ReserveEpoxyEffectData,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	new_state: Option<wire::ReserveEpoxyState>,
) {
	// An actor without a key has nothing to reserve
	let Some(key) = &effect_data.key else {
		effect_data.state.phase = wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved;
		return;
	};

	if let Some(new_state) = new_state {
		if new_state.create_unix_ms != create_unix_ms
			|| new_state.attempt < effect_data.state.attempt
		{
			return;
		}

		match &new_state.phase {
			wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved
			| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(_) => {
				effect_data.state.phase = new_state.phase;
				effects.cancel_timer(wire::ActorTimer::ReserveEpoxyTimeout(
					wire::ReserveEpoxyTimeout {
						attempt: effect_data.state.attempt,
						create_unix_ms,
					},
				));

				return;
			}
			wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending
			| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed => {
				if new_state.attempt >= EFFECT_MAX_ATTEMPTS {
					effect_data.state.phase =
						wire::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached;
					effects.cancel_timer(wire::ActorTimer::ReserveEpoxyTimeout(
						wire::ReserveEpoxyTimeout {
							attempt: effect_data.state.attempt,
							create_unix_ms,
						},
					));

					return;
				}
			}
			wire::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => unreachable!(),
		}
	}

	effects.cancel_timer(wire::ActorTimer::ReserveEpoxyTimeout(
		wire::ReserveEpoxyTimeout {
			attempt: effect_data.state.attempt,
			create_unix_ms,
		},
	));

	effect_data.state.attempt += 1;

	effects.nondurable_execute(Effect::ReserveEpoxy {
		namespace: effect_data.namespace.to_string(),
		name: effect_data.name.to_string(),
		key: key.to_string(),
		attempt: effect_data.state.attempt,
		create_unix_ms,
	});

	let backoff = executor::effect_backoff(effect_data.state.attempt);
	effects.arm_timer_after(
		wire::ActorTimer::ReserveEpoxyTimeout(wire::ReserveEpoxyTimeout {
			attempt: effect_data.state.attempt,
			create_unix_ms,
		}),
		EFFECT_TIMEOUT_MS + backoff,
	);
}

fn walk_write_index(
	effect_data: &mut wire::WriteIndexEffectData,
	index_bucket_count: u64,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	new_state: Option<wire::WriteIndexState>,
) {
	if let Some(new_state) = new_state {
		if new_state.create_unix_ms != create_unix_ms
			|| new_state.attempt < effect_data.state.attempt
		{
			return;
		}

		match &new_state.phase {
			wire::WriteIndexPhase::WriteIndexPhaseSuccess => {
				effect_data.state.phase = new_state.phase;
				effects.cancel_timer(wire::ActorTimer::WriteIndexTimeout(
					wire::WriteIndexTimeout {
						attempt: effect_data.state.attempt,
						create_unix_ms,
					},
				));

				return;
			}
			wire::WriteIndexPhase::WriteIndexPhasePending
			| wire::WriteIndexPhase::WriteIndexPhaseFailed => {}
		}
	}

	effects.cancel_timer(wire::ActorTimer::WriteIndexTimeout(
		wire::WriteIndexTimeout {
			attempt: effect_data.state.attempt,
			create_unix_ms,
		},
	));

	effect_data.state.attempt += 1;

	effects.execute(Effect::WriteIndex {
		namespace: effect_data.namespace.to_string(),
		name: effect_data.name.to_string(),
		index_bucket_count,
		create_unix_ms: effect_data.create_unix_ms,
		attempt: effect_data.state.attempt,
	});

	let backoff = executor::effect_backoff(effect_data.state.attempt);
	effects.arm_timer_after(
		wire::ActorTimer::WriteIndexTimeout(wire::WriteIndexTimeout {
			attempt: effect_data.state.attempt,
			create_unix_ms,
		}),
		EFFECT_TIMEOUT_MS + backoff,
	);
}

fn walk_destroy(
	effect_data: &mut wire::DestroyEffectData,
	index_bucket_count: u64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	new_state: Option<wire::DestroyState>,
) {
	let create_unix_ms = effect_data.create_unix_ms;
	if let Some(new_state) = new_state {
		if new_state.create_unix_ms != create_unix_ms
			|| new_state.attempt < effect_data.state.attempt
		{
			return;
		}

		match &new_state.phase {
			wire::DestroyPhase::DestroyPhaseSuccess => {
				effect_data.state.phase = new_state.phase;
				effects.cancel_timer(wire::ActorTimer::DestroyTimeout(wire::DestroyTimeout {
					attempt: effect_data.state.attempt,
					create_unix_ms,
				}));

				return;
			}
			wire::DestroyPhase::DestroyPhasePending | wire::DestroyPhase::DestroyPhaseFailed => {}
		}
	}

	effects.cancel_timer(wire::ActorTimer::DestroyTimeout(wire::DestroyTimeout {
		attempt: effect_data.state.attempt,
		create_unix_ms,
	}));

	effect_data.state.attempt += 1;

	effects.execute(Effect::Destroy {
		attempt: effect_data.state.attempt,
		index_bucket_count,
		create_unix_ms,
	});

	let backoff = executor::effect_backoff(effect_data.state.attempt);
	effects.arm_timer_after(
		wire::ActorTimer::DestroyTimeout(wire::DestroyTimeout {
			attempt: effect_data.state.attempt,
			create_unix_ms,
		}),
		EFFECT_TIMEOUT_MS + backoff,
	);
}

/// Arms the actor's alarm at `alarm_unix_ms`, replacing any earlier alarm, or cancels it for
/// `None`. The timer runs on boot time, so the unix deadline becomes a delay from the event's
/// clock; a deadline already in the past fires on the next delivery.
fn set_alarm(
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	alarm_unix_ms: Option<i64>,
	now_unix_ms: i64,
) {
	match alarm_unix_ms {
		Some(alarm_unix_ms) => {
			let delay_ms = alarm_unix_ms.saturating_sub(now_unix_ms).max(0) as u64;
			effects.arm_timer_after(wire::ActorTimer::AlarmTimeout, delay_ms);
		}
		None => effects.cancel_timer(wire::ActorTimer::AlarmTimeout),
	}
}

/// Brings the actor back up for a wake request: the alarm firing, or a try start or create.
/// A sleeping actor allocates. An actor stopping for a sleep is reallocated instead, and one on
/// its way up drops a pending sleep so it is not stopped on arrival; a pending destroy is kept
/// either way. A starting actor resends its committed command in case its previous host lost
/// the dispatch. A running actor needs nothing, and a destroyed one is not revived here.
fn wake(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	hint: Option<wire::ActorAllocation>,
	now_unix_ms: i64,
) {
	match &state.phase {
		wire::ActorPhase::Sleeping => {
			allocate_actor(state, effects, hint, Dispatch::Durable, now_unix_ms);
		}
		wire::ActorPhase::Stopping(_) => {
			set_pending_action(
				&mut state.pending_action,
				wire::PendingAction::PendingReallocate,
			);
		}
		wire::ActorPhase::Initializing(_) | wire::ActorPhase::Starting(_) => {
			// Only a sleep is cleared. Clearing through `set_pending_action` would also drop
			// a pending destroy, which must win.
			if let wire::PendingAction::PendingSleep = state.pending_action {
				state.pending_action = wire::PendingAction::PendingNone;
			}
			if let wire::ActorPhase::Starting(phase) = &state.phase {
				// Revocation can cancel the old host after Starting commits but before Start
				// is dispatched. Replay the same committed allocation and command identity;
				// keep the original timer so repeated wakes cannot extend the start attempt.
				effects.nondurable_execute(Effect::Start {
					epoch: state.epoch,
					command_index: state.command_index,
					allocation: phase.allocation.clone(),
					new_storage: state.command_index == 1,
				});
			}
		}
		wire::ActorPhase::Running(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed => {}
	}
}

/// Whether `event` applied to `state` starts the actor, so the executor's `prepare` selects
/// the envoy for it. Exact by construction: it runs on the state the transition will use. A
/// start it names for which no envoy exists sends nothing and sleeps.
pub fn needs_allocation(
	state: &wire::ActorState,
	event: &RuntimeEvent<Event, wire::ActorTimer>,
) -> bool {
	let reallocation_pending = state.pending_action == wire::PendingAction::PendingReallocate;
	let sleeping = matches!(state.phase, wire::ActorPhase::Sleeping);
	match event {
		RuntimeEvent::Input(Event(input)) => match input {
			wire::ActorEvent::TryStart(_) => sleeping,
			wire::ActorEvent::Create(_) => {
				sleeping || matches!(state.phase, wire::ActorPhase::Destroyed)
			}
			wire::ActorEvent::Reallocate(event) if event.epoch == state.epoch => {
				match state.phase {
					wire::ActorPhase::Sleeping => true,
					wire::ActorPhase::Initializing(_)
					| wire::ActorPhase::Starting(_)
					| wire::ActorPhase::Running(_)
					| wire::ActorPhase::Stopping(_)
					| wire::ActorPhase::Destroying(_)
					| wire::ActorPhase::Destroyed => false,
				}
			}
			wire::ActorEvent::ToEnvoyEvent(event) if event.epoch == state.epoch => {
				matches!(
					event.kind,
					wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_)
				) && reallocation_pending
					&& matches!(
						state.phase,
						wire::ActorPhase::Starting(_)
							| wire::ActorPhase::Running(_)
							| wire::ActorPhase::Stopping(_)
					)
			}
			wire::ActorEvent::EnvoyGone(event) if event.epoch == state.epoch => {
				reallocation_pending && matches!(state.phase, wire::ActorPhase::Stopping(_))
			}
			wire::ActorEvent::Reallocate(_)
			| wire::ActorEvent::ToEnvoyEvent(_)
			| wire::ActorEvent::EnvoyGone(_)
			| wire::ActorEvent::StartFailed(_)
			| wire::ActorEvent::ReserveEpoxyState(_)
			| wire::ActorEvent::WriteIndexState(_)
			| wire::ActorEvent::DestroyState(_)
			| wire::ActorEvent::Sleep
			| wire::ActorEvent::UpdateRoute(_)
			| wire::ActorEvent::Destroy => false,
		},
		RuntimeEvent::Timer(timer) => match timer {
			wire::ActorTimer::AlarmTimeout => sleeping,
			wire::ActorTimer::StartTimeout(_)
			| wire::ActorTimer::StopTimeout(_)
			| wire::ActorTimer::ReserveEpoxyTimeout(_)
			| wire::ActorTimer::WriteIndexTimeout(_)
			| wire::ActorTimer::DestroyTimeout(_) => false,
		},
	}
}

fn set_pending_action(
	pending_action: &mut wire::PendingAction,
	new_pending_action: wire::PendingAction,
) {
	let allow = match (&pending_action, &new_pending_action) {
		(wire::PendingAction::PendingNone, _)
		| (_, wire::PendingAction::PendingNone)
		| (wire::PendingAction::PendingSleep, wire::PendingAction::PendingReallocate)
		| (wire::PendingAction::PendingReallocate, wire::PendingAction::PendingSleep)
		| (_, wire::PendingAction::PendingDestroy) => true,
		_ => false,
	};

	if allow {
		*pending_action = new_pending_action;
	}
}

/// Arms the alarm again for a wake that found no envoy to start the actor on. The jitter keeps
/// the actors whose alarms came due together, as when the engine starts, from retrying in
/// lockstep.
fn rearm_alarm(effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>, entropy: Entropy) {
	let jitter_ms = entropy.rng().gen_range(0..STOP_RESEND_JITTER_MS);
	effects.arm_timer_after(
		wire::ActorTimer::AlarmTimeout,
		ALARM_NO_ENVOY_RETRY_MS + jitter_ms,
	);
}

/// Ends the current epoch: its timers are cancelled, the epoch moves on, and the pending action
/// decides whether the actor sleeps, starts again or is destroyed. `hint` is the envoy a
/// command pre-selected for a start this stop leads to.
fn handle_stopped(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	hint: Option<wire::ActorAllocation>,
	now_unix_ms: i64,
) {
	effects.cancel_timer(wire::ActorTimer::StartTimeout(wire::StartTimeout {
		epoch: state.epoch,
	}));
	effects.cancel_timer(wire::ActorTimer::StopTimeout(wire::StopTimeout {
		epoch: state.epoch,
	}));

	// The command index runs on across epochs: envoys deduplicate per epoch, and a stop for an
	// epoch that already ended still needs an index above that epoch's start command.
	state.epoch = state.epoch.wrapping_add(1);

	match &state.pending_action {
		wire::PendingAction::PendingNone | wire::PendingAction::PendingSleep => {
			state.phase = wire::ActorPhase::Sleeping;
			set_pending_action(&mut state.pending_action, wire::PendingAction::PendingNone);
		}
		wire::PendingAction::PendingReallocate => {
			allocate_actor(state, effects, hint, Dispatch::Durable, now_unix_ms);
		}
		wire::PendingAction::PendingDestroy => destroy_actor(state, effects, None, now_unix_ms),
	}
}

/// Whether a start command and its starting notice are dispatched after their transition
/// commits, or before it when the allocation they name was committed by an earlier transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dispatch {
	Durable,
	Nondurable,
}

/// Brings the actor up for the current epoch, unless a pending sleep or destroy says otherwise.
/// `allocation` is the envoy the executor selected for this start; without one the pool had
/// no envoy, nothing was sent for the epoch, and the actor sleeps until the next wake tries
/// again.
fn allocate_actor(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: Option<wire::ActorAllocation>,
	dispatch: Dispatch,
	now_unix_ms: i64,
) {
	match &state.pending_action {
		wire::PendingAction::PendingNone | wire::PendingAction::PendingReallocate => {}
		wire::PendingAction::PendingSleep => {
			state.phase = wire::ActorPhase::Sleeping;
			set_pending_action(&mut state.pending_action, wire::PendingAction::PendingNone);
			return;
		}
		wire::PendingAction::PendingDestroy => {
			destroy_actor(state, effects, None, now_unix_ms);
			return;
		}
	}
	set_pending_action(&mut state.pending_action, wire::PendingAction::PendingNone);

	match allocation {
		Some(allocation) => start_actor(state, effects, allocation, dispatch, now_unix_ms),
		None => state.phase = wire::ActorPhase::Sleeping,
	}
}

/// Sends the start command for the current epoch to `allocation` and waits for the envoy to
/// report running. The start timer bounds that wait. The waiters are told where the actor is
/// starting and kept for the running notice.
fn start_actor(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: wire::ActorAllocation,
	dispatch: Dispatch,
	now_unix_ms: i64,
) {
	state.command_index += 1;

	// Every command of an incarnation takes the next index, starting from 1 after
	// `initialize`, so index 1 is its first start. No runner has seen the incarnation's
	// database volume yet, since it is named by `create_unix_ms`.
	let new_storage = state.command_index == 1;
	let start = Effect::Start {
		epoch: state.epoch,
		command_index: state.command_index,
		allocation: allocation.clone(),
		new_storage,
	};
	match dispatch {
		Dispatch::Durable => effects.execute(start),
		Dispatch::Nondurable => effects.nondurable_execute(start),
	}
	effects.arm_timer_after(
		wire::ActorTimer::StartTimeout(wire::StartTimeout { epoch: state.epoch }),
		EFFECT_TIMEOUT_MS,
	);

	state
		.start_waiters
		.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
	// The notice goes out with the start, so the gateway's pre-sent request follows the start
	// without waiting for this commit. A start sent before the commit names an allocation and
	// waiters that earlier commits stored, and a failed commit resends both unchanged.
	if !state.start_waiters.is_empty() {
		let notice = Effect::NotifyStartWaiters {
			waiters: state.start_waiters.clone(),
			kind: wire::ActorStartNoticeKind::ActorStartNoticeStarting(
				wire::ActorStartNoticeStarting {
					epoch: state.epoch,
					allocation: allocation.clone(),
				},
			),
		};
		match dispatch {
			Dispatch::Durable => effects.execute(notice),
			Dispatch::Nondurable => effects.nondurable_execute(notice),
		}
	}

	state.phase = wire::ActorPhase::Starting(wire::Starting { allocation });
}

/// Sends a stop command for `epoch` to `allocation` without changing the phase. The current
/// epoch's stop carries the pending action as its reason; a stale epoch's stop is a
/// reallocation, since the actor has moved on.
fn send_stop_command(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	epoch: u32,
	allocation: wire::ActorAllocation,
	pending_action: wire::PendingAction,
) {
	state.command_index += 1;

	effects.execute(Effect::SendStopCommand {
		epoch,
		command_index: state.command_index,
		allocation,
		pending_action,
	});
}

/// Moves the actor to `Stopping`: the stop command goes to its allocation and the stop timer
/// resends it until the envoy reports stopped or is proven gone.
fn stop_actor(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: wire::ActorAllocation,
) {
	send_stop_command(
		state,
		effects,
		state.epoch,
		allocation.clone(),
		state.pending_action,
	);
	effects.arm_timer_after(
		wire::ActorTimer::StopTimeout(wire::StopTimeout { epoch: state.epoch }),
		STOP_RESEND_MS,
	);

	state.phase = wire::ActorPhase::Stopping(wire::Stopping { allocation });
}

/// Sends the current epoch's stop again under its own command index and re-arms the stop
/// timer. Nothing advances the command index while `Stopping`, so the index is the original
/// stop's: an envoy hosting the epoch drops the copy as a duplicate, and one without it reports
/// the epoch stopped. The copy carries the current pending action as its reason, which matters
/// only when the original stop never arrived. `jitter_ms` delays the next resend.
fn resend_stop(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: wire::ActorAllocation,
	jitter_ms: u64,
) {
	effects.execute(Effect::SendStopCommand {
		epoch: state.epoch,
		command_index: state.command_index,
		allocation,
		pending_action: state.pending_action,
	});
	effects.arm_timer_after(
		wire::ActorTimer::StopTimeout(wire::StopTimeout { epoch: state.epoch }),
		STOP_RESEND_MS + jitter_ms,
	);
}

/// A try start, or a create of an actor that exists: the waiter is stored against the phase
/// the actor is in, then the actor is woken.
fn try_start(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	waiter: Option<wire::StartWaiter>,
	hint: Option<wire::ActorAllocation>,
	now_unix_ms: i64,
) {
	store_start_waiter(state, waiter, now_unix_ms);
	wake(state, effects, hint, now_unix_ms);
}

/// Stores a start waiter for the actor's next start, if the event carried one. Nothing is
/// stored while the actor runs, since the waiter would only hold a slot until the next start,
/// or once it is being destroyed. Expired waiters are pruned first; a waiter already present
/// keeps its slot and takes the later expiry. A full list leaves the waiter out, which the
/// command reports as full.
fn store_start_waiter(
	state: &mut wire::ActorState,
	waiter: Option<wire::StartWaiter>,
	now_unix_ms: i64,
) {
	let Some(waiter) = waiter else {
		return;
	};
	match &state.phase {
		wire::ActorPhase::Running(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed => return,
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping => {}
	}

	state
		.start_waiters
		.retain(|stored| stored.expires_unix_ms > now_unix_ms);

	let existing = state
		.start_waiters
		.iter_mut()
		.find(|stored| stored.worker_id == waiter.worker_id && stored.wait_id == waiter.wait_id);
	if let Some(existing) = existing {
		existing.expires_unix_ms = existing.expires_unix_ms.max(waiter.expires_unix_ms);
		return;
	}

	if state.start_waiters.len() >= state.start_waiters_capacity as usize {
		return;
	}

	state.start_waiters.push(waiter);
}

/// Whether the actor's state holds the waiter, so a command can tell a stored waiter from one
/// the full list left out.
pub fn has_start_waiter(state: &wire::ActorState, waiter: &wire::StartWaiter) -> bool {
	state
		.start_waiters
		.iter()
		.any(|stored| stored.worker_id == waiter.worker_id && stored.wait_id == waiter.wait_id)
}

/// Moves the actor to `Destroying` for this incarnation. The waiters still stored are told the
/// actor is destroyed and taken, since no start will notify them; the already-reserved path
/// sends its own notice first and leaves none behind.
fn destroy_actor(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	epoxy_already_reserved: Option<wire::Id>,
	now_unix_ms: i64,
) {
	state
		.start_waiters
		.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
	if !state.start_waiters.is_empty() {
		effects.execute(Effect::NotifyStartWaiters {
			waiters: std::mem::take(&mut state.start_waiters),
			kind: wire::ActorStartNoticeKind::ActorStartNoticeDestroyed,
		});
	}
	// The alarm belongs to this incarnation and must never wake the next one.
	effects.cancel_timer(wire::ActorTimer::AlarmTimeout);

	let mut destroy_effect_data = wire::DestroyEffectData {
		create_unix_ms: state.create_unix_ms,
		state: wire::DestroyState {
			phase: wire::DestroyPhase::DestroyPhasePending,
			attempt: 0,
			create_unix_ms: state.create_unix_ms,
		},
	};

	walk_destroy(
		&mut destroy_effect_data,
		state.index_bucket_count,
		effects,
		None,
	);
	state.phase = wire::ActorPhase::Destroying(wire::Destroying {
		destroy: destroy_effect_data,
		epoxy_already_reserved,
	});
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
	ReserveEpoxy {
		namespace: String,
		name: String,
		key: String,
		attempt: u32,
		create_unix_ms: i64,
	},
	WriteIndex {
		namespace: String,
		name: String,
		index_bucket_count: u64,
		create_unix_ms: i64,
		attempt: u32,
	},
	Start {
		epoch: u32,
		command_index: i64,
		allocation: wire::ActorAllocation,
		/// Whether this is the incarnation's first start, so no runner has written its
		/// storage. The executor then preloads the empty keyspace, which lets the runner skip
		/// its storage probes and the envoy create the database volume with its first commit.
		new_storage: bool,
	},
	SendStopCommand {
		epoch: u32,
		command_index: i64,
		allocation: wire::ActorAllocation,
		pending_action: wire::PendingAction,
	},
	NotifyStartWaiters {
		waiters: Vec<wire::StartWaiter>,
		kind: wire::ActorStartNoticeKind,
	},
	Destroy {
		attempt: u32,
		index_bucket_count: u64,
		create_unix_ms: i64,
	},
}

impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::ReserveEpoxy { .. } => "reserve_epoxy",
			Effect::WriteIndex { .. } => "write_index",
			Effect::Start { .. } => "start",
			Effect::SendStopCommand { .. } => "send_stop_command",
			Effect::NotifyStartWaiters { .. } => "notify_start_waiters",
			Effect::Destroy { .. } => "destroy",
		}
	}
}

/// A write the executor applies inside the transition's own transaction, so it is durable
/// exactly when the state is. See `Executor::write`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteEffect {
	/// **When**
	///
	/// Initialization of a new incarnation, in the same transaction that records it.
	///
	/// **Executor**
	///
	/// Puts the actor's data record into the state volume. No response: the record is durable
	/// with the state, and every later effect reads it from there.
	WriteData { data: wire::ActorData },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot;

impl rivet_state_machine::Snapshot for Snapshot {}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event(pub wire::ActorEvent);

impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match &self.0 {
			wire::ActorEvent::ReserveEpoxyState(_) => "reserve_epoxy_state",
			wire::ActorEvent::WriteIndexState(_) => "write_index_state",
			wire::ActorEvent::StartFailed(_) => "start_failed",
			wire::ActorEvent::Sleep => "sleep",
			wire::ActorEvent::TryStart(_) => "try_start",
			wire::ActorEvent::Create(_) => "create",
			wire::ActorEvent::Reallocate(_) => "reallocate",
			wire::ActorEvent::UpdateRoute(_) => "update_route",
			wire::ActorEvent::Destroy => "destroy",
			wire::ActorEvent::DestroyState(_) => "destroy_state",
			wire::ActorEvent::ToEnvoyEvent(_) => "to_envoy_event",
			wire::ActorEvent::EnvoyGone(_) => "envoy_gone",
		}
	}
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct State(pub wire::ActorState);

impl rivet_state_machine::StateMachineState for State {}


