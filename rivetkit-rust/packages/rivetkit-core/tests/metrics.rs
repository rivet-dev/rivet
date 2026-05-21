use super::*;

#[path = "metrics_helpers.rs"]
mod metrics_helpers;

mod moved_tests {
	use std::panic::{AssertUnwindSafe, catch_unwind};

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
	fn missing_label_filter_keeps_unexpected_prometheus_errors() {
		use rivet_metrics::prometheus::Error;

		assert!(is_missing_labels_error(&Error::Msg(
			"missing label values [\"actor\"]".to_owned(),
		)));
		assert!(!is_missing_labels_error(&Error::InconsistentCardinality {
			expect: 3,
			got: 2,
		}));
		assert!(!is_missing_labels_error(&Error::Msg(
			"unexpected metric error".to_owned(),
		)));
	}

	#[test]
	fn actor_inbox_depth_metrics_render() {
		let metrics = ActorMetrics::new("actor-inbox-depth", Some(42), "counter/main", "envoy-1");

		metrics.set_lifecycle_inbox_depth(1);
		metrics.set_dispatch_inbox_depth(2);
		metrics.set_lifecycle_event_inbox_depth(3);

		let rendered = render_global_metrics();
		assert_metric_value(&rendered, "rivet_actor_lifecycle_inbox_depth", "1");
		assert_metric_value(&rendered, "rivet_actor_dispatch_inbox_depth", "2");
		assert_metric_value(&rendered, "rivet_actor_lifecycle_event_inbox_depth", "3");
	}

	#[test]
	fn actor_active_metric_is_retained_after_drop() {
		let metrics = ActorMetrics::new("actor-retention", Some(7), "counter/main", "envoy-1");

		let rendered = render_global_metrics();
		let line = rendered
			.lines()
			.find(|line| metric_line_for_actor(line, "rivet_actor_active", "actor-retention:7"))
			.expect("active actor metric should render");
		assert!(line.ends_with('1'), "actor should be active: {line}");

		drop(metrics);

		let rendered = render_global_metrics();
		let line = rendered
			.lines()
			.find(|line| metric_line_for_actor(line, "rivet_actor_active", "actor-retention:7"))
			.expect("inactive actor metric should remain during retention window");
		assert!(line.ends_with('0'), "actor should be inactive: {line}");
	}

	fn assert_metric_value(metrics: &str, name: &str, value: &str) {
		let line = metrics
			.lines()
			.find(|line| {
				line.starts_with(name)
					&& line.contains("actor_id_gen=\"actor-inbox-depth:42\"")
					&& line.contains("actor_key=\"counter/main\"")
					&& line.contains("envoy_key=\"envoy-1\"")
			})
			.unwrap_or_else(|| panic!("{name} should render"));
		assert!(line.ends_with(value), "{name} should have value {value}: {line}");
	}
}
