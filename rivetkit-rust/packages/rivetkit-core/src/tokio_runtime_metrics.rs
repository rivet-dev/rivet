//! Tokio runtime metrics for the actor pod.
//!
//! RivetKit does not build its own runtime. It runs on whichever one N-API
//! created for it, so runtime state has to be sampled through a handle taken
//! from a call that is already on that runtime. That rules out the builder
//! hooks the engine uses for thread and task lifecycle counts, but everything
//! below is readable from a handle alone.
//!
//! Names match the engine's runtime metrics so one dashboard panel works for
//! both. The engine has its own copy of these definitions because the two
//! processes collect them differently.

use std::{
	sync::{
		LazyLock,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use rivet_metrics::{
	REGISTRY,
	prometheus::{
		CounterVec, IntGauge, register_counter_vec_with_registry, register_int_gauge_with_registry,
	},
};
#[cfg(tokio_unstable)]
use rivet_metrics::prometheus::{IntGaugeVec, register_int_gauge_vec_with_registry};

/// How often runtime state is read. Everything sampled here is either an
/// instantaneous depth or a monotonic total, so the interval only needs to be
/// finer than the scrape interval.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

const WORKER_LABELS: &[&str] = &["worker"];

static GLOBAL_QUEUE_DEPTH: LazyLock<IntGauge> = LazyLock::new(|| {
	register_int_gauge_with_registry!(
		"tokio_global_queue_depth",
		"Number of pending tasks in the global queue.",
		*REGISTRY
	)
	.unwrap()
});

#[cfg(tokio_unstable)]
static BLOCKING_QUEUE_DEPTH: LazyLock<IntGauge> = LazyLock::new(|| {
	register_int_gauge_with_registry!(
		"tokio_blocking_queue_depth",
		"Number of tasks queued for the blocking thread pool.",
		*REGISTRY
	)
	.unwrap()
});

static ACTIVE_TASK_COUNT: LazyLock<IntGauge> = LazyLock::new(|| {
	register_int_gauge_with_registry!(
		"tokio_active_task_count",
		"Total number of active (running or sleeping) tasks.",
		*REGISTRY
	)
	.unwrap()
});

#[cfg(tokio_unstable)]
static BLOCKING_THREAD_COUNT: LazyLock<IntGauge> = LazyLock::new(|| {
	register_int_gauge_with_registry!(
		"tokio_blocking_thread_count",
		"Number of threads in the blocking thread pool.",
		*REGISTRY
	)
	.unwrap()
});

#[cfg(tokio_unstable)]
static IDLE_BLOCKING_THREAD_COUNT: LazyLock<IntGauge> = LazyLock::new(|| {
	register_int_gauge_with_registry!(
		"tokio_idle_blocking_thread_count",
		"Number of idle threads in the blocking thread pool.",
		*REGISTRY
	)
	.unwrap()
});

#[cfg(tokio_unstable)]
static WORKER_LOCAL_QUEUE_DEPTH: LazyLock<IntGaugeVec> = LazyLock::new(|| {
	register_int_gauge_vec_with_registry!(
		"tokio_worker_local_queue_depth",
		"Number of pending tasks in a worker's queue.",
		WORKER_LABELS,
		*REGISTRY
	)
	.unwrap()
});

#[cfg(tokio_unstable)]
static WORKER_OVERFLOW_COUNT: LazyLock<IntGaugeVec> = LazyLock::new(|| {
	register_int_gauge_vec_with_registry!(
		"tokio_worker_overflow_count",
		"Number of times the given worker thread saturated its local queue.",
		WORKER_LABELS,
		*REGISTRY
	)
	.unwrap()
});

static WORKER_BUSY_DURATION_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
	register_counter_vec_with_registry!(
		"tokio_worker_busy_duration_total",
		"Seconds a worker has spent polling tasks. Its rate against wall clock time is the worker's busy fraction, which separates a saturated runtime from one whose workers are parked.",
		WORKER_LABELS,
		*REGISTRY
	)
	.unwrap()
});

static SAMPLER_STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the sampler the first time this is reached from inside a runtime.
///
/// Synchronous entry points, such as the N-API `metrics()` binding, run on the
/// JS thread and have no runtime to sample, so this retries rather than latching
/// through a `Once`.
pub fn ensure_sampler_started() {
	if SAMPLER_STARTED.load(Ordering::Relaxed) {
		return;
	}

	let Ok(handle) = tokio::runtime::Handle::try_current() else {
		return;
	};

	if !SAMPLER_STARTED.swap(true, Ordering::Relaxed) {
		handle.spawn(sample(handle.clone()));
	}
}

async fn sample(handle: tokio::runtime::Handle) {
	let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
	interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

	// Tokio reports a cumulative busy duration per worker while a prometheus
	// counter only advances by a delta.
	let mut last_busy_nanos = vec![0u64; handle.metrics().num_workers()];

	loop {
		interval.tick().await;

		let metrics = handle.metrics();

		GLOBAL_QUEUE_DEPTH.set(metrics.global_queue_depth() as i64);
		ACTIVE_TASK_COUNT.set(metrics.num_alive_tasks() as i64);
		#[cfg(tokio_unstable)]
		{
			BLOCKING_QUEUE_DEPTH.set(metrics.blocking_queue_depth() as i64);
			BLOCKING_THREAD_COUNT.set(metrics.num_blocking_threads() as i64);
			IDLE_BLOCKING_THREAD_COUNT.set(metrics.num_idle_blocking_threads() as i64);
		}

		for worker in 0..metrics.num_workers() {
			let label = worker.to_string();
			let labels = [label.as_str()];

			#[cfg(tokio_unstable)]
			{
				WORKER_LOCAL_QUEUE_DEPTH
					.with_label_values(&labels)
					.set(metrics.worker_local_queue_depth(worker) as i64);
				WORKER_OVERFLOW_COUNT
					.with_label_values(&labels)
					.set(metrics.worker_overflow_count(worker) as i64);
			}

			let Some(last_busy_nanos) = last_busy_nanos.get_mut(worker) else {
				continue;
			};
			let busy_nanos = metrics.worker_total_busy_duration(worker).as_nanos() as u64;
			let delta_nanos = busy_nanos.saturating_sub(*last_busy_nanos);
			*last_busy_nanos = busy_nanos;
			WORKER_BUSY_DURATION_TOTAL
				.with_label_values(&labels)
				.inc_by(delta_nanos as f64 / 1_000_000_000.0);
		}
	}
}
