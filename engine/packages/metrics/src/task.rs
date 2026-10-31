//! Scheduling telemetry for long-lived async tasks, wrapping `std::future::Future` and
//! `std::task::Wake`.
//!
//! A serial task such as a state machine runner can be slow for two different reasons: its
//! own work takes the time, or it is runnable but waits for a Tokio worker because the
//! runtime is saturated. `rivet_task_poll_seconds_total` measures the first and
//! `rivet_task_schedule_delay_seconds` the second, from the wake that made the task runnable
//! to the start of its next poll. Both use real time, which is the metrics exception to the
//! environment boundary.

use crate::{MICRO_BUCKETS, REGISTRY};
use parking_lot::Mutex;
use prometheus::{
	CounterVec, Histogram, HistogramOpts, HistogramVec, Opts, core::AtomicF64,
	core::GenericCounter, register_counter_vec_with_registry, register_histogram_vec_with_registry,
};
use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	task::{Context, Poll, Wake, Waker},
	time::Instant,
};

lazy_static::lazy_static! {
	static ref TASK_POLL_SECONDS: CounterVec = register_counter_vec_with_registry!(
		Opts::new(
			"task_poll_seconds_total",
			"Wall time a monitored task spent inside its own polls, summed over every task with that name. Its rate is the task's busy fraction of one worker.",
		),
		&["task"],
		*REGISTRY
	).unwrap();

	static ref TASK_SCHEDULE_DELAY_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"task_schedule_delay_seconds",
			"Time from a wake of a monitored task until its next poll started, while it was runnable but waiting for a Tokio worker. The rate of the sum is the fraction of time the task was starved by the runtime rather than busy or idle.",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		&["task"],
		*REGISTRY
	).unwrap();
}

/// Wraps `future` so its polls and scheduling delay are recorded under `task`. `task` must be
/// a bounded, code-defined name. Use it for long-lived tasks whose throughput matters; the
/// wrapper costs a lock and two clock reads per poll.
pub fn monitor_task<F: Future>(task: &'static str, future: F) -> MonitoredTask<F> {
	#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
	let origin = Instant::now();
	let recorder = Arc::new(WakeRecorder {
		origin,
		woken_at_ns: AtomicU64::new(0),
		inner: Mutex::new(None),
	});
	MonitoredTask {
		future: Box::pin(future),
		waker: Waker::from(recorder.clone()),
		recorder,
		poll_seconds: TASK_POLL_SECONDS.with_label_values(&[task]),
		schedule_delay: TASK_SCHEDULE_DELAY_SECONDS.with_label_values(&[task]),
	}
}

/// A future wrapped by [`monitor_task`].
pub struct MonitoredTask<F> {
	future: Pin<Box<F>>,
	/// Handed to the inner future on every poll, so its wakes pass through the recorder.
	waker: Waker,
	recorder: Arc<WakeRecorder>,
	poll_seconds: GenericCounter<AtomicF64>,
	schedule_delay: Histogram,
}

/// Records when the task was first woken since its last poll, then forwards the wake.
struct WakeRecorder {
	origin: Instant,
	/// Nanoseconds after `origin`, plus one so zero can mean "not woken".
	woken_at_ns: AtomicU64,
	/// The waker of the task that polls the wrapper. Forced-sync: wakes run outside async code.
	inner: Mutex<Option<Waker>>,
}

impl WakeRecorder {
	fn now_ns(&self) -> u64 {
		u64::try_from(self.origin.elapsed().as_nanos())
			.unwrap_or(u64::MAX - 1)
			.saturating_add(1)
	}
}

impl Wake for WakeRecorder {
	fn wake(self: Arc<Self>) {
		self.wake_by_ref();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		// Only the first wake since the last poll starts the delay; later ones change nothing.
		let _ = self.woken_at_ns.compare_exchange(
			0,
			self.now_ns(),
			Ordering::Relaxed,
			Ordering::Relaxed,
		);
		let inner = self.inner.lock().clone();
		if let Some(inner) = inner {
			inner.wake();
		}
	}
}

impl<F: Future> Future for MonitoredTask<F> {
	type Output = F::Output;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
		let this = self.get_mut();
		{
			let mut inner = this.recorder.inner.lock();
			let current = inner
				.as_ref()
				.is_some_and(|waker| waker.will_wake(cx.waker()));
			if !current {
				*inner = Some(cx.waker().clone());
			}
		}

		// A wake before the first poll, or none at all, leaves no delay to record.
		let started_ns = this.recorder.now_ns();
		let woken_ns = this.recorder.woken_at_ns.swap(0, Ordering::Relaxed);
		if woken_ns != 0 {
			let delay_ns = started_ns.saturating_sub(woken_ns);
			this.schedule_delay.observe(delay_ns as f64 / 1e9);
		}

		let mut task_cx = Context::from_waker(&this.waker);
		let result = this.future.as_mut().poll(&mut task_cx);

		let finished_ns = this.recorder.now_ns();
		this.poll_seconds
			.inc_by(finished_ns.saturating_sub(started_ns) as f64 / 1e9);
		result
	}
}
