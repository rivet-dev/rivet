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

	#[cfg(feature = "sqlite")]
	#[test]
	fn sqlite_read_pool_metrics_render() {
		use rivetkit_sqlite::vfs::SqliteVfsMetrics;

		let metrics = ActorMetrics::new("actor-1", "test");
		metrics.set_read_pool_active_readers(2);
		metrics.set_read_pool_idle_readers(1);
		metrics.observe_read_pool_read_wait(std::time::Duration::from_millis(3));
		metrics.observe_read_pool_write_wait(std::time::Duration::from_millis(5));
		metrics.record_read_pool_routed_read_query();
		metrics.record_read_pool_write_fallback_query();
		metrics.observe_read_pool_manual_transaction(std::time::Duration::from_millis(7));
		metrics.record_read_pool_reader_open();
		metrics.record_read_pool_reader_close(1);
		metrics.record_read_pool_rejected_reader_mutation();
		metrics.record_read_pool_mode_transition("read", "write");

		let output = metrics.render().expect("metrics should render");
		for name in [
			"sqlite_read_pool_active_readers",
			"sqlite_read_pool_idle_readers",
			"sqlite_read_pool_read_wait_duration_seconds",
			"sqlite_read_pool_write_wait_duration_seconds",
			"sqlite_read_pool_routed_read_queries_total",
			"sqlite_read_pool_write_fallback_queries_total",
			"sqlite_read_pool_manual_transaction_duration_seconds",
			"sqlite_read_pool_reader_opens_total",
			"sqlite_read_pool_reader_closes_total",
			"sqlite_read_pool_rejected_reader_mutations_total",
			"sqlite_read_pool_mode_transitions_total",
		] {
			assert!(output.contains(name), "missing metric {name}");
		}
	}
}
