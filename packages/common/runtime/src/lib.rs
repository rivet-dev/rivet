use std::{env, future::Future, sync::Arc, time::Duration};

use tokio::sync::{Notify, OnceCell};

mod metrics;
mod otel;

static SHUTDOWN: OnceCell<Arc<Notify>> = OnceCell::const_new();

/// Returns `None` if the runtime was shut down manually.
pub fn run<F: Future>(f: F) -> Option<F::Output> {
	// Build runtime
	let mut rt_builder = build_tokio_runtime_builder();
	let rt = rt_builder.build().expect("failed to build tokio runtime");
	let output = rt.block_on(async move {
		let notify = SHUTDOWN
			.get_or_init(|| std::future::ready(Arc::new(Notify::new())))
			.await
			.clone();

		// Must be called from within a tokio context
		let _guard = otel::init_tracing_subscriber();

		tokio::select! {
			_ = notify.notified() => {
				tracing::info!("shutting down runtime");
				None
			},
			res = f => Some(res),
		}
	});

	output
}

/// Shuts down the entire rivet runtime, if one is running. This future will never resolve.
pub async fn shutdown() {
	if let Some(shutdown) = SHUTDOWN.get() {
		shutdown.notify_one();
	} else {
		tracing::error!("no runtime to shutdown");
	};

	// Wait forever, the runtime should be shutting down
	std::future::pending().await
}

/// Shuts down the entire rivet runtime, if one is running. Immediately panics afterwards.
pub fn sync_shutdown() {
	if let Some(shutdown) = SHUTDOWN.get() {
		shutdown.notify_one();
	} else {
		tracing::error!("no runtime to shutdown");
	};

	panic!("shutting down");
}

fn build_tokio_runtime_builder() -> tokio::runtime::Builder {
	let mut rt_builder = tokio::runtime::Builder::new_multi_thread();
	rt_builder.enable_all();

	rt_builder.on_thread_start(move || {
		metrics::TOKIO_THREAD_COUNT.inc();
	});
	rt_builder.on_thread_stop(move || {
		metrics::TOKIO_THREAD_COUNT.dec();
	});

	if env::var("TOKIO_RUNTIME_METRICS").is_ok() {
		rt_builder.on_before_task_poll(|_| {
			let metrics = tokio::runtime::Handle::current().metrics();
			let buckets = metrics.poll_time_histogram_num_buckets();

			metrics::TOKIO_GLOBAL_QUEUE_DEPTH.set(metrics.global_queue_depth() as i64);

			for worker in 0..metrics.num_workers() {
				metrics::TOKIO_WORKER_OVERFLOW_COUNT
					.with_label_values(&[&worker.to_string()])
					.set(metrics.worker_overflow_count(worker) as i64);
				metrics::TOKIO_WORKER_LOCAL_QUEUE_DEPTH
					.with_label_values(&[&worker.to_string()])
					.set(metrics.worker_local_queue_depth(worker) as i64);

				use rivet_metrics::prometheus::core::Metric;
				// Has some sort of internal lock, must read data before loop
				let prom_buckets = {
					metrics::TOKIO_TASK_POLL_DURATION
						.metric()
						.get_histogram()
						.get_bucket()
						.iter()
						.map(|bucket| bucket.get_cumulative_count())
						.collect::<Vec<_>>()
				};

				for (bucket, prom_bucket_count) in (0..buckets).zip(prom_buckets) {
					let range = metrics.poll_time_histogram_bucket_range(bucket);
					let count = metrics.poll_time_histogram_bucket_count(worker, bucket);
					let diff = count.saturating_sub(prom_bucket_count);

					for _ in 0..diff {
						metrics::TOKIO_TASK_POLL_DURATION.observe(range.start.as_secs_f64());
					}
				}
			}
		});
		
		rt_builder.metrics_poll_time_histogram_configuration(
			tokio::runtime::HistogramConfiguration::log(
				tokio::runtime::LogHistogram::builder()
					.min_value(Duration::from_micros(20))
					.max_value(Duration::from_millis(32))
					.precision_exact(0)
					.max_buckets(20)
					.unwrap(),
			),
		);
		rt_builder.enable_metrics_poll_time_histogram();
	}

	if let Ok(thread_stack_size) = env::var("TOKIO_THREAD_STACK_SIZE") {
		rt_builder.thread_stack_size(thread_stack_size.parse().unwrap());
	} else {
		// async-nats requires a fat stack
		rt_builder.thread_stack_size(8 * 1024 * 1024);
	}

	if let Ok(worker_threads) = env::var("TOKIO_WORKER_THREADS") {
		rt_builder.worker_threads(worker_threads.parse().unwrap());
	}

	if let Ok(max_blocking_threads) = env::var("TOKIO_MAX_BLOCKING_THREADS") {
		rt_builder.max_blocking_threads(max_blocking_threads.parse().unwrap());
	}

	if let Ok(global_queue_interval) = env::var("TOKIO_GLOBAL_QUEUE_INTERVAL") {
		rt_builder.global_queue_interval(global_queue_interval.parse().unwrap());
	}

	if let Ok(event_interval) = env::var("TOKIO_EVENT_INTERVAL") {
		rt_builder.event_interval(event_interval.parse().unwrap());
	}

	if let Ok(thread_keep_alive) = env::var("TOKIO_THREAD_KEEP_ALIVE") {
		rt_builder.thread_keep_alive(Duration::from_millis(thread_keep_alive.parse().unwrap()));
	}

	rt_builder
}
