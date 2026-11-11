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
pub(crate) const STOP_LOST_MARGIN_MS: u64 = 5 * 1000;
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
/// The backoff of a fork step's first refusal in a row. Each further refusal doubles it.
const FORK_REFUSAL_BACKOFF_BASE_MS: u64 = 100;
/// The most a fork step's refusal backoff grows to, so a fork refused for a long time still
/// retries about this often and never faster than its refusal's hint.
const FORK_REFUSAL_BACKOFF_MAX_MS: u64 = 10 * 1000;
/// The longest a refusal's own retry hint holds a fork step back. A larger hint from Depot is
/// cut to this, so one hint never parks a fork, and a destroy waiting behind it, for long.
const FORK_REFUSAL_HINT_MAX_MS: u64 = 60 * 1000;
/// The most bodies one fork replaces. A replacement follows an open that made the record of an
/// inherited source, so one is expected and a few cover a source whose record is made late; a
/// source whose record no open makes then fails the fork instead of cycling forever.
const FORK_MAX_BODY_REPLACEMENTS: u32 = 3;

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
			fork_reset,
			start_dispatched,
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
		// An inherited state is reset in the same transaction as its first event, so the stamp
		// is durable exactly when the first event's state is.
		if let Some(incarnation) = fork_reset {
			crate::metrics::ACTOR_FORK_RESETS
				.with_label_values(&[phase_label(&state.phase)])
				.inc();
			reset_for_fork(state, incarnation, effects);
		}

		// Legacy: the backfill's marks and their deadline, and its destroys, which are fenced by
		// its mover token and become an ordinary destroy once accepted. Removed in 3.1 with the
		// backfill.
		let event = match event {
			RuntimeEvent::Input(input) => {
				match legacy_move_input(
					state,
					input,
					effects,
					prepared_allocation.clone(),
					start_threshold_ms,
					entropy.clone(),
					now_unix_ms,
				) {
					Some(input) => RuntimeEvent::Input(input),
					None => return,
				}
			}
			RuntimeEvent::Timer(wire::ActorTimer::LegacyMovingTimeout(timeout)) => {
				if legacy_moving_mover(state) == Some(timeout.mover) {
					legacy_end_mark(
						state,
						effects,
						timeout.mover,
						prepared_allocation,
						start_threshold_ms,
						entropy,
						now_unix_ms,
					);
				}
				return;
			}
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
						start_dispatched,
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
						start_dispatched,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					alarm_fired(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						start_dispatched,
						now_unix_ms,
					);
				}
				RuntimeEvent::Input(wire::ActorEvent::ReserveEpoxyState(effect_state)) => {
					let key_check = create_key_check(init.legacy_move.is_some());
					walk_reserve_epoxy(
						&mut init.reserve_epoxy,
						state.create_unix_ms,
						key_check,
						effects,
						Some(effect_state),
						None,
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
					let key_check = create_key_check(init.legacy_move.is_some());
					walk_reserve_epoxy(
						&mut init.reserve_epoxy,
						state.create_unix_ms,
						key_check,
						effects,
						Some(wire::ReserveEpoxyState {
							phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
							attempt: timeout.attempt,
							create_unix_ms: timeout.create_unix_ms,
						}),
						None,
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
				RuntimeEvent::Input(wire::ActorEvent::ForkDatabaseState(result)) => {
					let forked = match &mut init.fork_database {
						Some(fork) => fork_database_result(
							fork,
							state.create_unix_ms,
							effects,
							result,
							now_unix_ms,
						),
						None => false,
					};
					let reservation = init.reserve_epoxy.state.phase.clone();
					let allocation = init.allocation.clone();
					if forked {
						notify_forked(state, effects, now_unix_ms);
					}
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
				RuntimeEvent::Input(wire::ActorEvent::ForkDatabaseRefused(refused)) => {
					if let Some(fork) = &mut init.fork_database {
						let mut rng = entropy.rng();
						fork_database_refused(
							fork,
							state.create_unix_ms,
							effects,
							refused,
							&mut rng,
						);
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::ForkDatabaseTimeout(timeout)) => {
					if let Some(fork) = &mut init.fork_database {
						fork_database_timeout(fork, state.create_unix_ms, effects, timeout);
					}
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
				RuntimeEvent::Timer(wire::ActorTimer::ForkDatabaseRetry(retry)) => {
					if let Some(fork) = &mut init.fork_database {
						fork_database_retry(fork, state.create_unix_ms, effects, retry);
					}
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
					// A fork that has sent nothing to its destination yet is abandoned now
					// rather than run to its end first.
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
				RuntimeEvent::Input(wire::ActorEvent::LegacyMoveConfirm(confirm)) => {
					legacy_confirm_move(
						state,
						effects,
						confirm,
						prepared_allocation,
						start_threshold_ms,
						now_unix_ms,
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
						start_dispatched,
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
						start_dispatched,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					alarm_fired(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						start_dispatched,
						now_unix_ms,
					);
				}
				// The start command did not reach the envoy, or the envoy never reported
				// running. Either way the envoy may hold the epoch, so it is only stopped, like
				// a lost report: the epoch ends on the stopped report, once the envoy is
				// proven gone, or at the stop deadline, so the next start never runs beside
				// this one. The stop ends in a sleep, so the alarm is armed again: an alarm
				// whose start failed, as on an envoy whose record still says connected though
				// its node is gone, is retried instead of lost.
				RuntimeEvent::Input(wire::ActorEvent::StartFailed(res)) => {
					if res.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						effects.cancel_timer(wire::ActorTimer::StartTimeout(wire::StartTimeout {
							epoch: state.epoch,
						}));
						stop_actor(state, effects, allocation, stop_deadline);
						if !pending_destroy(state) {
							arm_failed_start_alarm_retry(effects, "start_failed", entropy);
						}
					}
				}
				RuntimeEvent::Timer(wire::ActorTimer::StartTimeout(timeout)) => {
					if timeout.epoch == state.epoch {
						let allocation = phase.allocation.clone();
						stop_actor(state, effects, allocation, stop_deadline);
						if !pending_destroy(state) {
							arm_failed_start_alarm_retry(effects, "start_timeout", entropy);
						}
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
						start_dispatched,
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
						start_dispatched,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				// The alarm fires on the envoy itself. The engine's timer only checks that the
				// envoy is still there: an envoy gone past its lost threshold has stopped the
				// epoch, which a request would find out through its failed open. An alarm has
				// no request, so the check stands in for one. Legacy: an actor the backfill is
				// moving also keeps the alarm in its mark, since a lost envoy's reallocation starts
				// nothing under the mark and the move carries only the mark's alarm. Removed in 3.1
				// with the backfill.
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					let allocation = phase.allocation.clone();
					legacy_defer_alarm(state, now_unix_ms);
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
						start_dispatched,
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
						start_dispatched,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					alarm_fired(
						state,
						effects,
						prepared_allocation,
						start_threshold_ms,
						start_dispatched,
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
						start_dispatched,
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
						start_dispatched,
						start_waiters_capacity,
						now_unix_ms,
					);
				}
				// With no envoy to start on, as right after the engine starts and before its
				// envoys reconnect, the alarm is armed again so the wake is not lost. Legacy: an
				// actor the backfill is moving selects no envoy, and `alarm_fired` keeps the
				// alarm in its mark for the move to carry instead. Removed in 3.1 with the backfill.
				RuntimeEvent::Timer(wire::ActorTimer::AlarmTimeout) => {
					if prepared_allocation.is_some() || legacy_moving(state) {
						alarm_fired(
							state,
							effects,
							prepared_allocation,
							start_threshold_ms,
							start_dispatched,
							now_unix_ms,
						);
					} else {
						rearm_alarm(state, effects, entropy);
					}
				}
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
						// Legacy: an actor the backfill is moving selects no envoy, so its retry
						// arms nothing and the mark keeps the reallocation as a due alarm, which
						// the move carries or the mark's end fires. This also covers a mark an
						// older binary applied without the deferral. Removed in 3.1 with the
						// backfill.
						if prepared_allocation.is_some() {
							allocate_actor(
								state,
								effects,
								prepared_allocation,
								Dispatch::Durable,
								start_threshold_ms,
								now_unix_ms,
							);
						} else if !legacy_defer_alarm(state, now_unix_ms) {
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
						legacy_keeps_index_entry(&state.legacy_move),
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
						legacy_keeps_index_entry(&state.legacy_move),
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
							None,
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
							None,
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

	fn fork_reset(state: &Self::State) -> Option<[u8; 16]> {
		state.0.fork_reset
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
/// the event causes, as `needs_allocation` names them, the incarnation an inherited state must be
/// reset for first, and the configuration the transition reads, since the core never reads the
/// config itself.
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
	/// Set when the state volume was made by a group fork and the state was never reset for
	/// its incarnation: the transition resets the inherited state before the event.
	pub fork_reset: Option<[u8; 16]>,
	/// This opening of the state volume already dispatched the start command of the state's
	/// epoch and command index, so a wake of the starting actor does not send it again: the
	/// resend covers only a start whose dispatch a revoked earlier opening may have lost.
	pub start_dispatched: bool,
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
			fork_reset: None,
			start_dispatched: false,
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
/// key when another datacenter holds it. A forked actor whose key is reserved first runs its
/// database fork: the first call starts it, the start waits until it is `Forked`, and a failed
/// fork destroys the actor and records why in `database_origin`. Called from the create and after
/// each reservation result or timeout, with the `Initializing` phase's reservation state and the
/// allocation it holds; it does nothing while the reservation is pending. `dispatch` says where
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
			let (fork, legacy_move_pending) =
				if let wire::ActorPhase::Initializing(init) = &state.phase {
					let fork = init
						.fork_database
						.as_ref()
						.map(|fork| (fork.state.phase.clone(), fork.state.attempt));
					(fork, init.legacy_move.is_some())
				} else {
					(None, false)
				};
			match fork {
				// Legacy: a moved actor stays here until the backfill confirms its move, so it
				// never starts before the old actor's destroy ended its epoch. A pending destroy
				// still ends it. Removed in 3.1 with the backfill.
				None | Some((wire::ForkDatabasePhase::ForkDatabasePhaseForked, _))
					if legacy_move_pending
						&& state.pending_action != wire::PendingAction::PendingDestroy => {}
				None | Some((wire::ForkDatabasePhase::ForkDatabasePhaseForked, _)) => {
					allocate_actor(
						state,
						effects,
						allocation,
						dispatch,
						start_threshold_ms,
						now_unix_ms,
					);
				}
				// A destroy requested while the fork is still preparing ends the actor at once:
				// a prepare sends nothing to the destination, so no Depot job can publish later,
				// and a prepare still in flight reports to an incarnation that is gone.
				Some((wire::ForkDatabasePhase::ForkDatabasePhasePreparing, _))
					if state.pending_action == wire::PendingAction::PendingDestroy =>
				{
					if let wire::ActorPhase::Initializing(init) = &state.phase
						&& let Some(fork) = &init.fork_database
					{
						effects.cancel_timer(fork_database_timer(fork, state.create_unix_ms));
						effects.cancel_timer(fork_database_retry_timer(fork, state.create_unix_ms));
					}
					destroy_actor(state, effects, None, now_unix_ms);
				}
				// The key is reserved and nothing was copied yet: the fork starts now, so a create
				// that loses its key never copies the source's database.
				Some((wire::ForkDatabasePhase::ForkDatabasePhasePreparing, 0)) => {
					let create_unix_ms = state.create_unix_ms;
					if let wire::ActorPhase::Initializing(init) = &mut state.phase
						&& let Some(fork) = &mut init.fork_database
					{
						fork_database_attempt(
							fork,
							create_unix_ms,
							effects,
							AttemptStart::Progress,
						);
					}
				}
				// A prepared body may already run as a Depot job, so even a pending destroy waits
				// for its definite outcome: destroying first would leave a volume the job
				// publishes afterwards.
				Some((
					wire::ForkDatabasePhase::ForkDatabasePhasePreparing
					| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
					_,
				)) => {}
				Some((wire::ForkDatabasePhase::ForkDatabasePhaseFailed(failed), _)) => {
					state.database_origin =
						wire::ActorDatabaseOrigin::DatabaseForkFailed(wire::DatabaseForkFailed {
							failure: failed.failure,
						});
					destroy_actor(state, effects, None, now_unix_ms);
				}
			}
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
/// the allocation is committed only now. A create with `fork_from` copies the source's database
/// into the new incarnation's database volume before the first start. Legacy: a create with
/// `legacy_move` is the backfill moving an unprefixed actor here; it keeps the old creation time,
/// copies the old database the same way, carries the old alarm, and is held in `Initializing`
/// until `legacy_confirm_move`, with no fork recorded, so the runtime never runs its fork hook.
/// Removed in 3.1 with the backfill.
fn initialize(
	state: &mut wire::ActorState,
	init: wire::ActorCreateData,
	waiter: Option<wire::StartWaiter>,
	allocation: Option<wire::ActorAllocation>,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	// Legacy: a backfill move keeps the old incarnation's creation time, so the actor lists and
	// sorts where it did. Removed in 3.1 with the backfill.
	let created_at = match &init.legacy_move {
		Some(legacy_move) => legacy_move.create_unix_ms,
		None => now_unix_ms,
	};
	let create_unix_ms = created_at.max(state.create_unix_ms.saturating_add(1));
	state.create_unix_ms = create_unix_ms;
	state.storage_origin = wire::StorageOrigin::Created;
	state.command_index = 0;
	state.pending_action = wire::PendingAction::PendingNone;
	state.start_waiters = waiter.into_iter().collect();
	state.legacy_move = None;
	state.legacy_namespace = init.legacy_namespace.clone();

	state.write_index = wire::WriteIndexEffectData {
		name: init.name.clone(),
		create_unix_ms,
		state: wire::WriteIndexState {
			phase: wire::WriteIndexPhase::WriteIndexPhasePending,
			attempt: 0,
			create_unix_ms,
		},
	};
	// A fork copies the source's database, and so does a backfill move of an actor that opened
	// one; the move runs the same fork step but is not recorded as a fork, so no fork hook runs.
	let database_source = init.fork_from.clone().or_else(|| {
		init.legacy_move
			.as_ref()
			.and_then(|legacy_move| legacy_move.database.clone())
	});
	state.database_origin = match &database_source {
		Some(_) => wire::ActorDatabaseOrigin::DatabaseForked,
		None => wire::ActorDatabaseOrigin::DatabaseEmpty,
	};
	let fork_database = database_source.map(|source| wire::ForkDatabaseEffectData {
		source,
		state: wire::ForkDatabaseState {
			phase: wire::ForkDatabasePhase::ForkDatabasePhasePreparing,
			attempt: 0,
			create_unix_ms,
		},
		refusals: 0,
		replacements: 0,
		tail_unproven_refusals: 0,
	});
	let mut init_phase = wire::Initializing {
		reserve_epoxy: wire::ReserveEpoxyEffectData {
			name: init.name.clone(),
			key: init.key.clone(),
			state: wire::ReserveEpoxyState {
				phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
				attempt: 0,
				create_unix_ms,
			},
		},
		allocation,
		fork_database,
		legacy_move: init
			.legacy_move
			.as_ref()
			.map(|legacy_move| wire::LegacyMoveHold {
				alarm_unix_ms: legacy_move.alarm_unix_ms,
				mover: legacy_move.mover,
			}),
	};

	effects.write(WriteEffect::WriteData {
		data: wire::ActorData {
			actor_id: init.actor_id,
			pool_name: init.pool_name,
			name: init.name,
			key: init.key,
			input: init.input,
			create_unix_ms,
			legacy_namespace: init.legacy_namespace.clone(),
			forked_from: init.fork_from,
			legacy_moved_from: init.legacy_move.map(|legacy_move| legacy_move.moved_from),
		},
	});
	// Legacy: the first attempt runs before this transition's data record commits, so it carries
	// the legacy namespace an unprefixed create names instead of reading the record. Removed in
	// 3.1 with the backfill.
	walk_reserve_epoxy(
		&mut init_phase.reserve_epoxy,
		create_unix_ms,
		create_key_check(init_phase.legacy_move.is_some()),
		effects,
		None,
		init.legacy_namespace
			.as_ref()
			.map(|namespace| namespace.to_string()),
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

/// The fork chain of `source` as a fork of it carries it: every individual actor fork behind
/// the source, oldest first, ending with the fork that made the source. Empty for a source that
/// was never forked. The chain comes from the source's record, so it holds even when the
/// source's copied database has not recorded it yet.
///
/// A fork adds one more link, and each individual actor fork adds at least one link to Depot's
/// fork chain of the copy's database volume, so a source whose chain is already
/// `MAX_FORK_CHAIN_DEPTH` long is refused with `actor.fork_too_deep` before anything is created.
/// This also bounds every chain a record carries.
pub fn source_fork_chain(source: &wire::ActorData) -> Result<Vec<wire::ActorForkLink>> {
	let Some(forked_from) = &source.forked_from else {
		return Ok(Vec::new());
	};

	let mut chain = forked_from.chain.clone();
	chain.push(wire::ActorForkLink {
		actor_id: forked_from.actor_id.clone(),
		key: forked_from.key.clone(),
		forked_at: source.create_unix_ms,
	});
	if chain.len() >= depot_fs_core::MAX_FORK_CHAIN_DEPTH {
		return Err(crate::validation::ActorForkTooDeep {
			max_depth: depot_fs_core::MAX_FORK_CHAIN_DEPTH,
		}
		.build()
		.into());
	}
	Ok(chain)
}

/// Legacy: the backfill confirmed the move of the actor initializing here: it proved its claim
/// on the old database held through the database fork, and the old actor's destroy committed
/// and ended its epoch, so the old generation no longer runs.
/// Accepted only for the current incarnation and the hold's own mover, while the move is held
/// and its key is reserved and its database copied; anything else is stale or early and changes
/// nothing. The epoch is raised above the old actor's, so no generation of this actor coincides
/// with one of the old actor's on an envoy, which keys actors by id and generation: a random-id
/// actor keeps its id through the move. The carried alarm is armed and initialization ends,
/// starting the actor on the envoy selected for this transition, which replaces one a request
/// stored during the move and may have gone since, or leaving it asleep. Removed in 3.1 with the
/// backfill.
fn legacy_confirm_move(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	confirm: wire::LegacyMoveConfirm,
	allocation: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	now_unix_ms: i64,
) {
	if confirm.create_unix_ms != state.create_unix_ms {
		return;
	}
	let wire::ActorPhase::Initializing(init) = &mut state.phase else {
		return;
	};
	let reserved = matches!(
		init.reserve_epoxy.state.phase,
		wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved
	);
	let copied = match &init.fork_database {
		None => true,
		Some(fork) => matches!(
			fork.state.phase,
			wire::ForkDatabasePhase::ForkDatabasePhaseForked
		),
	};
	if !reserved || !copied {
		return;
	}
	if init
		.legacy_move
		.as_ref()
		.is_none_or(|hold| hold.mover != confirm.mover)
	{
		return;
	}
	let Some(hold) = init.legacy_move.take() else {
		return;
	};
	if allocation.is_some() {
		init.allocation = allocation;
	}
	let reservation = init.reserve_epoxy.state.phase.clone();
	let allocation = init.allocation.clone();

	state.epoch = state.epoch.max(confirm.old_epoch).saturating_add(1);
	if let Some(alarm_unix_ms) = hold.alarm_unix_ms {
		set_alarm(effects, Some(alarm_unix_ms), now_unix_ms);
	}
	// The epoch changes in this transition, so the start waits for its commit.
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

/// Where a create's reservation looks for another owner of the key. Legacy: a backfill move
/// (`legacy_move_pending`, the copy still held for its confirm) asks only Epoxy. The actor's V3
/// ownership of the key was settled when it was created, so asking V2 again could only delegate
/// a live V3 actor away, ending its copy `DelegatedToV2` with no data. Every other create asks V2
/// too. Removed in 3.1 with the backfill.
fn create_key_check(legacy_move_pending: bool) -> KeyCheck {
	if legacy_move_pending {
		KeyCheck::Epoxy
	} else {
		KeyCheck::EpoxyAndV2
	}
}

/// The `walk_*` helpers advance one effect's attempt state. `new_state` is the executor's
/// result or the attempt's timeout; it is ignored when it names another incarnation by
/// `create_unix_ms` or an attempt older than the current one. Without it, the first attempt
/// is started. `legacy_namespace` is the create's legacy namespace, passed only for the first
/// attempt, which runs before the record naming it commits.
fn walk_reserve_epoxy(
	effect_data: &mut wire::ReserveEpoxyEffectData,
	create_unix_ms: i64,
	key_check: KeyCheck,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	new_state: Option<wire::ReserveEpoxyState>,
	legacy_namespace: Option<String>,
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
		name: effect_data.name.to_string(),
		key: key.to_string(),
		attempt: effect_data.state.attempt,
		create_unix_ms,
		key_check,
		legacy_namespace,
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

/// Applies an executor result of the database fork: `Prepared` stores the body and sends it,
/// `Forked` and `Failed` settle the fork for `try_complete_initialization`. A `Prepared` result
/// at `Prepared` replaces the body: the executor reports one only after the old body's job
/// failed for good on a source that had no record in this group, which its open then made, so
/// nothing of the old body can still publish; once `FORK_MAX_BODY_REPLACEMENTS` bodies were
/// replaced, a further replacement fails the fork instead. A result for another incarnation, an attempt
/// other than the current one, or a step the fork is no longer at changes nothing. Every
/// accepted result ends the run of refusals. Returns whether this result settled the fork as
/// `Forked`.
fn fork_database_result(
	fork: &mut wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	result: wire::ForkDatabaseState,
	now_unix_ms: i64,
) -> bool {
	if result.create_unix_ms != create_unix_ms || result.attempt != fork.state.attempt {
		return false;
	}

	match (&fork.state.phase, result.phase) {
		// Replacements have their own cap, so a source whose record its open never makes cannot
		// cycle through opens and prepares forever, and timeouts never use it up.
		(
			wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
			wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
		) if fork.replacements >= FORK_MAX_BODY_REPLACEMENTS => {
			effects.cancel_timer(fork_database_timer(fork, create_unix_ms));
			fork.state.phase =
				wire::ForkDatabasePhase::ForkDatabasePhaseFailed(wire::ForkDatabasePhaseFailed {
					failure: wire::ForkDatabaseFailure::MaxAttemptsReached,
				});
			fork.refusals = 0;
			fork.tail_unproven_refusals = 0;
			observe_fork_step(effects, ForkStepResult::Failed, None);
		}
		(
			wire::ForkDatabasePhase::ForkDatabasePhasePreparing
			| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
			wire::ForkDatabasePhase::ForkDatabasePhasePrepared(prepared),
		) => {
			// The body is persisted by this transition and sent only after it commits, so a
			// crash never loses the operation id of a fork that may already be running.
			match &fork.state.phase {
				wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_) => {
					fork.replacements = fork.replacements.saturating_add(1);
				}
				wire::ForkDatabasePhase::ForkDatabasePhasePreparing
				| wire::ForkDatabasePhase::ForkDatabasePhaseForked
				| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_) => {}
			}
			fork.state.phase = wire::ForkDatabasePhase::ForkDatabasePhasePrepared(prepared);
			fork.refusals = 0;
			fork.tail_unproven_refusals = 0;
			fork_database_attempt(fork, create_unix_ms, effects, AttemptStart::Progress);
			observe_fork_step(effects, ForkStepResult::Prepared, None);
		}
		(
			wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
			wire::ForkDatabasePhase::ForkDatabasePhaseForked,
		) => {
			effects.cancel_timer(fork_database_timer(fork, create_unix_ms));
			fork.state.phase = wire::ForkDatabasePhase::ForkDatabasePhaseForked;
			fork.refusals = 0;
			fork.tail_unproven_refusals = 0;
			let elapsed_ms = now_unix_ms.saturating_sub(create_unix_ms).max(0);
			observe_fork_step(effects, ForkStepResult::Forked, Some(elapsed_ms));
			return true;
		}
		(
			wire::ForkDatabasePhase::ForkDatabasePhasePreparing
			| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
			wire::ForkDatabasePhase::ForkDatabasePhaseFailed(failed),
		) => {
			effects.cancel_timer(fork_database_timer(fork, create_unix_ms));
			fork.state.phase = wire::ForkDatabasePhase::ForkDatabasePhaseFailed(failed);
			fork.refusals = 0;
			fork.tail_unproven_refusals = 0;
			observe_fork_step(effects, ForkStepResult::Failed, None);
		}
		(
			wire::ForkDatabasePhase::ForkDatabasePhaseForked
			| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_),
			wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_),
		)
		| (
			wire::ForkDatabasePhase::ForkDatabasePhasePreparing,
			wire::ForkDatabasePhase::ForkDatabasePhaseForked,
		)
		| (
			wire::ForkDatabasePhase::ForkDatabasePhaseForked
			| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_),
			wire::ForkDatabasePhase::ForkDatabasePhaseForked
			| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_),
		)
		| (
			wire::ForkDatabasePhase::ForkDatabasePhasePreparing
			| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_)
			| wire::ForkDatabasePhase::ForkDatabasePhaseForked
			| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_),
			wire::ForkDatabasePhase::ForkDatabasePhasePreparing,
		) => {}
	}
	false
}

/// Tells the stored start waiters that the actor's database fork finished, once that outcome
/// commits, so a fork create answers as soon as the copy finished instead of waiting for a
/// start that needs a node. The waiters stay stored for the start notices that follow. A failed
/// fork needs no notice of its own: the destroy that follows it notifies them.
fn notify_forked(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	now_unix_ms: i64,
) {
	state
		.start_waiters
		.retain(|waiter| waiter.expires_unix_ms > now_unix_ms);
	if !state.start_waiters.is_empty() {
		effects.execute(Effect::NotifyStartWaiters {
			waiters: state.start_waiters.clone(),
			kind: wire::ActorStartNoticeKind::ActorStartNoticeForked,
		});
	}
}

/// Depot refused the current attempt's step for now. The body is kept, since the refusal does
/// not prove nothing ran. A refusal is not a failure, so it consumes no attempt: the attempt's
/// timeout is replaced by a retry, which sends the same step again under the same attempt. The
/// retry waits at least the refusal's hint and a backoff that doubles with each refusal in a
/// row up to a cap, with jitter drawn from the event's entropy in `rng`, so forks refused
/// together do not retry in lockstep. The refusal is the
/// attempt's only answer, so no other result of it can arrive after the resend.
///
/// A refusal because the source's tail is unproven is counted on its own, and any other
/// refusal ends that run. From the first one, the resent step carries the count, and the
/// executor opens the source first if the source actor sleeps, which proves the tail. No
/// refusal fails the fork or destroys the actor, since the job may still publish.
fn fork_database_refused(
	fork: &mut wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	refused: wire::ForkDatabaseRefused,
	rng: &mut rivet_env::rand::LocalRng,
) {
	if refused.create_unix_ms != create_unix_ms || refused.attempt != fork.state.attempt {
		return;
	}
	if !in_flight(&fork.state.phase) {
		return;
	}

	fork.refusals = fork.refusals.saturating_add(1);
	fork.tail_unproven_refusals = if refused.tail_unproven {
		fork.tail_unproven_refusals.saturating_add(1)
	} else {
		0
	};
	let delay_ms = fork_refusal_delay_ms(fork.refusals, refused.retry_after_ms, rng);

	effects.cancel_timer(fork_database_timer(fork, create_unix_ms));
	effects.arm_timer_after(fork_database_retry_timer(fork, create_unix_ms), delay_ms);
	observe_fork_step(effects, ForkStepResult::Refused, None);
}

/// Counts a fork step this transition accepted, after the transition publishes.
fn observe_fork_step(
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	result: ForkStepResult,
	elapsed_ms: Option<i64>,
) {
	effects.nondurable_execute(Effect::ObserveForkStep { result, elapsed_ms });
}

/// The wait before resending a step refused `refusals` times in a row: the backoff doubles from
/// `FORK_REFUSAL_BACKOFF_BASE_MS` up to `FORK_REFUSAL_BACKOFF_MAX_MS`, a draw from `rng` places
/// it uniformly in the backoff's upper half, and it is never shorter than the refusal's hint,
/// which counts only up to `FORK_REFUSAL_HINT_MAX_MS`.
fn fork_refusal_delay_ms(
	refusals: u32,
	retry_after_ms: u32,
	rng: &mut rivet_env::rand::LocalRng,
) -> u64 {
	let doublings = refusals.saturating_sub(1);
	let multiplier = 1u64.checked_shl(doublings).unwrap_or(u64::MAX);
	let backoff_ms = FORK_REFUSAL_BACKOFF_BASE_MS
		.saturating_mul(multiplier)
		.min(FORK_REFUSAL_BACKOFF_MAX_MS);

	let half_ms = backoff_ms / 2;
	let jittered_ms = half_ms + rng.gen_range(0..half_ms + 1);

	jittered_ms.max(u64::from(retry_after_ms).min(FORK_REFUSAL_HINT_MAX_MS))
}

/// The refusal's backoff passed: the current step is sent again under the same attempt, with the
/// attempt's timeout armed anew.
fn fork_database_retry(
	fork: &mut wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	retry: wire::ForkDatabaseRetry,
) {
	if retry.create_unix_ms != create_unix_ms || retry.attempt != fork.state.attempt {
		return;
	}
	if !in_flight(&fork.state.phase) {
		return;
	}

	let backoff_ms = executor::effect_backoff(fork.state.attempt);
	effects.execute(Effect::ForkDatabase {
		data: fork.clone(),
		backoff_ms,
	});
	effects.arm_timer_after(
		fork_database_timer(fork, create_unix_ms),
		EFFECT_TIMEOUT_MS + backoff_ms,
	);
}

/// The current attempt went unanswered: the step it was at is sent again under the next
/// attempt. A prepare is retried from scratch until the attempts run out, since nothing was
/// sent to the destination. A prepared body is resent unchanged with no cap, because its job
/// may run and publish later: the fork waits for the job's definite outcome, published or a
/// permanent failure, and never gives up into a destroy that would leave its volume behind.
fn fork_database_timeout(
	fork: &mut wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	timeout: wire::ForkDatabaseTimeout,
) {
	if timeout.create_unix_ms != create_unix_ms || timeout.attempt != fork.state.attempt {
		return;
	}
	if !in_flight(&fork.state.phase) {
		return;
	}

	let preparing = match &fork.state.phase {
		wire::ForkDatabasePhase::ForkDatabasePhasePreparing => true,
		wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_)
		| wire::ForkDatabasePhase::ForkDatabasePhaseForked
		| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_) => false,
	};
	if preparing && fork.state.attempt >= EFFECT_MAX_ATTEMPTS {
		fork.state.phase =
			wire::ForkDatabasePhase::ForkDatabasePhaseFailed(wire::ForkDatabasePhaseFailed {
				failure: wire::ForkDatabaseFailure::MaxAttemptsReached,
			});
		observe_fork_step(effects, ForkStepResult::Failed, None);
		return;
	}
	fork_database_attempt(fork, create_unix_ms, effects, AttemptStart::Retry);
}

/// Why a fork database attempt starts, which decides whether its step waits a backoff.
#[derive(Clone, Copy)]
enum AttemptStart {
	/// The fork's first step, or the step a successful one leads to: sent at once.
	Progress,
	/// The previous attempt went unanswered: the step is sent again after the effect backoff.
	Retry,
}

/// Whether the fork still has a step to send.
fn in_flight(phase: &wire::ForkDatabasePhase) -> bool {
	match phase {
		wire::ForkDatabasePhase::ForkDatabasePhasePreparing
		| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_) => true,
		wire::ForkDatabasePhase::ForkDatabasePhaseForked
		| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_) => false,
	}
}

/// Starts the next attempt of the fork's current step: the previous attempt's timer is
/// cancelled, the step is executed after this transition commits, and the new attempt's timer
/// is armed. Only a retry waits the effect backoff before its step; progress sends it at once.
fn fork_database_attempt(
	fork: &mut wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	start: AttemptStart,
) {
	effects.cancel_timer(fork_database_timer(fork, create_unix_ms));
	effects.cancel_timer(fork_database_retry_timer(fork, create_unix_ms));

	fork.state.attempt = fork.state.attempt.saturating_add(1);
	fork.state.create_unix_ms = create_unix_ms;

	let backoff_ms = match start {
		AttemptStart::Progress => 0,
		AttemptStart::Retry => executor::effect_backoff(fork.state.attempt),
	};
	effects.execute(Effect::ForkDatabase {
		data: fork.clone(),
		backoff_ms,
	});

	effects.arm_timer_after(
		fork_database_timer(fork, create_unix_ms),
		EFFECT_TIMEOUT_MS + backoff_ms,
	);
}

fn fork_database_timer(
	fork: &wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
) -> wire::ActorTimer {
	wire::ActorTimer::ForkDatabaseTimeout(wire::ForkDatabaseTimeout {
		attempt: fork.state.attempt,
		create_unix_ms,
	})
}

fn fork_database_retry_timer(
	fork: &wire::ForkDatabaseEffectData,
	create_unix_ms: i64,
) -> wire::ActorTimer {
	wire::ActorTimer::ForkDatabaseRetry(wire::ForkDatabaseRetry {
		attempt: fork.state.attempt,
		create_unix_ms,
	})
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
	legacy_keep_index_entry: bool,
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
		legacy_keep_index_entry,
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
/// the dispatch, unless `start_dispatched` says this opening of its state volume already sent
/// it: a second wake during the same start, such as a gateway's read finding it starting, then
/// sends nothing. A running actor needs nothing from a try start, and a destroyed one is not
/// revived here.
fn wake(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	hint: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	start_dispatched: bool,
	now_unix_ms: i64,
) {
	// Legacy: an actor the backfill is moving is not woken; the backfill's sleep stays pending.
	// Removed in 3.1 with the backfill.
	if legacy_moving(state) {
		return;
	}
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
			// An actor initializing with no envoy chosen, such as one the backfill moves, takes
			// the envoy this wake selected, so it starts once its initialization ends instead of
			// leaving the waiter asleep. It only stores the envoy: a moved actor still waits for
			// its move's confirm before it starts.
			if let wire::ActorPhase::Initializing(init) = &mut state.phase
				&& init.allocation.is_none()
				&& hint.is_some()
			{
				init.allocation = hint;
				return;
			}
			if let wire::ActorPhase::Starting(phase) = &state.phase
				&& !start_dispatched
			{
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

/// The actor's alarm fired: it wakes the actor. Legacy: an actor the backfill is moving keeps the
/// alarm in its mark instead, so the move carries it to the moved copy and the old actor never
/// wakes for it, and it fires when the mark ends without a destroy. Removed in 3.1 with the
/// backfill.
fn alarm_fired(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	hint: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	start_dispatched: bool,
	now_unix_ms: i64,
) {
	if legacy_defer_alarm(state, now_unix_ms) {
		return;
	}
	wake(
		state,
		effects,
		hint,
		start_threshold_ms,
		start_dispatched,
		now_unix_ms,
	);
}

/// Legacy: keeps a due alarm in the actor's moving mark at `now_unix_ms`, unless the mark already
/// keeps an earlier one, and answers whether the backfill is moving the actor. The move carries
/// the kept alarm into the moved copy, and the mark's end without a move fires it. Besides a
/// fired alarm, a pending reallocation is kept this way: the moving actor starts nothing, so its
/// reallocation would otherwise wait on a retry timer the move does not carry. A deferral at the
/// fire time is a wake, not a user alarm: the started runtime runs its overdue schedules and
/// pushes its next alarm itself. The legacy behavior armed no deferral. Removed in 3.1 with the
/// backfill.
fn legacy_defer_alarm(state: &mut wire::ActorState, now_unix_ms: i64) -> bool {
	match &mut state.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => {
			moving.deferred_alarm_unix_ms.get_or_insert(now_unix_ms);
			true
		}
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => false,
	}
}

/// Legacy: the alarm the backfill carries into the moved copy of an actor in `state` whose
/// timer row arms `armed_unix_ms`: the earlier of that and an alarm that came due while the
/// actor was marked moving, which `alarm_fired` keeps in the mark instead of arming. Removed in
/// 3.1 with the backfill.
pub(crate) fn legacy_carried_alarm(
	state: &wire::ActorState,
	armed_unix_ms: Option<i64>,
) -> Option<i64> {
	let deferred_unix_ms = match &state.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => moving.deferred_alarm_unix_ms,
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => None,
	};
	match (deferred_unix_ms, armed_unix_ms) {
		(Some(deferred), Some(armed)) => Some(deferred.min(armed)),
		(deferred, armed) => deferred.or(armed),
	}
}

/// Legacy: whether the backfill holds this actor's moving mark, so nothing starts it. Removed in
/// 3.1 with the backfill.
pub(crate) fn legacy_moving(state: &wire::ActorState) -> bool {
	matches!(
		state.legacy_move,
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(_))
	)
}

/// Whether the actor is a moved copy held for its move's confirm.
fn legacy_held(state: &wire::ActorState) -> Option<&wire::LegacyMoveHold> {
	match &state.phase {
		wire::ActorPhase::Initializing(init) => init.legacy_move.as_ref(),
		wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Running(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed
		| wire::ActorPhase::MigratingFromV2(_)
		| wire::ActorPhase::DelegatedToV2 => None,
	}
}

/// Legacy: applies the backfill's mark events and fences its destroys by mover token, before the
/// phase's own handling, and answers the event the phase then handles, if any. A page marks an
/// actor before it claims its database, and each later write of the move names the same mover,
/// so a page that outlived its turn, such as one still running in a process that lost the
/// namespace's volume, is refused at the actor: a mark of a higher mover replaces its mark, the
/// actor refuses marks below the highest mover it has seen even after that mark ended, and its
/// destroy or abandon finds the mark gone or the copy no longer its to abandon. An ordinary
/// `Destroy` drops a moving mark, since the actor is then destroyed, not moved, and records a
/// request's destroy of a held copy, so the backfill destroys the old actor too. Removed in 3.1
/// with the backfill.
fn legacy_move_input(
	state: &mut wire::ActorState,
	input: wire::ActorEvent,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	allocation: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	entropy: Entropy,
	now_unix_ms: i64,
) -> Option<wire::ActorEvent> {
	let ended = matches!(
		state.phase,
		wire::ActorPhase::Destroying(_) | wire::ActorPhase::Destroyed
	);
	let moving_mover = legacy_moving_mover(state);
	match input {
		wire::ActorEvent::LegacyMarkMoving(mark) => {
			// The backfill never moves an actor the V2 migrator holds or whose key is delegated
			// to V2: refusing the mark makes the move skip it.
			if matches!(
				state.phase,
				wire::ActorPhase::MigratingFromV2(_) | wire::ActorPhase::DelegatedToV2
			) {
				return None;
			}
			// The mark is a high-water mark: a mover below the highest one seen, held or
			// cleared, belongs to a page that outlived its turn.
			let (seen_mover, marked) = match &state.legacy_move {
				Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => {
					(Some(moving.mover), false)
				}
				Some(wire::LegacyMoveMark::LegacyMoveMarkCleared(cleared)) => {
					(Some(cleared.mover), false)
				}
				Some(
					wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
					| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed,
				) => (None, true),
				None => (None, false),
			};
			let superseded = seen_mover.is_some_and(|mover| mover > mark.mover);
			if ended || superseded || marked {
				return None;
			}
			let deferred_alarm_unix_ms = match state.legacy_move.take() {
				Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(previous)) => {
					effects.cancel_timer(wire::ActorTimer::LegacyMovingTimeout(
						wire::LegacyMovingTimeout {
							mover: previous.mover,
						},
					));
					previous.deferred_alarm_unix_ms
				}
				Some(
					wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
					| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
					| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
				)
				| None => None,
			};
			state.legacy_move = Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(
				wire::LegacyMoveMarkMoving {
					mover: mark.mover,
					deferred_alarm_unix_ms,
				},
			));
			// A reallocation pending when the mark lands, from a lost envoy or an alarm during
			// a sleep stop, would start nothing under the mark, so the mark keeps it as a due
			// alarm the move carries.
			let stopping_or_sleeping = matches!(
				state.phase,
				wire::ActorPhase::Stopping(_) | wire::ActorPhase::Sleeping(_)
			);
			if stopping_or_sleeping
				&& state.pending_action == wire::PendingAction::PendingReallocate
			{
				legacy_defer_alarm(state, now_unix_ms);
			}
			effects.arm_timer_after(
				wire::ActorTimer::LegacyMovingTimeout(wire::LegacyMovingTimeout {
					mover: mark.mover,
				}),
				mark.for_ms,
			);
			None
		}
		wire::ActorEvent::LegacyClearMoving(clear) => {
			if moving_mover.is_some_and(|mover| mover <= clear.mover) {
				legacy_end_mark(
					state,
					effects,
					clear.mover,
					allocation,
					start_threshold_ms,
					entropy,
					now_unix_ms,
				);
			}
			None
		}
		wire::ActorEvent::LegacyMoveDestroy(destroy) => {
			let Some(mover) = moving_mover else {
				return None;
			};
			if ended || mover != destroy.mover {
				return None;
			}
			effects.cancel_timer(wire::ActorTimer::LegacyMovingTimeout(
				wire::LegacyMovingTimeout { mover },
			));
			state.legacy_move = Some(wire::LegacyMoveMark::LegacyMoveMarkMoved(
				wire::LegacyMoveMarkMoved {
					mover: destroy.mover,
				},
			));
			Some(wire::ActorEvent::Destroy)
		}
		wire::ActorEvent::LegacyMoveAbandon(abandon) => {
			let abandons = abandon.create_unix_ms == state.create_unix_ms
				&& legacy_held(state).is_some_and(|hold| abandon.mover >= hold.mover);
			match abandons {
				true => Some(wire::ActorEvent::Destroy),
				false => None,
			}
		}
		wire::ActorEvent::Destroy => {
			if let Some(mover) = moving_mover {
				effects.cancel_timer(wire::ActorTimer::LegacyMovingTimeout(
					wire::LegacyMovingTimeout { mover },
				));
				state.legacy_move = None;
			}
			if legacy_held(state).is_some() {
				state.legacy_move = Some(wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed);
			}
			Some(wire::ActorEvent::Destroy)
		}
		wire::ActorEvent::ReserveEpoxyState(_)
		| wire::ActorEvent::WriteIndexState(_)
		| wire::ActorEvent::StartFailed(_)
		| wire::ActorEvent::Sleep
		| wire::ActorEvent::TryStart(_)
		| wire::ActorEvent::Create(_)
		| wire::ActorEvent::Reallocate(_)
		| wire::ActorEvent::UpdateRoute(_)
		| wire::ActorEvent::DestroyState(_)
		| wire::ActorEvent::ToEnvoyEvent(_)
		| wire::ActorEvent::EnvoyGone(_)
		| wire::ActorEvent::ForkDatabaseState(_)
		| wire::ActorEvent::ForkDatabaseRefused(_)
		| wire::ActorEvent::LegacyMoveConfirm(_)
		| wire::ActorEvent::MigrateFromV2(_) => Some(input),
	}
}

/// Legacy: whether the actor's destroy keeps its index entry, because the backfill moved it and
/// its page deletes the entry once the moved copy is confirmed. Without it, a page resumed
/// after a crash between the old actor's destroy and the confirm would no longer list the
/// actor, and the held copy would never be confirmed. The legacy behavior deleted the entry in
/// every destroy. Removed in 3.1 with the backfill.
fn legacy_keeps_index_entry(mark: &Option<wire::LegacyMoveMark>) -> bool {
	match mark {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoved(_)) => true,
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoving(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => false,
	}
}

/// Legacy: the mover of the backfill's moving mark the actor holds. Removed in 3.1 with the
/// backfill.
fn legacy_moving_mover(state: &wire::ActorState) -> Option<u64> {
	match &state.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => Some(moving.mover),
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => None,
	}
}

