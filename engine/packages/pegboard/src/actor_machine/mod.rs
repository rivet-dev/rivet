use crate::wire;
use anyhow::{Context, Result};
use rivet_data::families::VersionFamily;
use rivet_env::Clocks;
use rivet_state_machine::{Entropy, NoSlot};
use rivet_state_machine_durable::{DurableStateMachine, Effects, RuntimeEvent};
use serde::{Deserialize, Serialize};
use vbare::Versioned;

pub mod executor;

const EFFECT_TIMEOUT_MS: u64 = 15 * 1000;
const EFFECT_MAX_ATTEMPTS: u32 = 10;
/// How long a stop command goes unanswered before it is sent again. It is resent until the
/// envoy reports the epoch stopped, the executor proves the envoy gone, or the stop deadline
/// passes. The interval bounds how late after the envoy's lost threshold the proof comes.
const STOP_RESEND_MS: u64 = 5 * 1000;
/// How far past the stop threshold an unanswered stop ends its epoch as lost. The margin covers
/// the stop's delivery and the node's own grace, which RivetKit counts from when it receives
/// the stop, so a healthy node that drains for the full threshold never overlaps the next
/// epoch.
const STOP_LOST_MARGIN_MS: u64 = 5 * 1000;
/// The most a resend is delayed past `STOP_RESEND_MS`, drawn per resend, so the actors of one
/// envoy that were stopped together do not resend and read its record in lockstep.
const STOP_RESEND_JITTER_MS: u64 = 2 * 1000;
/// How long a due alarm that found no envoy to start its actor on first waits before it tries
/// again. Envoys reconnect about a second after the engine starts, so a short wait keeps the
/// alarm close to its deadline. The wait doubles with each retry.
const ALARM_NO_ENVOY_RETRY_BASE_MS: u64 = 1000;
/// The longest wait between two retries of a due alarm, so an alarm still fires soon after a
/// long outage ends.
const ALARM_NO_ENVOY_RETRY_MAX_MS: u64 = 30 * 1000;
/// The most a retry of a due alarm is delayed past its wait, drawn per retry, so the actors whose
/// alarms came due together do not retry in lockstep.
const ALARM_NO_ENVOY_RETRY_JITTER_MS: u64 = 2 * 1000;

/// How far past its lost threshold a disconnected envoy must be before it is judged gone. It
/// covers the delay between the engine's close and the envoy noticing it, and clock skew
/// between the worker that stamped the disconnect and the one judging it.
pub(crate) const ENVOY_GONE_MARGIN_MS: u64 = 5_000;
/// The `pegboard.actor` version whose timers carry an alarm's envoy check re-drive.
const ENVOY_CHECK_TIMER_VERSION: u16 = 2;
/// Replacement selection is retried independently after confirmed old-generation completion.
const ALLOCATION_RETRY_VERSION: u16 = 3;
/// The `pegboard.actor` version whose timers carry the stop deadline.
const STOP_THRESHOLD_TIMER_VERSION: u16 = 3;

pub struct ActorStateMachine;

impl DurableStateMachine for ActorStateMachine {
	type Slot = NoSlot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Timer = wire::ActorTimer;
	type Snapshot = Snapshot;
	type Preparation = Preparation;
	type WriteEffect = WriteEffect;

	const DEBUG_NAME: &'static str = "actor";

