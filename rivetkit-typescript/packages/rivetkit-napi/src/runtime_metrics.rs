use napi_derive::napi;
use rivetkit_core::runtime_metrics;

#[napi]
pub fn js_set_eventloop_lag_quantile(quantile: String, seconds: f64) {
	runtime_metrics::set_eventloop_lag_quantile(&quantile, seconds);
}

#[napi]
pub fn js_set_eventloop_utilization(value: f64) {
	runtime_metrics::set_eventloop_utilization(value);
}

#[napi]
pub fn js_set_eventloop_heartbeat_ts_ms(epoch_ms: i64) {
	runtime_metrics::set_eventloop_heartbeat_ts_ms(epoch_ms);
}

#[napi]
pub fn js_add_process_cpu_seconds(mode: String, seconds: f64) {
	runtime_metrics::add_process_cpu_seconds(&mode, seconds);
}

#[napi]
pub fn js_set_process_resident_memory_bytes(bytes: i64) {
	runtime_metrics::set_process_resident_memory_bytes(bytes);
}

#[napi]
pub fn js_set_heap_bytes(state: String, bytes: i64) {
	runtime_metrics::set_heap_bytes(&state, bytes);
}

#[napi]
pub fn js_observe_gc_duration(kind: String, seconds: f64) {
	runtime_metrics::observe_gc_duration(&kind, seconds);
}

#[napi]
pub fn js_set_active_handles(count: i64) {
	runtime_metrics::set_active_handles(count);
}

#[napi]
pub fn js_set_active_requests(count: i64) {
	runtime_metrics::set_active_requests(count);
}