/// Legacy: ends the actor's moving mark without a destroy, by the clear of `by_mover` or at the
/// mark's deadline (`by_mover` is then the mark's own): cancels the deadline's timer, keeps the
/// highest mover seen in `LegacyMoveMarkCleared` so no lower mover marks the actor again, and
/// fires an alarm that came due while it was marked, which wakes the actor on `allocation`. A
/// sleeping actor with no envoy to start on has the alarm armed again with the no-envoy backoff,
/// as any due alarm that finds none. Removed in 3.1 with the backfill.
fn legacy_end_mark(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	by_mover: u64,
	allocation: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	entropy: Entropy,
	now_unix_ms: i64,
) {
	let Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) = state.legacy_move.take() else {
		return;
	};
	effects.cancel_timer(wire::ActorTimer::LegacyMovingTimeout(
		wire::LegacyMovingTimeout {
			mover: moving.mover,
		},
	));
	state.legacy_move = Some(wire::LegacyMoveMark::LegacyMoveMarkCleared(
		wire::LegacyMoveMarkCleared {
			mover: moving.mover.max(by_mover),
		},
	));
	if moving.deferred_alarm_unix_ms.is_some() {
		let sleeping = matches!(state.phase, wire::ActorPhase::Sleeping(_));
		if allocation.is_none() && sleeping {
			rearm_alarm(state, effects, entropy);
		} else {
			alarm_fired(
				state,
				effects,
				allocation,
				start_threshold_ms,
				false,
				now_unix_ms,
			);
		}
	}
}

