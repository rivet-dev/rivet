use rivet_config::{
	Config, DynamicConfigUpdate,
	config::{Auth, Root, Sqlite},
	secret::Secret,
};

fn config_with_pull_limits() -> Config {
	Config::from_root(
		serde_json::from_value::<Root>(serde_json::json!({
			"auth": { "admin_token": "default" },
			"runtime": {
				"worker_poll_interval_ms": 8000,
				"worker_max_deduped_workflows_per_pull": 100,
				"worker_max_workflows_per_pull": 50,
				"worker_max_wake_condition_clears_per_pull": 25
			}
		}))
		.expect("valid config"),
	)
}

#[test]
fn pull_limits_set_clear_and_notify() {
	let config = config_with_pull_limits();
	let receiver = config.dynamic_watch();

	config
		.apply_dynamic(&DynamicConfigUpdate {
			worker_poll_interval_ms: Some(Some(4000)),
			worker_max_deduped_workflows_per_pull: Some(Some(300)),
			worker_max_workflows_per_pull: Some(Some(150)),
			worker_max_wake_condition_clears_per_pull: Some(Some(125)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	let observed = {
		let snapshot = receiver.borrow();
		(
			snapshot.runtime.worker_poll_interval().as_millis(),
			snapshot.runtime.worker_max_deduped_workflows_per_pull(),
			snapshot.runtime.worker_max_workflows_per_pull(),
			snapshot.runtime.worker_max_wake_condition_clears_per_pull(),
		)
	};
	assert_eq!(observed, (4000, 300, 150, 125));

	config
		.apply_dynamic(&DynamicConfigUpdate {
			worker_poll_interval_ms: Some(None),
			worker_max_deduped_workflows_per_pull: Some(None),
			worker_max_workflows_per_pull: Some(None),
			worker_max_wake_condition_clears_per_pull: Some(None),
			..DynamicConfigUpdate::default()
		})
		.expect("valid clears");

	let snapshot = config.dynamic();
	assert_eq!(snapshot.runtime.worker_poll_interval().as_millis(), 8000);
	assert_eq!(
		snapshot.runtime.worker_max_deduped_workflows_per_pull(),
		100
	);
	assert_eq!(snapshot.runtime.worker_max_workflows_per_pull(), 50);
	assert_eq!(
		snapshot.runtime.worker_max_wake_condition_clears_per_pull(),
		25
	);
}

#[test]
fn pull_limits_reject_zero_without_publishing() {
	let config = config_with_pull_limits();
	let before = config.dynamic();

	for update in [
		DynamicConfigUpdate {
			worker_poll_interval_ms: Some(Some(0)),
			..Default::default()
		},
		DynamicConfigUpdate {
			worker_max_deduped_workflows_per_pull: Some(Some(0)),
			..Default::default()
		},
		DynamicConfigUpdate {
			worker_max_workflows_per_pull: Some(Some(0)),
			..Default::default()
		},
		DynamicConfigUpdate {
			worker_max_wake_condition_clears_per_pull: Some(Some(0)),
			..Default::default()
		},
	] {
		config
			.apply_dynamic(&update)
			.expect_err("zero must be rejected");
		assert!(std::sync::Arc::ptr_eq(&before, &config.dynamic()));
	}
}

fn config_with_admission_percent(percent: f64) -> Config {
	Config::from_root(Root {
		auth: Some(Auth {
			admin_token: Secret::new("default".to_owned()),
			jwt: Default::default(),
		}),
		sqlite: Some(Sqlite {
			compaction_admission_percent: Some(percent),
			..Sqlite::default()
		}),
		..Root::default()
	})
}

#[test]
fn reading_a_property_dynamically_is_opt_in() {
	let config = config_with_admission_percent(10.0);

	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(80.0)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	// Dereferencing keeps serving the value loaded at startup, so an existing call site cannot pick
	// up a runtime change by accident.
	assert_eq!(config.sqlite().compaction_admission_percent, Some(10.0));
	assert_eq!(
		config.dynamic().sqlite().compaction_admission_percent,
		Some(80.0)
	);
}

#[test]
fn clearing_a_property_reverts_to_the_loaded_value() {
	let config = config_with_admission_percent(10.0);

	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(80.0)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");
	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(None),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	assert_eq!(
		config.dynamic().sqlite().compaction_admission_percent,
		Some(10.0)
	);
}

#[test]
fn an_absent_property_leaves_the_current_value_alone() {
	let config = config_with_admission_percent(10.0);

	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(80.0)),
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");
	config
		.apply_dynamic(&DynamicConfigUpdate {
			..DynamicConfigUpdate::default()
		})
		.expect("valid update");

	let dynamic = config.dynamic();
	assert_eq!(dynamic.sqlite().compaction_admission_percent, Some(80.0));
}

#[test]
fn an_invalid_update_is_rejected_and_changes_nothing() {
	let config = config_with_admission_percent(10.0);

	// The update goes through the same validation as a config loaded from disk.
	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_admission_percent: Some(Some(150.0)),
			..DynamicConfigUpdate::default()
		})
		.expect_err("percent above 100 must be rejected");
	config
		.apply_dynamic(&DynamicConfigUpdate {
			compaction_write_bytes_per_second: Some(Some(0)),
			..DynamicConfigUpdate::default()
		})
		.expect_err("a zero write budget stalls compaction and must be rejected");

	assert_eq!(
		config.dynamic().sqlite().compaction_admission_percent,
		Some(10.0)
	);
}

