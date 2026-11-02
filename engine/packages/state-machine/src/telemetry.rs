//! Process-wide runner and effect telemetry shared by the ephemeral and durable runners.
//!
//! Labels are bounded: `machine` is a state machine's `DEBUG_NAME`, `event` and `effect` are
//! the `name()` of an event body, timer or effect, `phase` is one of [`RunnerPhase`] and
//! `result` is `ok`, `error` or `dropped`. Every duration uses real time, which is the metrics
//! exception to the environment boundary.

use rivet_env::{Env, TaskHandle, TaskScope};
use rivet_metrics::prometheus::{
	Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts,
	register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
	register_int_gauge_vec_with_registry,
};
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Instant};

lazy_static::lazy_static! {
	static ref TRANSITION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"state_machine_transition_seconds",
			"Time a runner spends inside one state machine transition, by event. Timer events use the timer's name.",
		)
		.buckets(rivet_metrics::TASK_POLL_BUCKETS.to_vec()),
		&["machine", "event"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref RUNNER_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"state_machine_runner_seconds",
			"Runner work per event by phase: the transition, snapshot build and publication, timer arming and cancellation, and executor dispatch.",
		)
		.buckets(rivet_metrics::TASK_POLL_BUCKETS.to_vec()),
		&["machine", "phase"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref EFFECTS_EMITTED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"state_machine_effects_emitted_total",
			"Executor effects dispatched by a runner.",
		),
		&["machine", "effect"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref EFFECT_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"state_machine_effect_seconds",
			"Time from effect dispatch until its work finished. Synchronous effects measure the execute call; spawned and handed-off effects measure until their result. Dropped work never finished.",
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		&["machine", "effect", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref INBOX_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"state_machine_inbox_depth",
			"Inputs queued in ephemeral runner inboxes, summed over every runner of the machine and sampled each time a runner takes an input.",
		),
		&["machine"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref EFFECTS_IN_FLIGHT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"state_machine_effects_in_flight",
			"Executor effects dispatched whose work has not finished yet.",
		),
		&["machine", "effect"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

/// Runner work measured by `state_machine_runner_seconds`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnerPhase {
	/// The state machine transition itself.
	Transition,
	/// Building and publishing the snapshot.
	Snapshot,
	/// Arming and cancelling timers.
	Timers,
	/// Calling the executor for each effect.
	Dispatch,
}

impl RunnerPhase {
	fn label(self) -> &'static str {
		match self {
			RunnerPhase::Transition => "transition",
			RunnerPhase::Snapshot => "snapshot",
			RunnerPhase::Timers => "timers",
			RunnerPhase::Dispatch => "dispatch",
		}
	}
}

/// Metric handles for one effect name, shared by every in-flight record of that effect.
struct EffectHandles {
	emitted: IntCounter,
	in_flight: IntGauge,
	ok: Histogram,
	error: Histogram,
	dropped: Histogram,
}

/// Label handles of one runner, resolved once per name so recording an event or effect does
/// no shared label lookup after the first time.
pub struct RunnerMetrics {
	machine: &'static str,
	transition: Histogram,
	snapshot: Histogram,
	timers: Histogram,
	dispatch: Histogram,
	events: BTreeMap<&'static str, Histogram>,
	effects: BTreeMap<&'static str, Arc<EffectHandles>>,
	/// This runner's share of `state_machine_inbox_depth`, resolved on first use.
	inbox_depth: Option<(IntGauge, i64)>,
}

impl RunnerMetrics {
	pub fn new(machine: &'static str) -> Self {
		let phase =
			|phase: RunnerPhase| RUNNER_SECONDS.with_label_values(&[machine, phase.label()]);
		Self {
			machine,
			transition: phase(RunnerPhase::Transition),
			snapshot: phase(RunnerPhase::Snapshot),
			timers: phase(RunnerPhase::Timers),
			dispatch: phase(RunnerPhase::Dispatch),
			events: BTreeMap::new(),
			effects: BTreeMap::new(),
			inbox_depth: None,
		}
	}

	/// Records how many inputs remain queued in this runner's inbox. Runners of one machine
	/// share the gauge, so each contributes the change since its last sample.
	pub fn observe_inbox_depth(&mut self, depth: usize) {
		let machine = self.machine;
		let (gauge, last) = self
			.inbox_depth
			.get_or_insert_with(|| (INBOX_DEPTH.with_label_values(&[machine]), 0));
		let depth = i64::try_from(depth).unwrap_or(i64::MAX);
		gauge.add(depth - *last);
		*last = depth;
	}

	/// Records one transition under its event name and the `transition` runner phase.
	pub fn observe_transition(&mut self, event: &'static str, seconds: f64) {
		let machine = self.machine;
		self.events
			.entry(event)
			.or_insert_with(|| TRANSITION_SECONDS.with_label_values(&[machine, event]))
			.observe(seconds);
		self.transition.observe(seconds);
	}

	/// Records runner work outside the transition. Use [`Self::observe_transition`] for it.
	pub fn observe_phase(&self, phase: RunnerPhase, seconds: f64) {
		match phase {
			RunnerPhase::Transition => self.transition.observe(seconds),
			RunnerPhase::Snapshot => self.snapshot.observe(seconds),
			RunnerPhase::Timers => self.timers.observe(seconds),
			RunnerPhase::Dispatch => self.dispatch.observe(seconds),
		}
	}

