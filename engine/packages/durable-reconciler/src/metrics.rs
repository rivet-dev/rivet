//! Process-global metric families shared across reconcilers.
use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	pub static ref LOAD: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"state_machine_load_total",
			"Completed state machine load attempts.",
		),
		&["machine", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref LAST_LOAD: GaugeVec = register_gauge_vec_with_registry!(
		Opts::new(
			"state_machine_last_successful_load_timestamp_seconds",
			"Unix timestamp of the last successful loop load.",
		),
		&["machine"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"state_machine_completion_total",
			"Depot effect completion write outcomes.",
		),
		&["machine", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref PROGRESS: GaugeVec = register_gauge_vec_with_registry!(
		Opts::new(
			"state_machine_last_progress_timestamp_seconds",
			"Unix timestamp of last changed-state completion save.",
		),
		&["machine"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

pub(crate) fn loaded(machine: &'static str, success: bool) {
	crate::metrics::LOAD
		.with_label_values(&[machine, if success { "ok" } else { "error" }])
		.inc();
	if success {
		crate::metrics::LAST_LOAD
			.with_label_values(&[machine])
			.set(now());
	}
}

pub(crate) struct Completion {
	machine: &'static str,
	result: &'static str,
}

impl Completion {
	pub(crate) fn start(machine: &'static str) -> Self {
		Self {
			machine,
			result: "error",
		}
	}

	pub(crate) fn confirmed(&mut self, result: &'static str) {
		self.result = result;

		if result == "saved" {
			crate::metrics::PROGRESS
				.with_label_values(&[self.machine])
				.set(now());
		}
	}
}

impl Drop for Completion {
	fn drop(&mut self) {
		crate::metrics::TOTAL
			.with_label_values(&[self.machine, self.result])
			.inc();
	}
}

#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
fn now() -> f64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs_f64()
}
