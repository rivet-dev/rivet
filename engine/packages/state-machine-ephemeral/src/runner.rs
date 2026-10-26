//! Serializes inbox messages and timers into state machine steps. Each step publishes
//! its snapshot before dispatching effects; async completions return through the inbox.
//! Due alarms run before inbox inputs, including after a pause between selection and
//! clock observation. Alarm effects take effect before the next delivery is chosen.

use crate::{
	EphemeralStateMachine, EphemeralStateMachineEffect, EphemeralStateMachineEvent, RuntimeEffect,
	RuntimeEvent,
};
use rivet_env::{ClockError, Clocks, Env};
use rivet_state_machine::Effects;
use rivet_state_machine::{
	Effect, Entropy, Event, EventBody, RunnerMetrics, RunnerPhase, TraceHasher,
};
use std::{
	collections::{BTreeMap, BTreeSet},
	future::Future,
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
	task::Poll,
};
use tokio::sync::{mpsc, oneshot, watch};

/// Sends executor completions to a runner spawned with
/// [`Runner::spawn_with_completions`]. Unbounded, so a completion never waits for queued
/// inputs; the machine bounds it by bounding the effects it has outstanding.
pub type CompletionSender<M> = mpsc::UnboundedSender<<M as EphemeralStateMachine>::Event>;

/// The runner's inbox, `Env` and telemetry record for one effect.
pub type EffectContext<M> =
	rivet_state_machine::EffectContext<mpsc::Sender<<M as EphemeralStateMachine>::Event>>;

/// Builds the context for one effect outside a runner, for custom drivers and tests that call
/// an executor directly. Its telemetry records under the machine's `DEBUG_NAME` like a runner's.
pub fn effect_context<M: EphemeralStateMachine>(
	effect: &M::Effect,
	inbox: mpsc::Sender<M::Event>,
	env: Arc<dyn Env>,
) -> EffectContext<M> {
	RunnerMetrics::new(M::DEBUG_NAME).dispatch(effect.name(), inbox, env)
}

/// Construct stateful executors per runner. Sharing one across runners requires
/// keeping their in-flight work and cancellation independent.
pub trait Executor<M: EphemeralStateMachine>: Send + Sync + 'static {
	/// Return promptly so the runner can keep processing events. Use injected I/O
	/// clients and send completions to `ctx.inbox()`; the runner timestamps them on delivery.
	/// Spawn async work through `ctx.spawn` so its duration and result are recorded.
	fn execute(&self, effect: M::Effect, ctx: EffectContext<M>);
}

/// Observes published snapshots on the runner task without borrowing the snapshot watch.
///
/// `sample` runs on the freshly built snapshot before it is published, with no watch lock held.
/// `deliver` runs once the snapshot is visible to watch readers, again with no lock held, and
/// only when the snapshot differs from the previously published one. Both also run once at spawn
/// for the initial snapshot. Because neither call holds a watch lock, `deliver` may take a lock
/// that a watch reader holds without deadlocking. Both run on the runner task, so they must stay
/// cheap: a slow observer stalls every transition of its state machine.
pub trait SnapshotObserver<T>: Send + Sync + 'static {
	type Sample: Send;

	fn sample(&self, snapshot: &T) -> Self::Sample;

	fn deliver(&self, sample: Self::Sample);
}

/// Receives the wall time the runner spent on each event: its transition, snapshot and
/// effect dispatch. It runs on the runner task, so it must stay cheap. Telemetry only.
pub trait BusyObserver: Send + Sync + 'static {
	fn record_busy(&self, busy: std::time::Duration);
}

/// Object-safe publication through a [`SnapshotObserver`], so crates can pass an observer
/// without naming its sample type.
pub trait PublishObserver<T>: Send + Sync + 'static {
	/// Samples `snapshot`, publishes it when it changed, then delivers the sample.
	fn publish(&self, snapshot: T, snapshots: &watch::Sender<T>);

	/// Samples and delivers the initial snapshot, which is never published through `publish`.
	fn initial(&self, snapshot: &T);
}