/// Legacy: whether ending the actor's moving mark under `mover` or a later one fires a deferred
/// alarm that starts it, so the executor selects an envoy. Removed in 3.1 with the backfill.
fn legacy_end_mark_starts(state: &wire::ActorState, mover: u64) -> bool {
	match &state.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoving(moving)) => {
			moving.mover <= mover
				&& moving.deferred_alarm_unix_ms.is_some()
				&& matches!(state.phase, wire::ActorPhase::Sleeping(_))
		}
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoved(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => false,
	}
}

/// Resets a state inherited through a group fork for the child volume's `incarnation`: a
/// sleeping actor that keeps its identity, data, epoch, command index and scheduled alarm,
/// and drops the parent's allocation, start waiters, pending action and in-flight effect
/// timers. The parent's envoy never hears of the child. A destroy in flight continues in the
/// child, and an index write that had not succeeded is re-driven, which lands in the child's
/// bucket. An actor whose database fork had not finished is destroyed instead, since its create
/// never completed. The reset emits no start or stop, since no allocation is left to address. A
/// sleeping actor's no-envoy backoff starts over, and the parent's envoy check, stop threshold and
/// allocation retry timers are cancelled. An actor the V2 migrator held is not yet a live actor:
/// the child's copy is destroyed, since the migrator's events address the parent's volume and
/// would never activate or recall the child's.
pub fn reset_for_fork(
	state: &mut wire::ActorState,
	incarnation: [u8; 16],
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
) {
	let create_unix_ms = state.create_unix_ms;
	let index_bucket_count = state.index_bucket_count;
	let inherited = std::mem::replace(
		&mut state.phase,
		wire::ActorPhase::Sleeping(wire::Sleeping { wake_retries: 0 }),
	);
	// Timers are identified by variant, so any payload cancels the inherited one.
	effects.cancel_timer(wire::ActorTimer::EnvoyCheckTimeout(
		wire::EnvoyCheckTimeout { epoch: state.epoch },
	));
	effects.cancel_timer(wire::ActorTimer::StopThresholdTimeout(
		wire::StopThresholdTimeout {
			epoch: state.epoch,
			stop_unix_ms: 0,
		},
	));
	effects.cancel_timer(wire::ActorTimer::AllocationRetry(wire::AllocationRetry {
		epoch: state.epoch,
		attempt: 0,
	}));
	match inherited {
		wire::ActorPhase::Initializing(init) => {
			effects.cancel_timer(wire::ActorTimer::ReserveEpoxyTimeout(
				wire::ReserveEpoxyTimeout {
					attempt: init.reserve_epoxy.state.attempt,
					create_unix_ms,
				},
			));
			// A database fork that had not finished in the parent leaves a create that never
			// completed. A sleeping copy would start on an empty database while claiming to be
			// forked, so the child destroys it instead, deleting whatever the fork left.
			// Legacy: a backfill move still held for its confirm never completed either: its old
			// actor may still be the actor, so the child destroys the copy rather than make it
			// startable. Removed in 3.1 with the backfill.
			let fork_finished = match &init.fork_database {
				Some(fork) => {
					effects.cancel_timer(fork_database_timer(fork, create_unix_ms));
					effects.cancel_timer(fork_database_retry_timer(fork, create_unix_ms));
					match &fork.state.phase {
						wire::ForkDatabasePhase::ForkDatabasePhaseForked => true,
						wire::ForkDatabasePhase::ForkDatabasePhasePreparing
						| wire::ForkDatabasePhase::ForkDatabasePhasePrepared(_)
						| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_) => false,
					}
				}
				None => true,
			};
			if !fork_finished || init.legacy_move.is_some() {
				let mut destroy = wire::DestroyEffectData {
					create_unix_ms,
					state: wire::DestroyState {
						phase: wire::DestroyPhase::DestroyPhasePending,
						attempt: 0,
						create_unix_ms,
					},
				};
				walk_destroy(
					&mut destroy,
					index_bucket_count,
					legacy_keeps_index_entry(&state.legacy_move),
					effects,
					None,
				);
				state.phase = wire::ActorPhase::Destroying(wire::Destroying {
					destroy,
					key_owner: None,
				});
			}
			if state.write_index.state.phase != wire::WriteIndexPhase::WriteIndexPhaseSuccess {
				walk_write_index(
					&mut state.write_index,
					index_bucket_count,
					create_unix_ms,
					effects,
					None,
				);
			}
		}
		wire::ActorPhase::Starting(_) => {
			effects.cancel_timer(wire::ActorTimer::StartTimeout(wire::StartTimeout {
				epoch: state.epoch,
			}));
		}
		wire::ActorPhase::Stopping(_) => {
			effects.cancel_timer(wire::ActorTimer::StopTimeout(wire::StopTimeout {
				epoch: state.epoch,
			}));
		}
		wire::ActorPhase::Running(_) | wire::ActorPhase::Sleeping(_) => {}
		wire::ActorPhase::Destroying(mut destroying) => {
			// The key owner the parent's reservation found answers for the parent's namespace.
			// The child lives in another namespace, whose key V2 never held and whose owner
			// elsewhere was never asked, so its destroy ends `Destroyed`, never `DelegatedToV2`.
			destroying.key_owner = None;
			walk_destroy(
				&mut destroying.destroy,
				index_bucket_count,
				legacy_keeps_index_entry(&state.legacy_move),
				effects,
				None,
			);
			state.phase = wire::ActorPhase::Destroying(destroying);
		}
		wire::ActorPhase::MigratingFromV2(held) => {
			if let Some(reserve_epoxy) = &held.reserve_epoxy {
				effects.cancel_timer(wire::ActorTimer::ReserveEpoxyTimeout(
					wire::ReserveEpoxyTimeout {
						attempt: reserve_epoxy.state.attempt,
						create_unix_ms,
					},
				));
			}
			state.start_waiters.clear();
			destroy_actor(state, effects, None, create_unix_ms);
		}
		// A key delegated to V2 belongs to the parent's migrating namespace, never to the child,
		// and the delegation has no data record or index entry to delete, so the child's copy
		// simply ends destroyed.
		wire::ActorPhase::Destroyed | wire::ActorPhase::DelegatedToV2 => {
			state.phase = wire::ActorPhase::Destroyed;
		}
	}
	// The waiters were waits on the parent's notice subjects; they expire there.
	state.start_waiters.clear();
	state.pending_action = wire::PendingAction::PendingNone;
	state.fork_reset = Some(incarnation);
}

