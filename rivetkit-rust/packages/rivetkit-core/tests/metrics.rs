use super::*;

mod moved_tests {
	use std::panic::{AssertUnwindSafe, catch_unwind};

	use super::*;

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
	fn actor_inbox_depth_metrics_render() {
		let metrics = ActorMetrics::new("actor-inbox-depth", "metrics");

		metrics.set_lifecycle_inbox_depth(1);
		metrics.set_dispatch_inbox_depth(2);
		metrics.set_lifecycle_event_inbox_depth(3);

		let rendered = metrics.render().expect("metrics should render");
		assert_metric_value(&rendered, "lifecycle_inbox_depth", "1");
		assert_metric_value(&rendered, "dispatch_inbox_depth", "2");
		assert_metric_value(&rendered, "lifecycle_event_inbox_depth", "3");
	}

	fn assert_metric_value(metrics: &str, name: &str, value: &str) {
		let line = metrics
			.lines()
			.find(|line| line.starts_with(name))
			.unwrap_or_else(|| panic!("{name} should render"));
		assert!(line.ends_with(value), "{name} should have value {value}: {line}");
	}
}