	const FAMILY: VersionFamily = VersionFamily::PEGBOARD_ACTOR;

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
		preparation: Preparation,
		effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	) {
		let State(state) = state;
		let Preparation {
			allocation: prepared_allocation,
			start_threshold_ms,
			stop_threshold_ms,
			start_waiters_capacity,
			envoy_lost_threshold_ms,
			write_version,
			create_placement,
		} = preparation;
		let now_unix_ms = raw_event.now.unix_ms;
		let stop_deadline = StopDeadline {
			wait_ms: stop_threshold_ms.max(envoy_lost_threshold_ms),
			write_version,
			now_unix_ms,
		};
		let entropy = raw_event.entropy.clone();
		let event = match raw_event.body {
			RuntimeEvent::Input(Event(input)) => RuntimeEvent::Input(input),
			RuntimeEvent::Timer(timer) => RuntimeEvent::Timer(timer),
		};

		// Process event
		match state.phase {
			wire::ActorPhase::Initializing(ref mut init) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::ReserveEpoxyState(effect_state)) => {
					walk_reserve_epoxy(
						&mut init.reserve_epoxy,
						state.create_unix_ms,
						KeyCheck::EpoxyAndV2,
						effects,
						Some(effect_state),
					);
					let reservation = init.reserve_epoxy.state.phase.clone();
					let allocation = init.allocation.clone();
					try_complete_initialization(
						state,
						effects,
						reservation,
						allocation,
						Dispatch::Nondurable,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::ReserveEpoxyTimeout(timeout)) => {
					walk_reserve_epoxy(
						&mut init.reserve_epoxy,
						state.create_unix_ms,
						KeyCheck::EpoxyAndV2,
						effects,
						Some(wire::ReserveEpoxyState {
							phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
					);
					let reservation = init.reserve_epoxy.state.phase.clone();
					let allocation = init.allocation.clone();
					try_complete_initialization(
						state,
						effects,
						reservation,
						allocation,
						Dispatch::Nondurable,
						start_threshold_ms,
						now_unix_ms,
					);
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
						store_start_waiter(
							state,
							event.waiter,
							start_waiters_capacity,
							now_unix_ms,
						);
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
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				// The start command did not reach the envoy, or the envoy never reported
				// running. Either way the envoy may hold the epoch, so it is only stopped, like
				// a lost report: the epoch ends on the stopped report, once the envoy is
				// proven gone, or at the stop deadline, so the next start never runs beside
				// this one.
				RuntimeEvent::Input(wire::ActorEvent::StartFailed(res)) => {
					if res.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						effects.cancel_timer(wire::ActorTimer::StartTimeout(wire::StartTimeout {
							epoch: state.epoch,
						}));
						stop_actor(state, effects, allocation, stop_deadline);
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::StartTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						stop_actor(state, effects, allocation, stop_deadline);
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
										stop_actor(state, effects, allocation, stop_deadline);
									}
								}
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_) => {
								handle_stopped(
									state,
									effects,
									prepared_allocation,
									start_threshold_ms,
									now_unix_ms,
									write_version,
									entropy,
								);
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
						// the epoch is only stopped. It ends on the envoy's stopped report, once
						// the envoy is proven gone, or at the stop deadline, so the next start
						// never runs beside this one.
						if event.lost {
							let allocation = phase.allocation.clone();
							effects.cancel_timer(wire::ActorTimer::StartTimeout(
								wire::StartTimeout { epoch: state.epoch },
							));
							stop_actor(state, effects, allocation, stop_deadline);
						}
						store_start_waiter(
							state,
							event.waiter,
							start_waiters_capacity,
							now_unix_ms,
						);
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
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				// The alarm fires on the envoy itself. The engine's timer only checks that the
				// envoy is still there: an envoy gone past its lost threshold has stopped the
				// epoch, which a request would find out through its failed open. An alarm has
				// no request, so the check stands in for one.
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					let allocation = phase.allocation.clone();
					check_envoy(
						state,
						effects,
						allocation,
						envoy_lost_threshold_ms,
						write_version,
					);
				}
				// The re-drive of an alarm's check. Past the envoy's lost threshold the record
				// decides on its own, so this check needs no re-drive of its own.
				RuntimeEvent::Timer(wire::ActorTimer::EnvoyCheckTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						effects.nondurable_execute(Effect::CheckEnvoy {
							epoch: state.epoch,
							allocation: phase.allocation.clone(),
						});
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::ToEnvoyEvent(event)) => {
					if event.epoch == state.epoch {
						match event.kind {
							wire::ToEnvoyEventKind::ToEnvoyEventKindRunning => {}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_) => {
								handle_stopped(
									state,
									effects,
									prepared_allocation,
									start_threshold_ms,
									now_unix_ms,
									write_version,
									entropy,
								);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
								let allocation = phase.allocation.clone();
								stop_actor(state, effects, allocation, stop_deadline);
							}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
								set_pending_action(
									&mut state.pending_action,
									wire::PendingAction::PendingDestroy,
								);

								let allocation = phase.allocation.clone();
								stop_actor(state, effects, allocation, stop_deadline);
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
						// on the stopped report, once the envoy is proven gone, or at the stop
						// deadline.
						let allocation = phase.allocation.clone();
						stop_actor(state, effects, allocation, stop_deadline);
						store_start_waiter(
							state,
							event.waiter,
							start_waiters_capacity,
							now_unix_ms,
						);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {
					let allocation = phase.allocation.clone();
					stop_actor(state, effects, allocation, stop_deadline);
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
					stop_actor(state, effects, allocation, stop_deadline);
				}
				_ => {}
			},
			wire::ActorPhase::Stopping(ref mut phase) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					wake(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				// The stop is still unanswered. It may have been lost with a connection the
				// envoy has since replaced while it keeps the epoch running, so short silence
				// proves nothing: the same stop is sent again and the timer re-armed.
				RuntimeEvent::Timer(wire::ActorTimer::StopTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						let jitter_ms = entropy.rng().gen_range(0..STOP_RESEND_JITTER_MS);
						resend_stop(state, effects, allocation, jitter_ms);
					}
				}
				// The stop went unanswered past the stop threshold and its margin. A node that
				// received any of the stops has stopped the epoch within its grace, and one that
				// never answers, such as one that never received the epoch, would otherwise hold
				// the actor in `Stopping` forever. The epoch ends as lost, as for a gone envoy. A
				// deadline of an earlier epoch is ignored.
				RuntimeEvent::Timer(wire::ActorTimer::StopThresholdTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						tracing::warn!(
							epoch = timeout.epoch,
							waited_ms = now_unix_ms.saturating_sub(timeout.stop_unix_ms),
							stop_threshold_ms,
							"actor stop went unanswered past the stop threshold, ending the epoch as lost; the node may not hold the epoch or may be unresponsive"
						);
						handle_stopped(
							state,
							effects,
							prepared_allocation,
							start_threshold_ms,
							now_unix_ms,
							write_version,
							entropy,
						);
					}
				}
				// The executor found the envoy disconnected for longer than its lost
				// threshold, past which an envoy stops every actor it holds, so the epoch can
				// end without its stopped report. A late report of an earlier epoch is ignored.
				RuntimeEvent::Input(wire::ActorEvent::EnvoyGone(gone)) => {
					if gone.epoch == state.epoch {
						handle_stopped(
							state,
							effects,
							prepared_allocation,
							start_threshold_ms,
							now_unix_ms,
							write_version,
							entropy,
						);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::ToEnvoyEvent(event)) => {
					if event.epoch == state.epoch {
						match event.kind {
							wire::ToEnvoyEventKind::ToEnvoyEventKindRunning => {}
							wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_) => {
								handle_stopped(
									state,
									effects,
									prepared_allocation,
									start_threshold_ms,
									now_unix_ms,
									write_version,
									entropy,
								);
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
				// while the envoy still hosts it. The stopped report, the envoy's proven
				// absence or the stop deadline ends it.
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						set_pending_action(
							&mut state.pending_action,
							wire::PendingAction::PendingReallocate,
						);
						store_start_waiter(
							state,
							event.waiter,
							start_waiters_capacity,
							now_unix_ms,
						);
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
			wire::ActorPhase::Sleeping(_) => match event {
				RuntimeEvent::Input(wire::ActorEvent::TryStart(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(event)) => {
					try_start(
						state,
						effects,
						event.waiter,
						prepared_allocation,
						start_threshold_ms,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				// With no envoy to start on, as right after the engine starts and before its
				// envoys reconnect, the alarm is armed again so the wake is not lost.
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => match prepared_allocation {
					Some(_) => wake(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						now_unix_ms,
					),
					None => rearm_alarm(state, effects, entropy),
				},
				RuntimeEvent::Input(wire::ActorEvent::Reallocate(event)) => {
					if event.epoch == state.epoch {
						allocate_actor(
							state,
							effects,
							prepared_allocation,
							Dispatch::Durable,
							start_threshold_ms,
							now_unix_ms,
						);
						store_start_waiter(
							state,
							event.waiter,
							start_waiters_capacity,
							now_unix_ms,
						);
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::AllocationRetry(retry)) => {
					if retry.epoch == state.epoch
						&& state.pending_action == wire::PendingAction::PendingReallocate
					{
						if prepared_allocation.is_some() {
							allocate_actor(
								state,
								effects,
								prepared_allocation,
								Dispatch::Durable,
								start_threshold_ms,
								now_unix_ms,
							);
						} else {
							arm_allocation_retry(state, effects, retry.attempt, entropy);
						}
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::Sleep) => {
					if state.pending_action == wire::PendingAction::PendingReallocate {
						state.pending_action = wire::PendingAction::PendingNone;
						cancel_allocation_retry(state, effects);
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
					destroy_actor(state, effects, None, now_unix_ms);
				}
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Recall(recall),
				)) => {
					recall_to_v2(state, recall.migration_id, effects, now_unix_ms);
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
						// A key V2 holds stays V2's, so its next create goes there without
						// asking V2 again.
						state.phase = match phase.key_owner {
							Some(wire::KeyOwner::KeyOwnerV2) => wire::ActorPhase::DelegatedToV2,
							Some(wire::KeyOwner::KeyOwnerDatacenter(_)) | None => {
								wire::ActorPhase::Destroyed
							}
						};
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
				// A create whose key belongs in V2 settles that here, so every later create of
				// the key follows it whatever the share says by then. Nothing starts, and the
				// command reports the delegation to the caller.
				RuntimeEvent::Input(wire::ActorEvent::Create(create))
					if create.init.key.is_some()
						&& create_placement == CreatePlacement::MigrationV2 =>
				{
					state.phase = wire::ActorPhase::DelegatedToV2;
				}
				RuntimeEvent::Input(wire::ActorEvent::Create(create)) => {
					initialize(
						state,
						create.init,
						create.waiter,
						prepared_allocation,
						effects,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				// A try start stores no waiter; the command reports the actor destroyed.
				RuntimeEvent::Input(wire::ActorEvent::TryStart(_))
				| RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {}
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Begin(begin),
				)) => {
					begin_migration_from_v2(state, begin, effects);
				}
				_ => {}
			},
			// V2 may still serve the actor, so nothing here starts it. It came from `Destroyed`
			// or `Sleeping`, which have no epoch outstanding, so no envoy, start or stop event
			// can belong to it.
			wire::ActorPhase::MigratingFromV2(ref mut held) => match event {
				// No waiter is stored; the command reports the hold, and the request goes to V2.
				RuntimeEvent::Input(wire::ActorEvent::TryStart(_))
				| RuntimeEvent::Input(wire::ActorEvent::Create(_)) => {}
				// A recalled actor keeps its alarm, but nothing starts a held actor. The fire is
				// remembered so an activation that keeps the alarm wakes the actor at once.
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					held.alarm_due = true;
				}
				RuntimeEvent::Input(wire::ActorEvent::ReserveEpoxyState(effect_state)) => {
					if let Some(reserve_epoxy) = &mut held.reserve_epoxy {
						walk_reserve_epoxy(
							reserve_epoxy,
							state.create_unix_ms,
							KeyCheck::Epoxy,
							effects,
							Some(effect_state),
						);
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::ReserveEpoxyTimeout(timeout)) => {
					if let Some(reserve_epoxy) = &mut held.reserve_epoxy {
						walk_reserve_epoxy(
							reserve_epoxy,
							state.create_unix_ms,
							KeyCheck::Epoxy,
							effects,
							Some(wire::ReserveEpoxyState {
								phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
								attempt: timeout.attempt,
								create_unix_ms: timeout.create_unix_ms,
							}),
						);
					}
				}
				// A recall can leave an activation's index write in flight. It writes this
				// incarnation's entry, which a recalled actor keeps anyway.
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
				// The user's delete wins once V3 owns the actor: the activation destroys it.
				RuntimeEvent::Input(wire::ActorEvent::Destroy) => {
					set_pending_action(
						&mut state.pending_action,
						wire::PendingAction::PendingDestroy,
					);
				}
				// Every migrator event names its migration, so a late or repeated event of an
				// abandoned migration finds another id and changes nothing; the migrator reads the
				// phase back to learn what applied.
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Begin(begin),
				)) => {
					// A begin that outlived its migration left the actor held with no migrator
					// behind it. A newer migration takes the hold over and imports into the same
					// incarnation, keeping its reservation.
					if begin.migration_id > held.migration_id {
						held.migration_id = begin.migration_id;
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Activate(activate),
				)) => {
					let key_reserved = match &held.reserve_epoxy {
						None => true,
						Some(reserve_epoxy) => match reserve_epoxy.state.phase {
							wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => true,
							wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending
							| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(_)
							| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed
							| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached
							| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2 => false,
						},
					};
					if activate.migration_id == held.migration_id && key_reserved {
						let alarm_due = held.alarm_due;
						activate_from_v2(
							state,
							activate.alarm_unix_ms,
							alarm_due,
							effects,
							now_unix_ms,
						);
					}
				}
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Discard(discard),
				)) => {
					if discard.migration_id == held.migration_id {
						destroy_actor(state, effects, None, now_unix_ms);
					}
				}
				// Only a sleeping actor is recalled.
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Recall(_),
				)) => {}
				_ => {}
			},
			// V2 serves the key, so nothing here starts it. It has no epoch outstanding and no
			// alarm, so no envoy, start, stop or alarm event can belong to it.
			wire::ActorPhase::DelegatedToV2 => match event {
				// Once the namespace no longer migrates in from V2, the key is this cluster's
				// again, and a create starts it here as it would a destroyed actor.
				RuntimeEvent::Input(wire::ActorEvent::Create(create))
					if create_placement == CreatePlacement::Standard =>
				{
					initialize(
						state,
						create.init,
						create.waiter,
						prepared_allocation,
						effects,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				// No waiter is stored; the command reports the delegation, and the request goes
				// to V2.
				RuntimeEvent::Input(wire::ActorEvent::TryStart(_))
				| RuntimeEvent::Input(wire::ActorEvent::Create(_)) => {}
				// The migrator moves the key's V2 actor in, under the same id.
				RuntimeEvent::Input(wire::ActorEvent::MigrateFromV2(
					wire::MigrateFromV2::MigrateFromV2Begin(begin),
				)) => {
					begin_migration_from_v2(state, begin, effects);
				}
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

	fn serialize_state(state: &Self::State, version: u16) -> Result<Vec<u8>> {
		rivet_data::versioned::ActorState::serialize_with_embedded_version(
			&state.0,
			rivet_data::pegboard_actor::record_write_version(version),
		)
	}

	fn deserialize_state(bytes: &[u8]) -> Result<Self::State> {
		rivet_data::versioned::ActorState::deserialize_with_embedded_version(bytes).map(State)
	}

	fn timer_id(timer: &Self::Timer) -> Vec<u8> {
		(TimerId::of(timer) as u32).to_be_bytes().to_vec()
	}

	fn serialize_timer(timer: &Self::Timer, version: u16) -> Result<Vec<u8>> {
		rivet_data::versioned::ActorTimer::serialize_with_embedded_version(timer, version)
	}

	fn serialize_rearmed_timer(
		timer: &Self::Timer,
		version: u16,
		fired_payload: &[u8],
	) -> Result<Vec<u8>> {
		let version = if matches!(timer, wire::ActorTimer::AllocationRetry(_)) {
			// Discovery may fall back to v2 while a committed v3 retry is firing.
			let header = fired_payload
				.first_chunk::<2>()
				.context("missing actor timer version")?;
			version.max(u16::from_le_bytes(*header))
		} else {
			version
		};
		Self::serialize_timer(timer, version)
	}

	fn deserialize_timer(bytes: &[u8]) -> Result<Self::Timer> {
		rivet_data::versioned::ActorTimer::deserialize_with_embedded_version(bytes)
	}
}

/// What the executor's `prepare` supplies to one transition: the envoy selected for a start
/// the event causes, as `needs_allocation` names them, and the configuration the transition
/// reads, since the core never reads the config itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preparation {
	/// The envoy a start this event causes goes to. `None` when the event starts nothing or
	/// the pool has no envoy, in which case the actor sleeps.
	pub allocation: Option<wire::ActorAllocation>,
	/// How long a start waits for the envoy's running report before the epoch is stopped;
	/// `pegboard.actor_start_threshold`.
	pub start_threshold_ms: u64,
	/// How long a stop goes unanswered, past the stop's margin, before the epoch ends as lost;
	/// `pegboard.actor_stop_threshold`.
	pub stop_threshold_ms: u64,
	/// Most start waiters the actor keeps for its next start; a waiter past it is left out and
	/// reported as such. `pegboard.actor_start_waiters_capacity`.
	pub start_waiters_capacity: u32,
	/// How long an envoy may go without the engine before it stops every actor it holds, and
	/// so how long the engine waits before judging its record; `pegboard.envoy_lost_threshold`.
	pub envoy_lost_threshold_ms: u64,
	/// The `pegboard.actor` version the actor's volume writes: the highest every live storage
	/// worker reads. A timer that version cannot carry is not armed.
	pub write_version: u16,
	/// Where a keyed create belongs, read from the migration config for a `Create` event, and
	/// `Standard` for any other event.
	pub create_placement: CreatePlacement,
}

impl Default for Preparation {
	fn default() -> Self {
		let defaults = rivet_config::config::pegboard::Pegboard::default();
		Self {
			allocation: None,
			start_threshold_ms: defaults.actor_start_threshold(),
			stop_threshold_ms: defaults.actor_stop_threshold(),
			start_waiters_capacity: defaults.actor_start_waiters_capacity(),
			envoy_lost_threshold_ms: defaults.envoy_lost_threshold(),
			write_version: rivet_data::generated::PEGBOARD_ACTOR_VERSION,
			create_placement: CreatePlacement::Standard,
		}
	}
}

/// Where a keyed create belongs while its namespace may be migrating in from V2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatePlacement {
	/// The namespace is not migrating in from V2, so the key is this cluster's. A key delegated
	/// to V2 during an earlier migration is created here again.
	Standard,
	/// The namespace migrates in from V2, and the create forces actor v3 or its key's share is
	/// below `create_ratio`. The reservation still asks V2 whether it holds the key, and a key
	/// already delegated to V2 stays there.
	MigrationV3,
	/// The namespace migrates in from V2, and the key's share is at or past `create_ratio`, so
	/// a destroyed actor is delegated to V2 instead of created.
	MigrationV2,
}

/// Leaves `Initializing` once the reservation settled: on to the start when it is reserved, to
/// a destroy when it ran out of attempts, and to a destroy that tells the waiters who owns the
/// key when another datacenter holds it. Called from the create and after each reservation
/// result or timeout, with the `Initializing` phase's reservation state and the allocation it
/// holds; it does nothing while the reservation is pending. `dispatch` says where
/// the allocation chosen at create time is committed: in this transition when called from the
/// create, so the start waits for the commit, or in an earlier one when called from a
/// reservation result, so the start can go out before this commit. A failed commit then
/// resends the same allocation, epoch and command index, which the envoy drops as a replay.
fn try_complete_initialization(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	reservation: wire::ReserveEpoxyPhase,
	allocation: Option<wire::ActorAllocation>,
	dispatch: Dispatch,
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	match reservation {
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => {
			allocate_actor(
				state,
				effects,
				allocation,
				dispatch,
				start_threshold_ms,
				now_unix_ms,
			);
		}
		wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending => {}
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(already_reserved) => {
			// Another datacenter holds the key. The waiters learn the owner now; no start will
			// ever notify them.
			let reservation_id = already_reserved.reservation_id;
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
			destroy_actor(
				state,
				effects,
				Some(wire::KeyOwner::KeyOwnerDatacenter(
					wire::KeyOwnerDatacenter { reservation_id },
				)),
				now_unix_ms,
			);
		}
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2 => {
			// V2 holds the key. The waiters' requests go there now; no start will ever notify
			// them. The record ends delegated to V2, so the key's later creates go there too.
			state
				.start_waiters
				.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
			if !state.start_waiters.is_empty() {
				effects.execute(Effect::NotifyStartWaiters {
					waiters: std::mem::take(&mut state.start_waiters),
					kind: wire::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2,
				});
			}
			destroy_actor(
				state,
				effects,
				Some(wire::KeyOwner::KeyOwnerV2),
				now_unix_ms,
			);
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
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	let create_unix_ms = now_unix_ms.max(state.create_unix_ms.saturating_add(1));
	state.create_unix_ms = create_unix_ms;
	state.storage_origin = wire::StorageOrigin::Created;
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
	walk_reserve_epoxy(
		&mut init_phase.reserve_epoxy,
		create_unix_ms,
		KeyCheck::EpoxyAndV2,
		effects,
		None,
	);
	walk_write_index(
		&mut state.write_index,
		state.index_bucket_count,
		create_unix_ms,
		effects,
		None,
	);

	let reservation = init_phase.reserve_epoxy.state.phase.clone();
	let allocation = init_phase.allocation.clone();
	state.phase = wire::ActorPhase::Initializing(init_phase);
	try_complete_initialization(
		state,
		effects,
		reservation,
		allocation,
		Dispatch::Durable,
		start_threshold_ms,
		now_unix_ms,
	);
}

/// The `walk_*` helpers advance one effect's attempt state. `new_state` is the executor's
/// result or the attempt's timeout; it is ignored when it names another incarnation by
/// `create_unix_ms` or an attempt older than the current one. Without it, the first attempt
/// is started.
fn walk_reserve_epoxy(
	effect_data: &mut wire::ReserveEpoxyEffectData,
	create_unix_ms: i64,
	key_check: KeyCheck,
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
			| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(_)
			| wire::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2 => {
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
		key_check,
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

/// Checks the running epoch's envoy, as a gateway request would, and arms the check's
/// re-drive. A check is lost with the process that runs it; the timer runs it again once the
/// envoy's record can decide on its own, past the lost threshold and the margin for clock
/// skew. The timer is only armed where the actor's volume can store it: until every live
/// storage worker reads the version that carries it, the check runs once.
fn check_envoy(
	state: &wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: wire::ActorAllocation,
	envoy_lost_threshold_ms: u64,
	write_version: u16,
) {
	effects.nondurable_execute(Effect::CheckEnvoy {
		epoch: state.epoch,
		allocation,
	});
	if write_version >= ENVOY_CHECK_TIMER_VERSION {
		effects.arm_timer_after(
			wire::ActorTimer::EnvoyCheckTimeout(wire::EnvoyCheckTimeout { epoch: state.epoch }),
			envoy_lost_threshold_ms.saturating_add(ENVOY_GONE_MARGIN_MS),
		);
	}
}

/// Brings the actor back up for a wake request: the alarm firing, or a try start or create.
/// A sleeping actor allocates. An actor stopping for a sleep is reallocated instead, and one on
/// its way up drops a pending sleep so it is not stopped on arrival; a pending destroy is kept
/// either way. A starting actor resends its committed command in case its previous host lost
/// the dispatch. A running actor needs nothing from a try start, and a destroyed one is not
/// revived here.
fn wake(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	hint: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	match &state.phase {
		wire::ActorPhase::Sleeping(_) => {
			allocate_actor(
				state,
				effects,
				hint,
				Dispatch::Durable,
				start_threshold_ms,
				now_unix_ms,
			);
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
					new_storage: is_new_storage(state),
				});
			}
		}
		wire::ActorPhase::Running(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed
		| wire::ActorPhase::MigratingFromV2(_)
		| wire::ActorPhase::DelegatedToV2 => {}
	}
}

/// Whether `event` applied to `state` starts the actor, so the executor's `prepare` selects
/// the envoy for it. Exact by construction: it runs on the state the transition will use. A
/// start it names for which no envoy exists sends nothing and sleeps.
pub fn needs_allocation(
	state: &wire::ActorState,
	event: &RuntimeEvent<Event, wire::ActorTimer>,
) -> bool {
	needs_allocation_at_version(state, event, rivet_data::generated::PEGBOARD_ACTOR_VERSION)
}

pub(crate) fn needs_allocation_at_version(
	state: &wire::ActorState,
	event: &RuntimeEvent<Event, wire::ActorTimer>,
	write_version: u16,
) -> bool {
	let reallocation_pending = state.pending_action == wire::PendingAction::PendingReallocate;
	let sleeping = matches!(state.phase, wire::ActorPhase::Sleeping(_));
	match event {
		RuntimeEvent::Input(Event(input)) => match input {
			wire::ActorEvent::TryStart(_) => sleeping,
			wire::ActorEvent::Create(_) => {
				sleeping
					|| matches!(
						state.phase,
						wire::ActorPhase::Destroyed | wire::ActorPhase::DelegatedToV2
					)
			}
			wire::ActorEvent::Reallocate(event) if event.epoch == state.epoch => {
				match state.phase {
					wire::ActorPhase::Sleeping(_) => true,
					wire::ActorPhase::Initializing(_)
					| wire::ActorPhase::Starting(_)
					| wire::ActorPhase::Running(_)
					| wire::ActorPhase::Stopping(_)
					| wire::ActorPhase::Destroying(_)
					| wire::ActorPhase::Destroyed
					| wire::ActorPhase::MigratingFromV2(_)
					| wire::ActorPhase::DelegatedToV2 => false,
				}
			}
			wire::ActorEvent::ToEnvoyEvent(event) if event.epoch == state.epoch => {
				matches!(
					event.kind,
					wire::ToEnvoyEventKind::ToEnvoyEventKindStopped(_)
				) && write_version < ALLOCATION_RETRY_VERSION
					&& reallocation_pending
					&& matches!(
						state.phase,
						wire::ActorPhase::Starting(_)
							| wire::ActorPhase::Running(_)
							| wire::ActorPhase::Stopping(_)
					)
			}
			wire::ActorEvent::EnvoyGone(event) if event.epoch == state.epoch => {
				write_version < ALLOCATION_RETRY_VERSION
					&& reallocation_pending
					&& matches!(state.phase, wire::ActorPhase::Stopping(_))
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
			| wire::ActorEvent::Destroy
			| wire::ActorEvent::MigrateFromV2(_) => false,
		},
		RuntimeEvent::Timer(timer) => match timer {
			wire::ActorTimer::AlarmTimeout => sleeping,
			wire::ActorTimer::AllocationRetry(retry) => {
				sleeping && reallocation_pending && retry.epoch == state.epoch
			}
			wire::ActorTimer::StopThresholdTimeout(timeout) if timeout.epoch == state.epoch => {
				write_version < ALLOCATION_RETRY_VERSION
					&& reallocation_pending
					&& matches!(state.phase, wire::ActorPhase::Stopping(_))
			}
			wire::ActorTimer::StopThresholdTimeout(_)
			| wire::ActorTimer::StartTimeout(_)
			| wire::ActorTimer::StopTimeout(_)
			| wire::ActorTimer::EnvoyCheckTimeout(_)
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

/// Arms the alarm again for a wake that found no envoy to start the sleeping actor on. Each
/// retry waits longer, up to a limit, and there is no limit on the retries themselves.
fn rearm_alarm(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	entropy: Entropy,
) {
	let wire::ActorPhase::Sleeping(sleeping) = &mut state.phase else {
		return;
	};
	let wait_ms = 1u64
		.checked_shl(sleeping.wake_retries)
		.map_or(u64::MAX, |multiplier| {
			ALARM_NO_ENVOY_RETRY_BASE_MS.saturating_mul(multiplier)
		})
		.min(ALARM_NO_ENVOY_RETRY_MAX_MS);
	sleeping.wake_retries = sleeping.wake_retries.saturating_add(1);

	let jitter_ms = entropy.rng().gen_range(0..ALARM_NO_ENVOY_RETRY_JITTER_MS);
	effects.arm_timer_after(wire::ActorTimer::AlarmTimeout, wait_ms + jitter_ms);
}

/// Retry only replacement allocation: this identity never replaces the actor's user alarm.
fn arm_allocation_retry(
	state: &wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	attempt: u32,
	entropy: Entropy,
) {
	let wait = 1u64
		.checked_shl(attempt)
		.unwrap_or(u64::MAX)
		.saturating_mul(ALARM_NO_ENVOY_RETRY_BASE_MS)
		.min(ALARM_NO_ENVOY_RETRY_MAX_MS);
	let jitter = entropy.rng().gen_range(0..ALARM_NO_ENVOY_RETRY_JITTER_MS);
	effects.arm_timer_after(
		wire::ActorTimer::AllocationRetry(wire::AllocationRetry {
			epoch: state.epoch,
			attempt: attempt.saturating_add(1),
		}),
		wait.saturating_add(jitter),
	);
}

fn cancel_allocation_retry(
	state: &wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
) {
	effects.cancel_timer(wire::ActorTimer::AllocationRetry(wire::AllocationRetry {
		epoch: state.epoch,
		attempt: 0,
	}));
}

/// Puts the actor to sleep. An actor already asleep keeps its alarm's retry count, so a request
/// that found no envoy either does not restart the backoff.
fn enter_sleeping(state: &mut wire::ActorState) {
	if !matches!(state.phase, wire::ActorPhase::Sleeping(_)) {
		state.phase = wire::ActorPhase::Sleeping(wire::Sleeping { wake_retries: 0 });
	}
}

/// Ends the current epoch: its timers are cancelled, the epoch moves on, and the pending action
/// decides whether the actor sleeps, starts again or is destroyed. `hint` is the envoy a
/// command pre-selected for a start this stop leads to.
fn handle_stopped(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	hint: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	now_unix_ms: i64,
	write_version: u16,
	entropy: Entropy,
) {
	effects.cancel_timer(wire::ActorTimer::StartTimeout(wire::StartTimeout {
		epoch: state.epoch,
	}));
	effects.cancel_timer(wire::ActorTimer::StopTimeout(wire::StopTimeout {
		epoch: state.epoch,
	}));
	effects.cancel_timer(wire::ActorTimer::EnvoyCheckTimeout(
		wire::EnvoyCheckTimeout { epoch: state.epoch },
	));
	cancel_stop_deadline(effects, state.epoch);

	// The command index runs on across epochs: envoys deduplicate per epoch, and a stop for an
	// epoch that already ended still needs an index above that epoch's start command.
	state.epoch = state.epoch.wrapping_add(1);

	match &state.pending_action {
		wire::PendingAction::PendingNone | wire::PendingAction::PendingSleep => {
			enter_sleeping(state);
			set_pending_action(&mut state.pending_action, wire::PendingAction::PendingNone);
		}
		wire::PendingAction::PendingReallocate if write_version >= ALLOCATION_RETRY_VERSION => {
			enter_sleeping(state);
			arm_allocation_retry(state, effects, 0, entropy);
		}
		wire::PendingAction::PendingReallocate => {
			allocate_actor(
				state,
				effects,
				hint,
				Dispatch::Durable,
				start_threshold_ms,
				now_unix_ms,
			);
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
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	match &state.pending_action {
		wire::PendingAction::PendingNone | wire::PendingAction::PendingReallocate => {}
		wire::PendingAction::PendingSleep => {
			enter_sleeping(state);
			set_pending_action(&mut state.pending_action, wire::PendingAction::PendingNone);
			return;
		}
		wire::PendingAction::PendingDestroy => {
			destroy_actor(state, effects, None, now_unix_ms);
			return;
		}
	}
	if allocation.is_none()
		&& state.pending_action == wire::PendingAction::PendingReallocate
		&& matches!(state.phase, wire::ActorPhase::Sleeping(_))
	{
		enter_sleeping(state);
		return;
	}
	set_pending_action(&mut state.pending_action, wire::PendingAction::PendingNone);

	match allocation {
		Some(allocation) => start_actor(
			state,
			effects,
			allocation,
			dispatch,
			start_threshold_ms,
			now_unix_ms,
		),
		None => enter_sleeping(state),
	}
}

/// Sends the start command for the current epoch to `allocation` and waits for the envoy to
/// report running. The start timer bounds that wait to `start_threshold_ms`. The waiters are told where the actor is
/// starting and kept for the running notice.
fn start_actor(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: wire::ActorAllocation,
	dispatch: Dispatch,
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	cancel_allocation_retry(state, effects);
	state.command_index += 1;

	let start = Effect::Start {
		epoch: state.epoch,
		command_index: state.command_index,
		allocation: allocation.clone(),
		new_storage: is_new_storage(state),
	};
	match dispatch {
		Dispatch::Durable => effects.execute(start),
		Dispatch::Nondurable => effects.nondurable_execute(start),
	}
	effects.arm_timer_after(
		wire::ActorTimer::StartTimeout(wire::StartTimeout { epoch: state.epoch }),
		start_threshold_ms,
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

/// Whether the current start is the incarnation's first over storage it created. Every command
/// of an incarnation takes the next index, starting from 1, so index 1 is its first start, and no
/// runner has seen the database volume named by its `create_unix_ms`. An imported incarnation's
/// volume already holds the data imported into it.
fn is_new_storage(state: &wire::ActorState) -> bool {
	let created = match state.storage_origin {
		wire::StorageOrigin::Created => true,
		wire::StorageOrigin::Imported => false,
	};
	state.command_index == 1 && created
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

/// What a transition that begins a stop needs to arm the stop deadline: how long the stop may
/// go unanswered, the version the actor's volume writes, and the event's clock.
#[derive(Clone, Copy, Debug)]
struct StopDeadline {
	/// The longer of the stop threshold and the envoy lost threshold. An envoy cut off from the
	/// engine keeps running its actors until its lost threshold passes, so ending the epoch any
	/// sooner could start the next epoch while the old one still runs.
	wait_ms: u64,
	write_version: u16,
	now_unix_ms: i64,
}

/// Moves the actor to `Stopping`: the stop command goes to its allocation, the stop timer
/// resends it until the envoy reports stopped or is proven gone, and the stop deadline ends
/// the epoch as lost once the longer of the stop and envoy lost thresholds, plus its margin,
/// passes unanswered. Every path into
/// `Stopping` comes through here, so the deadline is armed once per epoch and counted from its
/// first stop. It is armed only where the actor's volume can store it: until every live storage
/// worker reads the version that carries it, the stop waits as before.
fn stop_actor(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: wire::ActorAllocation,
	deadline: StopDeadline,
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
	if deadline.write_version >= STOP_THRESHOLD_TIMER_VERSION {
		effects.arm_timer_after(
			wire::ActorTimer::StopThresholdTimeout(wire::StopThresholdTimeout {
				epoch: state.epoch,
				stop_unix_ms: deadline.now_unix_ms,
			}),
			deadline.wait_ms.saturating_add(STOP_LOST_MARGIN_MS),
		);
	}

	state.phase = wire::ActorPhase::Stopping(wire::Stopping { allocation });
}

/// Cancels `epoch`'s stop deadline. The runner names an actor timer by its variant alone, so
/// the cancel does not need the time the stop began.
fn cancel_stop_deadline(effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>, epoch: u32) {
	effects.cancel_timer(wire::ActorTimer::StopThresholdTimeout(
		wire::StopThresholdTimeout {
			epoch,
			stop_unix_ms: 0,
		},
	));
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
	start_threshold_ms: u64,
	start_waiters_capacity: u32,
	now_unix_ms: i64,
) {
	store_start_waiter(state, waiter, start_waiters_capacity, now_unix_ms);
	wake(state, effects, hint, start_threshold_ms, now_unix_ms);
}

/// Stores a start waiter for the actor's next start, if the event carried one. Nothing is
/// stored while the actor runs, since the waiter would only hold a slot until the next start,
/// or once it is being destroyed. Expired waiters are pruned first; a waiter already present
/// keeps its slot and takes the later expiry. A full list leaves the waiter out, which the
/// command reports as full.
fn store_start_waiter(
	state: &mut wire::ActorState,
	waiter: Option<wire::StartWaiter>,
	start_waiters_capacity: u32,
	now_unix_ms: i64,
) {
	let Some(waiter) = waiter else {
		return;
	};
	match &state.phase {
		wire::ActorPhase::Running(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed
		| wire::ActorPhase::MigratingFromV2(_)
		| wire::ActorPhase::DelegatedToV2 => return,
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_) => {}
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

	if state.start_waiters.len() >= start_waiters_capacity as usize {
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
	key_owner: Option<wire::KeyOwner>,
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
	cancel_allocation_retry(state, effects);
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
		key_owner,
	});
}

/// Starts the incarnation a V2 migration imports into, as `initialize` does for a create, but
/// held: nothing starts it and no index entry is written until activation. The data record
/// commits with the hold, and the incarnation's `create_unix_ms` names the database volume the
/// migrator imports into. It is V2's creation time, so listings keep the real creation order,
/// raised past the previous incarnation's like any new incarnation. A keyed actor reserves its
/// key as a create does.
fn begin_migration_from_v2(
	state: &mut wire::ActorState,
	begin: wire::MigrateFromV2Begin,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
) {
	let create_unix_ms = begin
		.create_unix_ms
		.max(state.create_unix_ms.saturating_add(1));
	state.create_unix_ms = create_unix_ms;
	state.storage_origin = wire::StorageOrigin::Imported;
	state.command_index = 0;
	state.pending_action = wire::PendingAction::PendingNone;
	state.start_waiters.clear();

	state.write_index = wire::WriteIndexEffectData {
		namespace: begin.init.namespace.clone(),
		name: begin.init.name.clone(),
		create_unix_ms,
		state: wire::WriteIndexState {
			phase: wire::WriteIndexPhase::WriteIndexPhasePending,
			attempt: 0,
			create_unix_ms,
		},
	};
	let mut reserve_epoxy = wire::ReserveEpoxyEffectData {
		namespace: begin.init.namespace.clone(),
		name: begin.init.name.clone(),
		key: begin.init.key.clone(),
		state: wire::ReserveEpoxyState {
			phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
			attempt: 0,
			create_unix_ms,
		},
	};

	effects.write(WriteEffect::WriteData {
		data: wire::ActorData {
			actor_id: begin.init.actor_id,
			namespace: begin.init.namespace,
			pool_name: begin.init.pool_name,
			name: begin.init.name,
			key: begin.init.key,
			input: begin.init.input,
			create_unix_ms,
		},
	});
	walk_reserve_epoxy(
		&mut reserve_epoxy,
		create_unix_ms,
		KeyCheck::Epoxy,
		effects,
		None,
	);

	state.phase = wire::ActorPhase::MigratingFromV2(wire::MigratingFromV2 {
		migration_id: begin.migration_id,
		reserve_epoxy: Some(reserve_epoxy),
		alarm_due: false,
	});
}

/// Hands a held actor to V3 once V2 gave it up: it sleeps, and its index entry is written so it
/// lists. `alarm_unix_ms` replaces the actor's alarm; without it the actor keeps its own, and
/// `alarm_due` says that alarm came due while the actor was held, so it fires at once. A begun
/// actor has no alarm of its own. A destroy that arrived while it was held destroys it instead.
fn activate_from_v2(
	state: &mut wire::ActorState,
	alarm_unix_ms: Option<i64>,
	alarm_due: bool,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	now_unix_ms: i64,
) {
	if let wire::PendingAction::PendingDestroy = state.pending_action {
		destroy_actor(state, effects, None, now_unix_ms);
		return;
	}

	enter_sleeping(state);
	match alarm_unix_ms {
		Some(alarm_unix_ms) => set_alarm(effects, Some(alarm_unix_ms), now_unix_ms),
		None => {
			if alarm_due {
				set_alarm(effects, Some(now_unix_ms), now_unix_ms);
			}
		}
	}

	// A recalled actor kept its entry, so its write already succeeded.
	match state.write_index.state.phase {
		wire::WriteIndexPhase::WriteIndexPhaseSuccess => {}
		wire::WriteIndexPhase::WriteIndexPhasePending
		| wire::WriteIndexPhase::WriteIndexPhaseFailed => {
			walk_write_index(
				&mut state.write_index,
				state.index_bucket_count,
				state.create_unix_ms,
				effects,
				None,
			);
		}
	}
}

/// Holds a sleeping actor again so the migrator can hand it back to V2. A sleeping actor has no
/// epoch an envoy may hold, so nothing else must stop first. The waiters stored for its next
/// start are told to go to V2, since no start will notify them. Its alarm stays armed, so an
/// activation that keeps it still has it; a discard cancels it with the actor. Its key stays
/// reserved and its index entry stays, as it was before migration.
fn recall_to_v2(
	state: &mut wire::ActorState,
	migration_id: u64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	now_unix_ms: i64,
) {
	state
		.start_waiters
		.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
	if !state.start_waiters.is_empty() {
		effects.execute(Effect::NotifyStartWaiters {
			waiters: std::mem::take(&mut state.start_waiters),
			kind: wire::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2,
		});
	}

	state.phase = wire::ActorPhase::MigratingFromV2(wire::MigratingFromV2 {
		migration_id,
		reserve_epoxy: None,
		alarm_due: false,
	});
}

/// Where a reservation looks for an owner of the key besides this datacenter.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyCheck {
	/// Only Epoxy. The V2 migrator's hold reserves the key of an actor V2 holds by definition,
	/// so asking V2 would always find it.
	Epoxy,
	/// Epoxy, and V2 while the actor's namespace migrates in from V2, so a create of a key V2
	/// already holds ends delegated to V2. The executor reads the migration config when it
	/// runs, so a migration configured or removed between attempts applies at once.
	EpoxyAndV2,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
	ReserveEpoxy {
		namespace: String,
		name: String,
		key: String,
		attempt: u32,
		create_unix_ms: i64,
		key_check: KeyCheck,
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
	/// Judges the running epoch's envoy by its record, as a gateway request does, and reports
	/// the epoch lost when the envoy is gone.
	CheckEnvoy {
		epoch: u32,
		allocation: wire::ActorAllocation,
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
			Effect::CheckEnvoy { .. } => "check_envoy",
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
	/// Initialization of a new incarnation, or the begin of a V2 migration, in the same
	/// transaction that records it.
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
			wire::ActorEvent::MigrateFromV2(_) => "migrate_from_v2",
		}
	}
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct State(pub wire::ActorState);

impl rivet_state_machine::StateMachineState for State {}

/// The durable runner's id for each actor timer: one timer per variant. Every cancel names the
/// current epoch's or attempt's timer, and each handler ignores a timeout whose epoch, attempt
/// or incarnation in the payload is not the current one, so arming a variant may replace a stale
/// timer of the same variant. Stored timers keep their ids, so a variant's number never changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::FromRepr)]
#[repr(u32)]
enum TimerId {
	ReserveEpoxyTimeout = 1,
	WriteIndexTimeout = 2,
	DestroyTimeout = 3,
	StartTimeout = 4,
	AlarmTimeout = 5,
	StopTimeout = 6,
	EnvoyCheckTimeout = 7,
	StopThresholdTimeout = 8,
	AllocationRetry = 9,
}

impl TimerId {
	fn of(timer: &wire::ActorTimer) -> Self {
		match timer {
			wire::ActorTimer::ReserveEpoxyTimeout(_) => Self::ReserveEpoxyTimeout,
			wire::ActorTimer::WriteIndexTimeout(_) => Self::WriteIndexTimeout,
			wire::ActorTimer::DestroyTimeout(_) => Self::DestroyTimeout,
			wire::ActorTimer::StartTimeout(_) => Self::StartTimeout,
			wire::ActorTimer::AlarmTimeout => Self::AlarmTimeout,
			wire::ActorTimer::StopTimeout(_) => Self::StopTimeout,
			wire::ActorTimer::EnvoyCheckTimeout(_) => Self::EnvoyCheckTimeout,
			wire::ActorTimer::StopThresholdTimeout(_) => Self::StopThresholdTimeout,
			wire::ActorTimer::AllocationRetry(_) => Self::AllocationRetry,
		}
	}
}
