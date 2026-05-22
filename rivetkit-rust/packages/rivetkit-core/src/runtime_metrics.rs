use std::sync::LazyLock;

use rivet_metrics::prometheus::{
	CounterVec, Gauge, GaugeVec, HistogramOpts, HistogramVec, IntGauge, IntGaugeVec, Opts,
	Registry,
};

const QUANTILE_LABELS: &[&str] = &["quantile"];
const CPU_MODE_LABELS: &[&str] = &["mode"];
const HEAP_STATE_LABELS: &[&str] = &["state"];
const GC_KIND_LABELS: &[&str] = &["kind"];

struct RuntimeMetricCollectors {
	// Event loop quantile snapshot. Periodic gauge populated from
	// `monitorEventLoopDelay()` percentiles every scrape interval. Not a real
	// Prometheus histogram.
	eventloop_lag_seconds: GaugeVec,
	eventloop_utilization: Gauge,
	// Last-heartbeat epoch (ms). JS-side `setInterval(100ms)` updates this; the
	// scraping dashboard computes `now - this` to get heartbeat age.
	eventloop_heartbeat_ts_ms: IntGauge,
	process_cpu_seconds_total: CounterVec,
	process_resident_memory_bytes: IntGauge,
	heap_bytes: IntGaugeVec,
	gc_duration_seconds: HistogramVec,
	active_handles: IntGauge,
	active_requests: IntGauge,
}

static METRICS: LazyLock<RuntimeMetricCollectors> = LazyLock::new(RuntimeMetricCollectors::new);

impl RuntimeMetricCollectors {
	fn new() -> Self {
		let eventloop_lag_seconds = GaugeVec::new(
			Opts::new(
				"rivetkit_js_eventloop_lag_seconds",
				"event loop delay quantile snapshot (seconds) from monitorEventLoopDelay; periodic gauge, not a true histogram",
			),
			QUANTILE_LABELS,
		)
		.expect("create js_eventloop_lag_seconds gauge");
		let eventloop_utilization = Gauge::new(
			"rivetkit_js_eventloop_utilization",
			"event loop utilization fraction (0.0..1.0) from performance.eventLoopUtilization delta",
		)
		.expect("create js_eventloop_utilization gauge");
		let eventloop_heartbeat_ts_ms = IntGauge::new(
			"rivetkit_js_eventloop_heartbeat_ts_ms",
			"epoch millisecond timestamp of last JS-side event loop heartbeat; dashboards compute now - this for age",
		)
		.expect("create js_eventloop_heartbeat_ts_ms gauge");
		let process_cpu_seconds_total = CounterVec::new(
			Opts::new(
				"rivetkit_js_process_cpu_seconds_total",
				"total CPU time consumed by the Node.js process in seconds",
			),
			CPU_MODE_LABELS,
		)
		.expect("create js_process_cpu_seconds_total counter");
		let process_resident_memory_bytes = IntGauge::new(
			"rivetkit_js_process_resident_memory_bytes",
			"Node.js process resident set size in bytes",
		)
		.expect("create js_process_resident_memory_bytes gauge");
		let heap_bytes = IntGaugeVec::new(
			Opts::new(
				"rivetkit_js_heap_bytes",
				"V8 heap size in bytes by state (used|total|limit)",
			),
			HEAP_STATE_LABELS,
		)
		.expect("create js_heap_bytes gauge");
		let gc_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_js_gc_duration_seconds",
				"V8 garbage collection pause duration in seconds by kind",
			)
			.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
			GC_KIND_LABELS,
		)
		.expect("create js_gc_duration_seconds histogram");
		let active_handles = IntGauge::new(
			"rivetkit_js_active_handles",
			"number of active libuv handles from process._getActiveHandles()",
		)
		.expect("create js_active_handles gauge");
		let active_requests = IntGauge::new(
			"rivetkit_js_active_requests",
			"number of active libuv requests from process._getActiveRequests()",
		)
		.expect("create js_active_requests gauge");

		register_metric(&rivet_metrics::REGISTRY, eventloop_lag_seconds.clone());
		register_metric(&rivet_metrics::REGISTRY, eventloop_utilization.clone());
		register_metric(&rivet_metrics::REGISTRY, eventloop_heartbeat_ts_ms.clone());
		register_metric(&rivet_metrics::REGISTRY, process_cpu_seconds_total.clone());
		register_metric(
			&rivet_metrics::REGISTRY,
			process_resident_memory_bytes.clone(),
		);
		register_metric(&rivet_metrics::REGISTRY, heap_bytes.clone());
		register_metric(&rivet_metrics::REGISTRY, gc_duration_seconds.clone());
		register_metric(&rivet_metrics::REGISTRY, active_handles.clone());
		register_metric(&rivet_metrics::REGISTRY, active_requests.clone());

		Self {
			eventloop_lag_seconds,
			eventloop_utilization,
			eventloop_heartbeat_ts_ms,
			process_cpu_seconds_total,
			process_resident_memory_bytes,
			heap_bytes,
			gc_duration_seconds,
			active_handles,
			active_requests,
		}
	}
}

pub fn set_eventloop_lag_quantile(quantile: &str, seconds: f64) {
	METRICS
		.eventloop_lag_seconds
		.with_label_values(&[quantile])
		.set(seconds);
}

pub fn set_eventloop_utilization(value: f64) {
	METRICS.eventloop_utilization.set(value);
}

pub fn set_eventloop_heartbeat_ts_ms(epoch_ms: i64) {
	METRICS.eventloop_heartbeat_ts_ms.set(epoch_ms);
}

pub fn add_process_cpu_seconds(mode: &str, seconds: f64) {
	METRICS
		.process_cpu_seconds_total
		.with_label_values(&[mode])
		.inc_by(seconds);
}

pub fn set_process_resident_memory_bytes(bytes: i64) {
	METRICS.process_resident_memory_bytes.set(bytes);
}

pub fn set_heap_bytes(state: &str, bytes: i64) {
	METRICS.heap_bytes.with_label_values(&[state]).set(bytes);
}

pub fn observe_gc_duration(kind: &str, seconds: f64) {
	METRICS
		.gc_duration_seconds
		.with_label_values(&[kind])
		.observe(seconds);
}

pub fn set_active_handles(count: i64) {
	METRICS.active_handles.set(count);
}

pub fn set_active_requests(count: i64) {
	METRICS.active_requests.set(count);
}

fn register_metric<M>(registry: &Registry, metric: M)
where
	M: rivet_metrics::prometheus::core::Collector + Clone + Send + Sync + 'static,
{
	if let Err(error) = registry.register(Box::new(metric)) {
		tracing::warn!(
			?error,
			"runtime metric registration failed, using existing collector"
		);
	}
}