	/// Counts one dispatched effect and starts its in-flight record. The record finishes when
	/// the returned context, or the work it is handed to, completes or is dropped.
	pub fn dispatch<I>(
		&mut self,
		effect: &'static str,
		inbox: I,
		env: Arc<dyn Env>,
	) -> EffectContext<I> {
		let machine = self.machine;
		let handles = self
			.effects
			.entry(effect)
			.or_insert_with(|| {
				let result =
					|result: &str| EFFECT_SECONDS.with_label_values(&[machine, effect, result]);
				Arc::new(EffectHandles {
					emitted: EFFECTS_EMITTED.with_label_values(&[machine, effect]),
					in_flight: EFFECTS_IN_FLIGHT.with_label_values(&[machine, effect]),
					ok: result("ok"),
					error: result("error"),
					dropped: result("dropped"),
				})
			})
			.clone();
		handles.emitted.inc();
		handles.in_flight.inc();
		EffectContext {
			inbox,
			env,
			completion: Some(EffectCompletion {
				handles,
				#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
				dispatched: Instant::now(),
				finished: false,
			}),
		}
	}
}

impl Drop for RunnerMetrics {
	/// Withdraws this runner's inbox depth, so a stopped runner leaves nothing in the gauge.
	fn drop(&mut self) {
		if let Some((gauge, last)) = &self.inbox_depth {
			gauge.sub(*last);
		}
	}
}

/// What an executor gets with each effect: the runner's inbox and `Env`, plus the effect's
/// telemetry record.
///
/// The record finishes in one of three ways. Dropping an unused context records a synchronous
/// effect as `ok`, timed until the drop, so an effect handled inside `execute` needs no code.
/// [`Self::spawn`] and its variants time spawned work until its future finishes, recording
/// `ok` or `error` from its output, or `dropped` if the future is dropped first.
/// [`Self::complete_later`] hands the record to work queued on another long-running task.
pub struct EffectContext<I> {
	inbox: I,
	env: Arc<dyn Env>,
	/// Taken by the methods that move the record into spawned or queued work.
	completion: Option<EffectCompletion>,
}

impl<I> EffectContext<I> {
	/// Where completion events go. A durable runner has no inbox and passes `()`.
	pub fn inbox(&self) -> &I {
		&self.inbox
	}

	pub fn env(&self) -> &Arc<dyn Env> {
		&self.env
	}

	/// Spawns `fut` on the `Env` and records its outcome when it finishes.
	#[track_caller]
	pub fn spawn<F>(self, fut: F) -> TaskHandle
	where
		F: Future + Send + 'static,
		F::Output: EffectOutcome,
	{
		let env = self.env.clone();
		let tracked = self.track(fut);
		env.spawn(Box::pin(async move {
			tracked.await;
		}))
	}

	/// Spawns `fut` in `scope`, so the scope owns its cancellation, and records its outcome.
	#[track_caller]
	pub fn spawn_in<F>(self, scope: &TaskScope, fut: F) -> TaskHandle
	where
		F: Future + Send + 'static,
		F::Output: EffectOutcome,
	{
		let tracked = self.track(fut);
		scope.spawn(async move {
			tracked.await;
		})
	}

	/// Wraps `fut` so it records the effect's outcome, for work the executor spawns or awaits
	/// some other way.
	pub fn track<F>(self, fut: F) -> impl Future<Output = F::Output> + Send + 'static
	where
		F: Future + Send + 'static,
		F::Output: EffectOutcome,
	{
		let completion = self.complete_later();
		async move {
			let output = fut.await;
			completion.finish(&output);
			output
		}
	}

	/// Hands the record to work queued on another long-running task, such as a disk I/O queue.
	/// The completion travels with the queued item; see [`EffectCompletion`].
	pub fn complete_later(mut self) -> EffectCompletion {
		self.completion
			.take()
			.expect("an effect context owns its completion until it is consumed")
	}
}

impl<I> Drop for EffectContext<I> {
	fn drop(&mut self) {
		if let Some(completion) = self.completion.take() {
			completion.finish(&());
		}
	}
}

/// The telemetry record of one effect whose work finishes on another task.
/// [`Self::finish`] records the outcome; dropping it unfinished records `dropped`.
pub struct EffectCompletion {
	handles: Arc<EffectHandles>,
	dispatched: Instant,
	finished: bool,
}

impl EffectCompletion {
	/// Records `ok` or `error` from `result` and the time since dispatch.
	pub fn finish(mut self, result: &impl EffectOutcome) {
		let histogram = if result.is_ok() {
			&self.handles.ok
		} else {
			&self.handles.error
		};
		histogram.observe(self.dispatched.elapsed().as_secs_f64());
		self.handles.in_flight.dec();
		self.finished = true;
	}
}

impl Drop for EffectCompletion {
	fn drop(&mut self) {
		if !self.finished {
			self.handles
				.dropped
				.observe(self.dispatched.elapsed().as_secs_f64());
			self.handles.in_flight.dec();
		}
	}
}

/// Classifies effect work as `ok` or `error`. Work that reports its result as an event and
/// returns `()` counts as `ok`.
pub trait EffectOutcome {
	fn is_ok(&self) -> bool;
}

impl EffectOutcome for () {
	fn is_ok(&self) -> bool {
		true
	}
}

impl<T, E> EffectOutcome for Result<T, E> {
	fn is_ok(&self) -> bool {
		Result::is_ok(self)
	}
}