impl<T: PartialEq, O: SnapshotObserver<T>> PublishObserver<T> for O {
	fn publish(&self, snapshot: T, snapshots: &watch::Sender<T>) {
		// The runner is the only writer, so comparing under a short read guard and replacing
		// afterwards cannot race another publication.
		let changed = *snapshots.borrow() != snapshot;
		if !changed {
			return;
		}
		let sample = self.sample(&snapshot);
		snapshots.send_replace(snapshot);
		self.deliver(sample);
	}

	fn initial(&self, snapshot: &T) {
		let sample = self.sample(snapshot);
		self.deliver(sample);
	}
}

/// Runs one state machine in an Env task; transitions never overlap.
pub struct Runner<M: EphemeralStateMachine>(PhantomData<M>);

pub struct RunnerHandle<M: EphemeralStateMachine> {
	/// Bounded input queue. The runner stops when all strong senders are dropped.
	pub inbox: mpsc::Sender<M::Event>,
	/// Latest published snapshot; slow readers may skip intermediate snapshots.
	pub snapshot: watch::Receiver<M::Snapshot>,
	/// Resolves successfully after a clean drain; cancellation or panic returns an error.
	pub stopped: oneshot::Receiver<()>,
}

type TimerFuture = Pin<Box<dyn Future<Output = Result<(), ClockError>> + Send>>;

/// How the runner learns that its earliest alarm is due. Either way the runner then checks its
/// alarms against a fresh clock, so a wake only prompts that check.
enum Wake {
	/// One sleep for the earliest alarm, with the deadline it sleeps until.
	Own(Option<(u64, TimerFuture)>),
	/// The runner's earliest alarm is queued in a shared deadline queue, and `waiting` resolves
	/// when the queue's driver wakes it.
	Shared {
		slot: crate::shared_timers::TimerSlot,
		waiting: Pin<Box<dyn Future<Output = ()> + Send>>,
	},
}

struct Running<M: EphemeralStateMachine> {
	env: Arc<dyn Env>,
	state: M::State,
	executor: Arc<dyn Executor<M>>,
	// Avoid keeping the runner alive through its own completion channel.
	inbox: mpsc::WeakSender<M::Event>,
	snapshots: watch::Sender<M::Snapshot>,
	// Dropped with the runner task, so the observer's registrations end with the state machine.
	observer: Option<Arc<dyn PublishObserver<M::Snapshot>>>,
	busy: Option<Arc<dyn BusyObserver>>,
	// Deadline order lets the event loop wait only for the earliest timer.
	armed: BTreeSet<(u64, M::Timer)>,
	// Each armed timer's deadline, so arm and cancel find its `armed` entry by identity in
	// O(log armed) instead of scanning. It always holds exactly the timers in `armed`.
	armed_at: BTreeMap<M::Timer, u64>,
	wake: Wake,
	seq: u64,
	metrics: RunnerMetrics,
	/// Simulated processing time not yet spent because it is under one clock millisecond.
	cost_debt_us: u64,
}

impl<M: EphemeralStateMachine> Runner<M> {
	/// Runs startup effects before accepting events. The initial snapshot is available
	/// immediately; startup and later steps notify readers only when it changes.
	pub fn spawn(
		env: Arc<dyn Env>,
		state: M::State,
		executor: Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		Self::spawn_with_executor(env, state, |_| executor)
	}

	/// Connect handlers to the same snapshot published before each effect.
	pub fn spawn_with_executor(
		env: Arc<dyn Env>,
		state: M::State,
		make_executor: impl FnOnce(watch::Receiver<M::Snapshot>) -> Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		Self::spawn_with_observer(env, state, None, make_executor)
	}

