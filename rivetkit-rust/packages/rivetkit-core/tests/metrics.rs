use super::*;

#[path = "metrics_helpers.rs"]
mod metrics_helpers;

mod moved_tests {
	use std::panic::{AssertUnwindSafe, catch_unwind};
	use std::time::Duration;

	use rivet_metrics::prometheus::{IntGauge, Opts, Registry};

	use crate::actor::task_types::UserTaskKind;

	use super::metrics_helpers::{metric_line_for_actor, render_global_metrics};
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
	fn actor_startup_duration_metrics_render() {
		let actor_name = "counter-startup";
		let metrics = ActorMetrics::new(actor_name);

		metrics.observe_create_state(Duration::from_millis(10));
		metrics.observe_create_vars(Duration::from_millis(20));
		metrics.observe_startup_phase(
			startup_phase::StartupPhase::RuntimePreamble,
			Some(true),
			"success",
			Duration::from_millis(30),
		);

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
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_startup_phase_duration_seconds_count",
			actor_name,
			&[
				"phase=\"runtime_preamble\"",
				"is_new=\"true\"",
				"outcome=\"success\"",
			],
			"1",
		);
	}

	#[test]
	fn startup_timer_records_total_success_and_error() {
		let success_actor = "counter-startup-total-success";
		let success_metrics = ActorMetrics::new(success_actor);
		let mut success_timer = success_metrics.begin_startup_timer();
		success_timer.set_is_new(true);
		success_timer.finish_success();

		let error_actor = "counter-startup-total-error";
		let error_metrics = ActorMetrics::new(error_actor);
		{
			let mut error_timer = error_metrics.begin_startup_timer();
			error_timer.set_is_new(false);
		}

		let rendered = render_global_metrics();
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_startup_phase_duration_seconds_count",
			success_actor,
			&["phase=\"total\"", "is_new=\"true\"", "outcome=\"success\""],
			"1",
		);
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_startup_phase_duration_seconds_count",
			error_actor,
			&["phase=\"total\"", "is_new=\"false\"", "outcome=\"error\""],
			"1",
		);
	}

	#[cfg(feature = "sqlite-local")]
	#[test]
	fn sqlite_metrics_render_lifecycle_and_startup_kind_labels() {
		let actor_name = "counter-sqlite-labels";
		let metrics = ActorMetrics::new(actor_name);

		metrics.begin_startup();
		metrics.set_startup_is_new(false);
		metrics.set_startup_phase(startup_phase::StartupPhase::RuntimePreamble);
		depot_client::vfs::SqliteVfsMetrics::record_get_pages_request(&metrics, 2, 1, 4096);
		depot_client::vfs::SqliteVfsMetrics::observe_open_phase(
			&metrics,
			depot_client::vfs::SqliteOpenPhase::InitialPreload,
			"success",
			Duration::from_millis(5).as_nanos() as u64,
		);
		depot_client::vfs::SqliteVfsMetrics::record_startup_preload_pages(&metrics, "requested", 2);
		metrics.finish_startup();
		depot_client::vfs::SqliteVfsMetrics::record_get_pages_request(&metrics, 1, 0, 4096);

		let rendered = render_global_metrics();
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_sqlite_vfs_get_pages_total",
			actor_name,
			&[
				"actor_lifecycle_bucket=\"runtime_preamble\"",
				"is_new=\"false\"",
			],
			"1",
		);
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_sqlite_vfs_get_pages_total",
			actor_name,
			&["actor_lifecycle_bucket=\"ready_0_1s\"", "is_new=\"false\""],
			"1",
		);
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_sqlite_open_phase_duration_seconds_count",
			actor_name,
			&[
				"phase=\"initial_preload\"",
				"is_new=\"false\"",
				"outcome=\"success\"",
			],
			"1",
		);
		assert_metric_value_with_labels(
			&rendered,
			"rivetkit_actor_sqlite_startup_preload_pages_total",
			actor_name,
			&["is_new=\"false\"", "kind=\"requested\""],
			"2",
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

	#[test]
	fn actor_current_gauges_aggregate_by_actor_name() {
		let actor_name = "counter-gauge-aggregate";
		let first = ActorMetrics::new(actor_name);
		let second = ActorMetrics::new(actor_name);

		first.set_active_connections(2);
		second.set_active_connections(3);
		first.set_queue_depth(4);
		second.set_queue_depth(5);
		first.set_dispatch_inbox_depth(6);
		second.set_dispatch_inbox_depth(7);
		first.begin_user_task(UserTaskKind::Action);
		second.begin_user_task(UserTaskKind::Action);

		let rendered = render_global_metrics();
		assert_metric_value(
			&rendered,
			"rivetkit_actor_connections_active",
			actor_name,
			"5",
		);
		assert_metric_value(&rendered, "rivetkit_actor_queue_depth", actor_name, "9");
		assert_metric_value_with_label(
			&rendered,
			"rivetkit_actor_inbox_depth",
			actor_name,
			"inbox=\"dispatch\"",
			"13",
		);
		assert_metric_value_with_label(
			&rendered,
			"rivetkit_actor_user_tasks_active",
			actor_name,
			"kind=\"action\"",
			"2",
		);

		first.set_active_connections(1);
		first.end_user_task(UserTaskKind::Action, Duration::from_millis(1));
		drop(first);

		let rendered = render_global_metrics();
		assert_metric_value(
			&rendered,
			"rivetkit_actor_connections_active",
			actor_name,
			"3",
		);
		assert_metric_value(&rendered, "rivetkit_actor_queue_depth", actor_name, "5");
		assert_metric_value_with_label(
			&rendered,
			"rivetkit_actor_user_tasks_active",
			actor_name,
			"kind=\"action\"",
			"1",
		);

		drop(second);

		let rendered = render_global_metrics();
		assert_metric_value(
			&rendered,
			"rivetkit_actor_connections_active",
			actor_name,
			"0",
		);
		assert_metric_value(&rendered, "rivetkit_actor_queue_depth", actor_name, "0");
		assert_metric_value_with_label(
			&rendered,
			"rivetkit_actor_user_tasks_active",
			actor_name,
			"kind=\"action\"",
			"0",
		);
	}

	fn assert_metric_value(metrics: &str, name: &str, actor_name: &str, value: &str) {
		assert_metric_value_with_label(metrics, name, actor_name, "", value);
	}

	fn assert_metric_value_with_label(
		metrics: &str,
		name: &str,
		actor_name: &str,
		label: &str,
		value: &str,
	) {
		let labels = if label.is_empty() {
			Vec::new()
		} else {
			vec![label]
		};
		assert_metric_value_with_labels(metrics, name, actor_name, &labels, value);
	}

	fn assert_metric_value_with_labels(
		metrics: &str,
		name: &str,
		actor_name: &str,
		labels: &[&str],
		value: &str,
	) {
		let line = metrics
			.lines()
			.find(|line| {
				line.starts_with(name)
					&& line.contains(&format!("actor_name=\"{actor_name}\""))
					&& labels.iter().all(|label| line.contains(label))
			})
			.unwrap_or_else(|| panic!("{name} should render"));
		assert!(
			line.ends_with(value),
			"{name} should have value {value}: {line}"
		);
	}
}