#[test]
fn an_update_message_round_trips_as_json() {
	let update = DynamicConfigUpdate {
		compaction_admission_percent: Some(Some(80.0)),
		..DynamicConfigUpdate::default()
	};

	let encoded = serde_json::to_string(&update).expect("encodes");
	let decoded: DynamicConfigUpdate = serde_json::from_str(&encoded).expect("decodes");

	// Absent, cleared, and set must survive the wire as three distinct states.
	assert_eq!(decoded, update);
	assert_eq!(decoded.compaction_write_bytes_per_second, None);
}

/// The `runtime` properties are private, so the loaded config comes through deserialization the
/// same way it would from a config file.
fn config_with_max_concurrent_foo() -> Config {
	Config::from_root(
		serde_json::from_value::<Root>(serde_json::json!({
			"auth": { "admin_token": "default" },
			"runtime": { "worker_max_concurrent_workflows": { "foo": 5 } },
		}))
		.expect("valid config"),
	)
}

fn max_concurrent_update(entries: [(&str, Option<usize>); 1]) -> DynamicConfigUpdate {
	DynamicConfigUpdate {
		worker_max_concurrent_workflows: Some(
			entries
				.into_iter()
				.map(|(name, max)| (name.to_string(), max))
				.collect(),
		),
		..DynamicConfigUpdate::default()
	}
}

#[test]
fn max_concurrent_workflows_overrides_one_name_at_a_time() {
	let config = config_with_max_concurrent_foo();

	config
		.apply_dynamic(&max_concurrent_update([("bar", Some(7))]))
		.expect("valid update");

	let dynamic = config.dynamic();
	let max_concurrent = dynamic.runtime.worker_max_concurrent_workflows();

	// The name that was not in the update keeps its loaded value, and the built in defaults for
	// names nobody configured still apply.
	assert_eq!(max_concurrent.get("foo"), Some(&5));
	assert_eq!(max_concurrent.get("bar"), Some(&7));
	assert_eq!(max_concurrent.get("depot_db_manager3"), Some(&100));
}