	/// Like [`Runner::spawn_with_executor`], and also reports every changed snapshot, plus the
	/// initial one, to `observer` on the runner task.
	pub fn spawn_with_observer(
		env: Arc<dyn Env>,
		state: M::State,
		observer: Option<Arc<dyn PublishObserver<M::Snapshot>>>,
		make_executor: impl FnOnce(watch::Receiver<M::Snapshot>) -> Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		Self::spawn_with_observers(env, state, observer, None, make_executor)
	}

	/// Like [`Runner::spawn_with_observer`], and also reports the time spent on each event to
	/// `busy`.
	pub fn spawn_with_observers(
		env: Arc<dyn Env>,
		state: M::State,
		observer: Option<Arc<dyn PublishObserver<M::Snapshot>>>,
		busy: Option<Arc<dyn BusyObserver>>,
		make_executor: impl FnOnce(watch::Receiver<M::Snapshot>) -> Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		Self::spawn_with_timers(env, state, observer, busy, None, make_executor)
	}

	/// Like [`Runner::spawn_with_observers`]. With `timers`, the runner keeps no sleep of its
	/// own: its earliest alarm waits in that shared deadline queue, whose owner must run it
	/// while this runner can hold alarms.
	pub fn spawn_with_timers(
		env: Arc<dyn Env>,
		state: M::State,
		observer: Option<Arc<dyn PublishObserver<M::Snapshot>>>,
		busy: Option<Arc<dyn BusyObserver>>,
		timers: Option<crate::SharedTimers>,
		make_executor: impl FnOnce(watch::Receiver<M::Snapshot>) -> Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		Self::spawn_inner(env, state, observer, busy, timers, false, |snapshot, _| {
			make_executor(snapshot)
		})
	}

	/// Like [`Runner::spawn_with_executor`], and also gives the executor a completion channel.
	/// Completions sent on it are delivered after due alarms and before any queued input, so
	/// work the machine already admitted finishes ahead of new requests, and a sender never
	/// waits for the bounded inbox. The runner still stops when the inbox closes.
	pub fn spawn_with_completions(
		env: Arc<dyn Env>,
		state: M::State,
		make_executor: impl FnOnce(
			watch::Receiver<M::Snapshot>,
			CompletionSender<M>,
		) -> Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		Self::spawn_inner(
			env,
			state,
			None,
			None,
			None,
			true,
			|snapshot, completions| {
				let completions = completions.expect("completion channel requested");
				make_executor(snapshot, completions)
			},
		)
	}

