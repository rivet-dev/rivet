//! The sampler reads runtime state through a handle rather than builder hooks,
//! so what is worth testing is that it starts from an async caller and produces
//! live values on the metrics endpoint.

use std::time::Duration;

use rivetkit_core::{metrics_endpoint, tokio_runtime_metrics};

fn rendered_metrics() -> String {
	let rendered = metrics_endpoint::render_prometheus_metrics().expect("render metrics");
	String::from_utf8(rendered.body).expect("metrics body is utf8")
}

/// The shared registry prefixes every metric it exports.
const PREFIX: &str = "rivet_";

fn sample_value(body: &str, name: &str) -> Option<f64> {
	let name = format!("{PREFIX}{name}");
	body.lines()
		.find(|line| line.starts_with(&name))
		.and_then(|line| line.rsplit(' ').next())
		.and_then(|value| value.parse().ok())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn samples_runtime_state_from_an_async_caller() {
	tokio_runtime_metrics::ensure_sampler_started();

	// Starting twice must not spawn a second sampler or double count.
	tokio_runtime_metrics::ensure_sampler_started();

	// `interval` fires its first tick immediately, so one yield past the spawn
	// is enough for the first sample.
	tokio::time::sleep(Duration::from_millis(100)).await;

	let body = rendered_metrics();
	let busy_before = sample_value(&body, "tokio_worker_busy_duration_total")
		.expect("busy duration is exported after the first sample");
	assert!(
		sample_value(&body, "tokio_active_task_count").is_some(),
		"active task count is exported",
	);
	assert_eq!(
		body.matches("rivet_tokio_worker_busy_duration_total{")
			.count(),
		2,
		"expected one series per worker thread, and only one sampler writing them",
	);

	// Burn measurable time on a worker so the next sample has to advance.
	tokio::spawn(async {
		let start = std::time::Instant::now();
		while start.elapsed() < Duration::from_millis(50) {
			std::hint::spin_loop();
		}
	})
	.await
	.expect("spin task");

	// The sampler ticks every 5 seconds, so wait past one full interval rather
	// than racing it.
	tokio::time::sleep(Duration::from_secs(6)).await;

	let busy_after = sample_value(&rendered_metrics(), "tokio_worker_busy_duration_total")
		.expect("busy duration is exported on a later sample");
	assert!(
		busy_after > busy_before,
		"workers polled tasks between samples but busy duration did not advance \
		 ({busy_before} -> {busy_after})",
	);
}