#[test]
fn clearing_one_max_concurrent_workflow_reverts_only_that_name() {
	let config = config_with_max_concurrent_foo();

	config
		.apply_dynamic(&max_concurrent_update([("foo", Some(50))]))
		.expect("valid update");
	config
		.apply_dynamic(&max_concurrent_update([("bar", Some(7))]))
		.expect("valid update");
	config
		.apply_dynamic(&max_concurrent_update([("foo", None)]))
		.expect("valid update");

	let dynamic = config.dynamic();
	let max_concurrent = dynamic.runtime.worker_max_concurrent_workflows();

	assert_eq!(max_concurrent.get("foo"), Some(&5));
	assert_eq!(max_concurrent.get("bar"), Some(&7));

	// Clearing a name that was never in the config file removes it entirely.
	config
		.apply_dynamic(&max_concurrent_update([("bar", None)]))
		.expect("valid update");
	assert_eq!(
		config
			.dynamic()
			.runtime
			.worker_max_concurrent_workflows()
			.get("bar"),
		None
	);
}

/// The `runtime` properties are private, so the loaded config comes through deserialization the
/// same way it would from a config file.
fn config_with_max_wake_keys_foo() -> Config {
	Config::from_root(
		serde_json::from_value::<Root>(serde_json::json!({
			"auth": { "admin_token": "default" },
			"runtime": { "worker_max_wake_keys_per_workflow_name_per_pull": { "foo": 200 } },
		}))
		.expect("valid config"),
	)
}

fn max_wake_keys_update(entries: [(&str, Option<usize>); 1]) -> DynamicConfigUpdate {
	DynamicConfigUpdate {
		worker_max_wake_keys_per_workflow_name_per_pull: Some(
			entries
				.into_iter()
				.map(|(name, max)| (name.to_string(), max))
				.collect(),
		),
		..DynamicConfigUpdate::default()
	}
}

#[test]
fn max_wake_keys_overrides_one_name_at_a_time() {
	let config = config_with_max_wake_keys_foo();

	config
		.apply_dynamic(&max_wake_keys_update([("bar", Some(7))]))
		.expect("valid update");

	let dynamic = config.dynamic();
	let runtime = &dynamic.runtime;

	// The name that was not in the update keeps its loaded value, and a name nobody configured
	// falls back to the built in default.
	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("foo"),
		200
	);
	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("bar"),
		7
	);
	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("baz"),
		20_000
	);

	// The compaction workflows carry a lower built in default.
	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("depot_db_hot_compactor3"),
		5_000
	);
}

#[test]
fn max_wake_keys_default_key_applies_to_unconfigured_names() {
	let config = config_with_max_wake_keys_foo();

	config
		.apply_dynamic(&max_wake_keys_update([("default", Some(500))]))
		.expect("valid update");

	let dynamic = config.dynamic();
	let runtime = &dynamic.runtime;

	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("foo"),
		200
	);
	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("baz"),
		500
	);
}

#[test]
fn clearing_one_max_wake_keys_name_reverts_only_that_name() {
	let config = config_with_max_wake_keys_foo();

	config
		.apply_dynamic(&max_wake_keys_update([("foo", Some(50))]))
		.expect("valid update");
	config
		.apply_dynamic(&max_wake_keys_update([("bar", Some(7))]))
		.expect("valid update");
	config
		.apply_dynamic(&max_wake_keys_update([("foo", None)]))
		.expect("valid update");

	let dynamic = config.dynamic();
	let runtime = &dynamic.runtime;

	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("foo"),
		200
	);
	assert_eq!(
		runtime.worker_max_wake_keys_per_workflow_name_per_pull("bar"),
		7
	);

	// Clearing a name that was never in the config file removes it entirely.
	config
		.apply_dynamic(&max_wake_keys_update([("bar", None)]))
		.expect("valid update");
	assert_eq!(
		config
			.dynamic()
			.runtime
			.worker_max_wake_keys_per_workflow_name_per_pull("bar"),
		20_000
	);
}

#[test]
fn max_wake_keys_rejects_zero_without_publishing() {
	let config = config_with_max_wake_keys_foo();
	let before = config.dynamic();

	config
		.apply_dynamic(&max_wake_keys_update([("foo", Some(0))]))
		.expect_err("zero must be rejected");
	assert!(std::sync::Arc::ptr_eq(&before, &config.dynamic()));
}