	fn spawn_inner(
		env: Arc<dyn Env>,
		state: M::State,
		observer: Option<Arc<dyn PublishObserver<M::Snapshot>>>,
		busy: Option<Arc<dyn BusyObserver>>,
		timers: Option<crate::SharedTimers>,
		with_completions: bool,
		make_executor: impl FnOnce(
			watch::Receiver<M::Snapshot>,
			Option<CompletionSender<M>>,
		) -> Arc<dyn Executor<M>>,
	) -> RunnerHandle<M> {
		let (inbox, mut receiver) = mpsc::channel(M::inbox_capacity(&state));
		let initial = M::snapshot(&state);
		if let Some(observer) = &observer {
			observer.initial(&initial);
		}
		let (snapshots, snapshot) = watch::channel(initial);
		let (completion_sender, mut completions) = if with_completions {
			let (sender, receiver) = mpsc::unbounded_channel();
			(Some(sender), Some(receiver))
		} else {
			(None, None)
		};
		let executor = make_executor(snapshot.clone(), completion_sender);
		let (finished, stopped) = oneshot::channel();
		let handle = RunnerHandle {
			inbox: inbox.clone(),
			snapshot,
			stopped,
		};
		let mut run = Running::<M> {
			env: env.clone(),
			state,
			executor,
			inbox: inbox.downgrade(),
			snapshots,
			observer,
			busy,
			armed: BTreeSet::new(),
			armed_at: BTreeMap::new(),
			wake: match timers {
				Some(timers) => {
					let slot = timers.register();
					let waiting = Box::pin(slot.wake());
					Wake::Shared { slot, waiting }
				}
				None => Wake::Own(None),
			},
			seq: 0,
			metrics: RunnerMetrics::new(M::DEBUG_NAME),
			cost_debt_us: 0,
		};
		// Scheduling telemetry separates a runner that is busy from one starved by the runtime.
		env.spawn(Box::pin(rivet_metrics::monitor_task(
			M::DEBUG_NAME,
			async move {
				let now = run.read_clocks().await;
				let mut entropy_stream = run.env.rng("state-machine.entropy");
				let entropy = Entropy(entropy_stream.fork_seed());
				tracing::info!(target: "rivet_state_machine", sm = M::DEBUG_NAME,
				seq = run.seq, ?now, ?entropy, "state machine startup");
				let mut effects = Effects::new(now);
				#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
				let started = std::time::Instant::now();
				M::start(&mut run.state, now, entropy, &mut effects);
				run.metrics
					.observe_transition("start", started.elapsed().as_secs_f64());
				run.debug_validate().await;
				run.publish_snapshot();
				run.dispatch_effects(effects.drain()).await;
				run.debug_validate_timers().await;

				// Startup can dispatch work before relinquishing our strong sender.
				drop(inbox);
				let mut rng = run.env.rng("state-machine.select");

				let mut pending_input = None;
				let mut pending_completion = None;
				let mut inbox_closed = false;
				let mut alarm_batch = 0;
				loop {
					// Alarm priority must not starve other tasks or scope cancellation.
					// Keep the input queued and recheck time/alarms after yielding.
					if alarm_batch == 64 {
						rivet_env::tokio::task::yield_now().await;
						alarm_batch = 0;
					}
					// Selection only wakes this loop. Choose delivery using a fresh clock
					// and the current queue, never a previously selected alarm.
					let now = run.read_clocks().await;
					if run.armed.first().is_some_and(|(at, _)| *at <= now.boot_ms) {
						let (_, timer) = run.armed.pop_first().expect("due alarm exists");
						run.armed_at.remove(&timer);
						run.process_event(Event {
							now,
							entropy: Entropy(entropy_stream.fork_seed()),
							body: RuntimeEvent::Timer(timer),
						})
						.await;
						alarm_batch += 1;
						continue;
					}
					alarm_batch = 0;
					// Completions of admitted work go ahead of every queued input.
					let completion = pending_completion.take().or_else(|| {
						completions
							.as_mut()
							.and_then(|channel| channel.try_recv().ok())
					});
					if let Some(body) = completion {
						run.process_event(Event {
							now,
							entropy: Entropy(entropy_stream.fork_seed()),
							body: RuntimeEvent::Input(body),
						})
						.await;
						continue;
					}
					if let Some(body) = pending_input.take() {
						run.process_event(Event {
							now,
							entropy: Entropy(entropy_stream.fork_seed()),
							body: RuntimeEvent::Input(body),
						})
						.await;
						continue;
					}
					if inbox_closed {
						break;
					}

					run.arm_wake();
					rivet_env::select!(&mut rng;
						result = std::future::poll_fn(|cx| run.poll_wake(cx)) => {
							if let Err(error) = result {
								run.exit_on_clock_error(error).await;
							}
						},
						body = receiver.recv() => {
							inbox_closed = body.is_none();
							pending_input = body;
							run.metrics.observe_inbox_depth(receiver.len());
						},
						body = std::future::poll_fn(|cx| match completions.as_mut() {
							Some(channel) => channel.poll_recv(cx),
							None => Poll::Pending,
						}) => {
							// Every sender gone means no completion can arrive; stop polling it.
							match body {
								Some(body) => pending_completion = Some(body),
								None => completions = None,
							}
						},
					);
				}
				drop(run);
				let _ = finished.send(());
			},
		)));
		handle
	}
}

