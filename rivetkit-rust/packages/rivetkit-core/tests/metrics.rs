use super::*;

#[path = "metrics_helpers.rs"]
mod metrics_helpers;

mod moved_tests {
	use std::panic::{AssertUnwindSafe, catch_unwind};
	use std::time::Duration;

	use rivet_metrics::prometheus::{IntGauge, Opts, Registry};

	use super::*;
	use super::metrics_helpers::{metric_line_for_actor, render_global_metrics};

	#[test]
	fn duplicate_metric_registration_uses_noop_fallback() {
		let registry = Registry::new();
		let first = IntGauge::with_opts(Opts::new(
			"duplicate_actor_metric",
			"first duplicate metric",
		))
		.expect("first gauge should be valid");
		let second = IntGauge::with_opts(Opts::new(
			"duplicate_actor_metric",
			"second duplicate metric",
		))
		.expect("second gauge should be valid");

		register_metric(&registry, first.clone());
		let result = catch_unwind(AssertUnwindSafe(|| {
			register_metric(&registry, second.clone());
		}));

		assert!(result.is_ok());
		assert_eq!(
			1,
			registry
				.gather()
				.iter()
				.filter(|family| family.name() == "duplicate_actor_metric")
				.count()
		);
	}

	#[test]
	fn actor_startup_duration_metrics_render() {
		let actor_name = "counter-startup";
		let metrics = ActorMetrics::new(actor_name);

		metrics.observe_create_state(Duration::from_millis(10));
		metrics.observe_create_vars(Duration::from_millis(20));

		let rendered = render_global_metrics();
		assert_metric_value(
			&rendered,
			"rivetkit_actor_create_state_duration_seconds_count",
			actor_name,
			"1",
		);
		assert_metric_value(
			&rendered,
			"rivetkit_actor_create_vars_duration_seconds_count",
			actor_name,
			"1",
		);
	}

	#[test]
	fn actor_active_count_tracks_metric_lifetime() {
		let actor_name = "counter-active";
		let metrics = ActorMetrics::new(actor_name);

		let rendered = render_global_metrics();
		let line = rendered
			.lines()
			.find(|line| metric_line_for_actor(line, "rivetkit_actor_active_count", actor_name))
			.expect("active actor count metric should render");
		assert!(line.ends_with('1'), "actor should be active: {line}");

		drop(metrics);

		let rendered = render_global_metrics();
		let line = rendered
			.lines()
			.find(|line| metric_line_for_actor(line, "rivetkit_actor_active_count", actor_name))
			.expect("inactive actor count metric should remain");
		assert!(line.ends_with('0'), "actor should be inactive: {line}");
	}

	fn assert_metric_value(metrics: &str, name: &str, actor_name: &str, value: &str) {
		let line = metrics
			.lines()
			.find(|line| {
				line.starts_with(name)
					&& line.contains(&format!("actor_name=\"{actor_name}\""))
			})
			.unwrap_or_else(|| panic!("{name} should render"));
		assert!(line.ends_with(value), "{name} should have value {value}: {line}");
	}
}