/// The state an inherited, not yet reset state reads as, so a reader never routes to the
/// parent's envoy: the reset applied to a copy, with its effects discarded.
pub fn reset_view(state: &wire::ActorState, incarnation: [u8; 16]) -> wire::ActorState {
	let mut view = state.clone();
	let mut discarded = Effects::new(Clocks {
		boot_ms: 0,
		unix_ms: 0,
	});
	reset_for_fork(&mut view, incarnation, &mut discarded);
	view
}

fn phase_label(phase: &wire::ActorPhase) -> &'static str {
	match phase {
		wire::ActorPhase::Initializing(_) => "initializing",
		wire::ActorPhase::Starting(_) => "starting",
		wire::ActorPhase::Running(_) => "running",
		wire::ActorPhase::Stopping(_) => "stopping",
		wire::ActorPhase::Sleeping(_) => "sleeping",
		wire::ActorPhase::Destroying(_) => "destroying",
		wire::ActorPhase::Destroyed => "destroyed",
		wire::ActorPhase::MigratingFromV2(_) => "migrating_from_v2",
		wire::ActorPhase::DelegatedToV2 => "delegated_to_v2",
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
	// Legacy: an actor the backfill is moving never starts, so it selects no envoy, unless the
	// event ends its mark and fires the alarm the mark kept. Removed in 3.1 with the backfill.
	let ends_mark = match event {
		RuntimeEvent::Input(Event(wire::ActorEvent::LegacyClearMoving(clear))) => {
			Some(legacy_end_mark_starts(state, clear.mover))
		}
		RuntimeEvent::Timer(wire::ActorTimer::LegacyMovingTimeout(timeout)) => Some(
			legacy_moving_mover(state) == Some(timeout.mover)
				&& legacy_end_mark_starts(state, timeout.mover),
		),
		RuntimeEvent::Input(_) | RuntimeEvent::Timer(_) => None,
	};
	if let Some(starts) = ends_mark {
		return starts;
	}
	if legacy_moving(state) {
		return false;
	}
	let reallocation_pending = state.pending_action == wire::PendingAction::PendingReallocate;
	let sleeping = matches!(state.phase, wire::ActorPhase::Sleeping(_));
	let initializing_without_allocation = matches!(
		&state.phase,
		wire::ActorPhase::Initializing(init) if init.allocation.is_none()
	);
	match event {
		RuntimeEvent::Input(Event(input)) => match input {
			wire::ActorEvent::TryStart(_) => sleeping || initializing_without_allocation,
			// Legacy: a backfill move leaves its actor asleep, so it selects no envoy. Removed in
			// 3.1 with the backfill.
			wire::ActorEvent::Create(create) if create.init.legacy_move.is_some() => false,
			wire::ActorEvent::Create(_) => {
				sleeping
					|| initializing_without_allocation
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
			| wire::ActorEvent::MigrateFromV2(_)
			| wire::ActorEvent::ForkDatabaseState(_)
			| wire::ActorEvent::ForkDatabaseRefused(_)
			| wire::ActorEvent::LegacyMarkMoving(_)
			| wire::ActorEvent::LegacyMoveDestroy(_)
			| wire::ActorEvent::LegacyMoveAbandon(_) => false,
			// Legacy: decided above, since it ends the mark. Removed in 3.1 with the backfill.
			wire::ActorEvent::LegacyClearMoving(_) => false,
			// Legacy: a confirmed move starts only when a request stored an envoy during the
			// move, and on one selected now, since the stored one may be gone. Removed in 3.1
			// with the backfill.
			wire::ActorEvent::LegacyMoveConfirm(_) => matches!(
				&state.phase,
				wire::ActorPhase::Initializing(init) if init.allocation.is_some()
			),
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
			| wire::ActorTimer::DestroyTimeout(_)
			| wire::ActorTimer::ForkDatabaseTimeout(_)
			| wire::ActorTimer::ForkDatabaseRetry(_) => false,
			// Legacy: decided above, since it ends the mark. Removed in 3.1 with the backfill.
			wire::ActorTimer::LegacyMovingTimeout(_) => false,
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

/// Whether a destroy waits for the current stop, which ends the actor's alarm with it.
fn pending_destroy(state: &wire::ActorState) -> bool {
	match state.pending_action {
		wire::PendingAction::PendingDestroy => true,
		wire::PendingAction::PendingNone
		| wire::PendingAction::PendingSleep
		| wire::PendingAction::PendingReallocate => false,
	}
}

/// Arms the actor's alarm again after a start that did not complete, counted under `reason`.
/// The start's envoy took the epoch and never ran it, so it is likely slow or gone: the retry
/// waits the no-envoy backoff's ceiling, plus its jitter, rather than its first step, and a
/// start that keeps failing retries at that pace. It replaces any alarm armed earlier; a
/// runtime that starts on the retry pushes its own next alarm. A start that completes arms
/// nothing, so a one-shot alarm that already ran does not wake the actor again.
fn arm_failed_start_alarm_retry(
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	reason: &'static str,
	entropy: Entropy,
) {
	crate::metrics::ACTOR_ALARM_RETRIES
		.with_label_values(&[reason])
		.inc();
	let jitter_ms = entropy.rng().gen_range(0..ALARM_NO_ENVOY_RETRY_JITTER_MS);
	effects.arm_timer_after(
		wire::ActorTimer::AlarmTimeout,
		ALARM_NO_ENVOY_RETRY_MAX_MS.saturating_add(jitter_ms),
	);
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

	// Legacy: an actor the backfill is moving starts nothing, so its reallocation is kept in the
	// mark as a due alarm, which the move carries, before the retry is armed or the allocation
	// sleeps. Removed in 3.1 with the backfill.
	if state.pending_action == wire::PendingAction::PendingReallocate {
		legacy_defer_alarm(state, now_unix_ms);
	}

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

	// Legacy: an actor the backfill is moving never starts; it sleeps until its destroy.
	// Removed in 3.1 with the backfill.
	if legacy_moving(state) {
		enter_sleeping(state);
		return;
	}
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
/// report running. The start timer bounds that wait to `start_threshold_ms`. The waiters are told
/// where the actor is starting and kept for the running notice.
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

/// Whether the current start is the incarnation's first over storage it created empty. Every
/// command of an incarnation takes the next index, starting from 1, so index 1 is its first start,
/// and no runner has seen the database volume named by its `create_unix_ms`. An imported
/// incarnation's volume already holds the data imported into it, and a forked one the source's
/// copied data, so the runtime must read those tables instead of bootstrapping over them.
fn is_new_storage(state: &wire::ActorState) -> bool {
	let created = match state.storage_origin {
		wire::StorageOrigin::Created => true,
		wire::StorageOrigin::Imported => false,
	};
	let empty = match &state.database_origin {
		wire::ActorDatabaseOrigin::DatabaseEmpty => true,
		wire::ActorDatabaseOrigin::DatabaseForked
		| wire::ActorDatabaseOrigin::DatabaseForkFailed(_) => false,
	};
	state.command_index == 1 && created && empty
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
		stop_command_action(state),
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
		pending_action: stop_command_action(state),
	});
	effects.arm_timer_after(
		wire::ActorTimer::StopTimeout(wire::StopTimeout { epoch: state.epoch }),
		STOP_RESEND_MS + jitter_ms,
	);
}

/// The pending action the current epoch's stop command carries as the runtime's stop reason.
///
/// Legacy: an actor the backfill moved lives on as its copy, so its runtime always stops as a
/// sleep, even once the move's destroy made the pending action `PendingDestroy`. That covers a
/// resend that reaches the runtime after the first stop was lost, and the first stop of an
/// actor that was still starting when the destroy came; neither runs the user's destroy hook.
/// The state keeps `PendingDestroy`, so the stopped report still ends the actor in
/// `Destroying`. Removed in 3.1 with the backfill.
fn stop_command_action(state: &wire::ActorState) -> wire::PendingAction {
	let moved = match &state.legacy_move {
		Some(wire::LegacyMoveMark::LegacyMoveMarkMoved(_)) => true,
		Some(
			wire::LegacyMoveMark::LegacyMoveMarkMoving(_)
			| wire::LegacyMoveMark::LegacyMoveMarkCopyDestroyed
			| wire::LegacyMoveMark::LegacyMoveMarkCleared(_),
		)
		| None => false,
	};
	match state.pending_action {
		wire::PendingAction::PendingDestroy if moved => wire::PendingAction::PendingSleep,
		wire::PendingAction::PendingNone
		| wire::PendingAction::PendingSleep
		| wire::PendingAction::PendingReallocate
		| wire::PendingAction::PendingDestroy => state.pending_action,
	}
}

/// A try start, or a create of an actor that exists: the waiter is stored against the phase
/// the actor is in, then the actor is woken.
fn try_start(
	state: &mut wire::ActorState,
	effects: &mut Effects<Effect, wire::ActorTimer, WriteEffect>,
	waiter: Option<wire::StartWaiter>,
	hint: Option<wire::ActorAllocation>,
	start_threshold_ms: u64,
	start_dispatched: bool,
	start_waiters_capacity: u32,
	now_unix_ms: i64,
) {
	// Legacy: a wake of an actor the backfill is moving stores no waiter, since nothing will
	// start it; the command answers `TryStartLegacyMoving`. Removed in 3.1 with the backfill.
	if !legacy_moving(state) {
		store_start_waiter(state, waiter, start_waiters_capacity, now_unix_ms);
	}
	wake(
		state,
		effects,
		hint,
		start_threshold_ms,
		start_dispatched,
		now_unix_ms,
	);
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
		legacy_keeps_index_entry(&state.legacy_move),
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
	state.database_origin = wire::ActorDatabaseOrigin::DatabaseEmpty;
	state.legacy_namespace = begin.init.legacy_namespace.clone();
	state.command_index = 0;
	state.pending_action = wire::PendingAction::PendingNone;
	state.start_waiters.clear();

	// A migrated actor lands as a fresh create in its namespace would: its index and Epoxy
	// entries are keyed in the namespace's group, or by its legacy namespace without one.
	state.write_index = wire::WriteIndexEffectData {
		name: begin.init.name.clone(),
		create_unix_ms,
		state: wire::WriteIndexState {
			phase: wire::WriteIndexPhase::WriteIndexPhasePending,
			attempt: 0,
			create_unix_ms,
		},
	};
	let mut reserve_epoxy = wire::ReserveEpoxyEffectData {
		name: begin.init.name.clone(),
		key: begin.init.key.clone(),
		state: wire::ReserveEpoxyState {
			phase: wire::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
			attempt: 0,
			create_unix_ms,
		},
	};

	// The first attempt runs before the data record below commits, so it carries the legacy
	// namespace the begin names instead of reading the record.
	let legacy_namespace = begin
		.init
		.legacy_namespace
		.as_ref()
		.map(|namespace| namespace.to_string());
	effects.write(WriteEffect::WriteData {
		data: wire::ActorData {
			actor_id: begin.init.actor_id,
			pool_name: begin.init.pool_name,
			name: begin.init.name,
			key: begin.init.key,
			input: begin.init.input,
			create_unix_ms,
			legacy_namespace: begin.init.legacy_namespace,
			forked_from: None,
			legacy_moved_from: None,
		},
	});
	walk_reserve_epoxy(
		&mut reserve_epoxy,
		create_unix_ms,
		KeyCheck::Epoxy,
		effects,
		None,
		legacy_namespace,
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
	/// so asking V2 would always find it. Legacy: a backfill move reserves the key of an actor
	/// V3 already owns (see `create_key_check`). Removed in 3.1 with the backfill.
	Epoxy,
	/// Epoxy, and V2 while the actor's namespace migrates in from V2, so a create of a key V2
	/// already holds ends delegated to V2. The executor reads the migration config when it
	/// runs, so a migration configured or removed between attempts applies at once.
	EpoxyAndV2,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
	/// **When**
	///
	/// A keyed actor's create starts its reservation, or a failed or timed out attempt is retried.
	///
	/// **Executor**
	///
	/// The actor executor reserves `name` and `key` in Epoxy under the actor's group, or at the
	/// legacy unprefixed key named by the legacy namespace for an unprefixed volume, then
	/// returns `ReserveEpoxyState` with the outcome.
	ReserveEpoxy {
		name: String,
		key: String,
		attempt: u32,
		create_unix_ms: i64,
		key_check: KeyCheck,
		/// Legacy: the namespace of an unprefixed actor, carried by the first attempt from its
		/// create, since that attempt runs before the create's data record commits. Absent for a
		/// grouped actor and for later attempts, which read it from the record. Removed in 3.1
		/// with the backfill.
		legacy_namespace: Option<String>,
	},
	/// **When**
	///
	/// A create starts the incarnation's index entry, or a failed or timed out attempt is retried.
	///
	/// **Executor**
	///
	/// The actor executor inserts the entry in the bucket of `name` under the actor's group, or
	/// in the legacy unprefixed bucket of the record's legacy namespace for an unprefixed volume,
	/// then returns `WriteIndexState` with the outcome.
	WriteIndex {
		name: String,
		index_bucket_count: u64,
		create_unix_ms: i64,
		attempt: u32,
	},
	Start {
		epoch: u32,
		command_index: i64,
		allocation: wire::ActorAllocation,
		/// Whether this is the incarnation's first start on an empty database, so no runner
		/// has written its storage. A forked incarnation's first start is not. The executor then preloads the empty keyspace, which lets the runner skip
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
	/// **When**
	///
	/// The actor's destroy starts for its incarnation `create_unix_ms`, or its previous attempt
	/// failed or `DestroyTimeout` fired.
	///
	/// **Executor**
	///
	/// The actor executor waits the attempt's backoff, moves the incarnation's index entry to
	/// the destroyed entries unless `legacy_keep_index_entry` is set, deletes its database
	/// volume, clears a finished backfill's skipped entry for an unprefixed actor, and returns
	/// `DestroyState` with the outcome.
	Destroy {
		attempt: u32,
		index_bucket_count: u64,
		create_unix_ms: i64,
		/// Legacy: the actor is an unprefixed actor the backfill moved, whose index entry the
		/// backfill page deletes once the moved copy is confirmed, so a page resumed after a
		/// crash still lists it and confirms the copy. The legacy behavior deleted the entry in
		/// the destroy. Set only while the actor holds `LegacyMoveMarkMoved`. Removed in 3.1 with
		/// the backfill.
		legacy_keep_index_entry: bool,
	},
	/// **When**
	///
	/// A forked actor's key is reserved and its database fork starts, the fork's prepared body
	/// was just persisted, `ForkDatabaseTimeout` fired for an attempt still in flight, or
	/// `ForkDatabaseRetry` fired after a refusal.
	///
	/// **Executor**
	///
	/// At `Preparing`, the actor executor calls Depot's `prepare_fork` for the latest version of
	/// the source's database volume into this incarnation's database volume and returns
	/// `ForkDatabaseState` with `Prepared` and the encoded body. At `Prepared`, it sends the
	/// persisted body with `resume_fork` and returns `ForkDatabaseState` with `Forked` or
	/// `Failed`, or `ForkDatabaseRefused` with Depot's hint. When the job fails because the
	/// source has no record in this group yet (inherited and never opened), it opens and closes
	/// the source, which makes the record from the parent group's copy, prepares a new body and
	/// returns `ForkDatabaseState` with `Prepared`. At `Prepared` with `tailUnprovenRefusals`
	/// above zero, it first reads the source actor's state and, only when the source sleeps,
	/// opens and closes the source's database volume, whose recovery proves the tail Depot
	/// refused the fork for; a `TailUnproven` refusal returns `ForkDatabaseRefused` with
	/// `tailUnproven` set. An unknown outcome, or a preparation that sent nothing, returns
	/// nothing, and the timer sends the step again.
	///
	/// The executor first waits `backoff_ms`: the effect backoff of the attempt for a step
	/// sent again after a timeout or a refusal, and zero for a first step or for the step a
	/// successful one leads to, which is progress and not a retry.
	ForkDatabase {
		data: wire::ForkDatabaseEffectData,
		backoff_ms: u64,
	},
	/// **When**
	///
	/// The state machine accepted a step result or a refusal of the current fork attempt, so a
	/// stale or duplicate report, which changes nothing, is never counted.
	///
	/// **Executor**
	///
	/// Counts the step in `pegboard_actor_fork_steps_total` and, for `Forked`, observes
	/// `elapsed_ms` in `pegboard_actor_fork_seconds`, both labelled with the actor's namespace.
	/// No response.
	ObserveForkStep {
		result: ForkStepResult,
		/// Time from the incarnation's create to the accepted `Forked`, by the event's clock.
		/// Set only for `Forked`.
		elapsed_ms: Option<i64>,
	},
}

/// The kind of fork step the state machine accepted, the `result` label of its metric.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkStepResult {
	Prepared,
	Forked,
	Failed,
	Refused,
}

impl ForkStepResult {
	pub fn label(self) -> &'static str {
		match self {
			ForkStepResult::Prepared => "prepared",
			ForkStepResult::Forked => "forked",
			ForkStepResult::Failed => "failed",
			ForkStepResult::Refused => "refused",
		}
	}
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
			Effect::ForkDatabase { .. } => "fork_database",
			Effect::ObserveForkStep { .. } => "observe_fork_step",
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
			wire::ActorEvent::ForkDatabaseState(_) => "fork_database_state",
			wire::ActorEvent::ForkDatabaseRefused(_) => "fork_database_refused",
			wire::ActorEvent::LegacyMoveConfirm(_) => "legacy_move_confirm",
			wire::ActorEvent::LegacyMarkMoving(_) => "legacy_mark_moving",
			wire::ActorEvent::LegacyClearMoving(_) => "legacy_clear_moving",
			wire::ActorEvent::LegacyMoveDestroy(_) => "legacy_move_destroy",
			wire::ActorEvent::LegacyMoveAbandon(_) => "legacy_move_abandon",
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
	ForkDatabaseTimeout = 100,
	ForkDatabaseRetry = 101,
	LegacyMovingTimeout = 102,
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
			wire::ActorTimer::ForkDatabaseTimeout(_) => Self::ForkDatabaseTimeout,
			wire::ActorTimer::ForkDatabaseRetry(_) => Self::ForkDatabaseRetry,
			wire::ActorTimer::LegacyMovingTimeout(_) => Self::LegacyMovingTimeout,
		}
	}
}