impl<M: EphemeralStateMachine> Running<M> {
	/// Points the wake at the earliest armed alarm: a fresh sleep when its deadline changed, or
	/// the shared queue's entry. Nothing is armed when no alarm is.
	fn arm_wake(&mut self) {
		let earliest = self.armed.first().map(|(at, _)| *at);
		match &mut self.wake {
			Wake::Own(sleep) => {
				if sleep.as_ref().map(|(at, _)| *at) != earliest {
					*sleep = earliest.map(|at| (at, self.env.clock().sleep_until(at)));
				}
			}
			Wake::Shared { slot, .. } => slot.set(earliest),
		}
	}

	/// Resolves when the earliest alarm may be due. A completed wake is cleared, so an early
	/// adapter wake never delivers an early alarm and no completed future is polled again.
	fn poll_wake(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), ClockError>> {
		match &mut self.wake {
			Wake::Own(sleep) => {
				let Some((_, future)) = sleep else {
					return Poll::Pending;
				};
				let result = std::task::ready!(future.as_mut().poll(cx));
				*sleep = None;
				Poll::Ready(result)
			}
			Wake::Shared { slot, waiting } => {
				std::task::ready!(waiting.as_mut().poll(cx));
				slot.fired();
				*waiting = Box::pin(slot.wake());
				Poll::Ready(Ok(()))
			}
		}
	}

	/// A clock failure terminates the process instead of advancing with untrusted time.
	async fn read_clocks(&mut self) -> Clocks {
		match self.env.clock().observe() {
			Ok(now) => now,
			Err(error) => self.exit_on_clock_error(error).await,
		}
	}

	async fn exit_on_clock_error(&mut self, error: ClockError) -> ! {
		tracing::error!(target: "rivet_state_machine", sm = M::DEBUG_NAME,
            seq = self.seq, %error, "clock failed; exiting process");
		match self.env.exit(1).await {}
	}

	/// In debug builds, invalid state is fatal before any effects are dispatched.
	async fn debug_validate(&mut self) {
		#[cfg(debug_assertions)]
		if let Err(error) = M::debug_validate(&self.state) {
			tracing::error!(machine = M::DEBUG_NAME, %error, "invalid state machine state");
			// EX_SOFTWARE (70): a state machine invariant failed.
			match self.env.exit(70).await {}
		}
	}

	/// In debug builds, a timer index that disagrees with the deadline order is fatal. Every
	/// update keeps one entry per identity in both maps, so a stale or missing index entry
	/// shows up as a length mismatch and the check stays O(1).
	async fn debug_validate_timers(&mut self) {
		#[cfg(debug_assertions)]
		if self.armed.len() != self.armed_at.len() {
			tracing::error!(
				machine = M::DEBUG_NAME,
				armed = self.armed.len(),
				indexed = self.armed_at.len(),
				"runner timer index disagrees with armed timers"
			);
			// EX_SOFTWARE (70): a runner invariant failed.
			match self.env.exit(70).await {}
		}
	}

	/// Apply one transition, record it, then publish before effects can start I/O.
	async fn process_event(&mut self, event: EphemeralStateMachineEvent<M>) {
		let event_name = match &event.body {
			RuntimeEvent::Input(input) => input.name(),
			RuntimeEvent::Timer(timer) => M::timer_name(timer),
		};
		let mut effects = Effects::new(event.now);
		let cost_us = M::simulated_cost_us(&self.state, &event);
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		M::on_event(&mut self.state, event, &mut effects);
		self.metrics
			.observe_transition(event_name, started.elapsed().as_secs_f64());
		self.seq += 1;
		// The state hash serializes the whole state, which costs more than the transition
		// for data-path state machines holding thousands of entries. Debug level keeps it
		// off unless step tracing is explicitly enabled; the macro skips field evaluation.
		tracing::debug!(target: "rivet_state_machine", sm = M::DEBUG_NAME, seq = self.seq,
            event = event_name,
            state_hash = ?TraceHasher::hash(&self.state),
            hasher = TraceHasher::NAME, "state machine step");
		self.debug_validate().await;
		// Simulated processing time passes before the transition's effects leave, as real
		// time does.
		self.spend_simulated_cost(cost_us).await;

		self.publish_snapshot();
		self.dispatch_effects(effects.drain()).await;
		if let Some(busy) = &self.busy {
			busy.record_busy(started.elapsed());
		}
		self.debug_validate_timers().await;
	}

