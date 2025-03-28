use std::{env, future::Future, sync::Arc, time::Duration};

use tokio::sync::{Notify, OnceCell};

mod metrics;
mod otel;

static SHUTDOWN: OnceCell<Arc<Notify>> = OnceCell::const_new();

pub fn run<F: Future>(f: F) -> F::Output {
	let notify = Arc::new(Notify::new());
	SHUTDOWN
		.set(notify.clone())
		.expect("more than one runtime running");

	// Build runtime
	let mut rt_builder = build_tokio_runtime_builder();
	let rt = rt_builder.build().expect("failed to build tokio runtime");
	let output = rt.block_on(async move {
		// Must be called from within a tokio context
		let _guard = otel::init_tracing_subscriber();

		tokio::select! {
			_ = notify.notified() => panic!("global shutdown"),
			res = f => res,
		}
	});

	output
}

pub fn shutdown() {
	SHUTDOWN.get().expect("no runtime").notify_one();
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