	/// Holds the runner on its clock for simulated processing time, in whole milliseconds,
	/// carrying the remainder to the next event. Nothing is spent when the cost is zero.
	async fn spend_simulated_cost(&mut self, cost_us: u64) {
		if cost_us == 0 {
			return;
		}
		self.cost_debt_us = self.cost_debt_us.saturating_add(cost_us);
		let spend_ms = self.cost_debt_us / 1000;
		if spend_ms == 0 {
			return;
		}
		self.cost_debt_us %= 1000;
		let now = self.read_clocks().await;
		let until = now.boot_ms.saturating_add(spend_ms);
		if let Err(error) = self.env.clock().sleep_until(until).await {
			self.exit_on_clock_error(error).await;
		}
	}

	/// Notify readers only when the visible snapshot changes. Records the `snapshot` phase,
	/// which covers both building and publishing it.
	fn publish_snapshot(&self) {
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		let snapshot = M::snapshot(&self.state);
		if let Some(observer) = &self.observer {
			observer.publish(snapshot, &self.snapshots);
		} else {
			self.snapshots.send_if_modified(|published| {
				if *published == snapshot {
					false
				} else {
					*published = snapshot;
					true
				}
			});
		}
		self.metrics
			.observe_phase(RunnerPhase::Snapshot, started.elapsed().as_secs_f64());
	}

	/// Apply effects in order. Timer identity replaces an earlier arm; domain effects
	/// go to the executor while runtime effects stay with the runner. Timer work and
	/// executor dispatch are recorded as separate runner phases.
	async fn dispatch_effects(&mut self, effects: Vec<EphemeralStateMachineEffect<M>>) {
		let mut timers_seconds = 0.0;
		let mut dispatch_seconds = 0.0;
		for effect in effects {
			#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
			let started = std::time::Instant::now();
			match effect {
				RuntimeEffect::ArmTimer { timer, at_boot_ms } => {
					if let Some(previous) = self.armed_at.insert(timer.clone(), at_boot_ms) {
						self.armed.remove(&(previous, timer.clone()));
					}
					self.armed.insert((at_boot_ms, timer));
					timers_seconds += started.elapsed().as_secs_f64();
				}
				RuntimeEffect::CancelTimer(timer) => {
					if let Some(previous) = self.armed_at.remove(&timer) {
						self.armed.remove(&(previous, timer));
					}
					timers_seconds += started.elapsed().as_secs_f64();
				}
				RuntimeEffect::Exit => {
					tracing::error!(
						machine = M::DEBUG_NAME,
						seq = self.seq,
						"state machine requested fail-stop; exiting process"
					);
					match self.env.exit(1).await {}
				}
				RuntimeEffect::Execute(effect) => {
					let inbox = self.inbox.upgrade().unwrap_or_else(|| {
						// Admitted events still execute during drain. Their completion
						// sink is closed once every producer has left.
						let (inbox, receiver) = mpsc::channel(1);
						drop(receiver);
						inbox
					});
					let ctx = self
						.metrics
						.dispatch(effect.name(), inbox, self.env.clone());
					self.executor.execute(effect, ctx);
					dispatch_seconds += started.elapsed().as_secs_f64();
				}
			}
		}
		self.metrics
			.observe_phase(RunnerPhase::Timers, timers_seconds);
		self.metrics
			.observe_phase(RunnerPhase::Dispatch, dispatch_seconds);
	}
}
