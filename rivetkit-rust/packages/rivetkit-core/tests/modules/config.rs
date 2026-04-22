use super::*;

mod moved_tests {
	use std::time::Duration;

	use super::{ActorConfig, ActorConfigInput};

	#[test]
	fn actor_config_from_input_applies_overrides() {
		let config = ActorConfig::from_input(ActorConfigInput {
			name: Some("demo".to_owned()),
			on_migrate_timeout_ms: Some(30_000),
			sleep_grace_period_ms: Some(12_000),
			max_queue_size: Some(42),
			preload_max_workflow_bytes: Some(1024.0),
			..ActorConfigInput::default()
		});

		assert_eq!(config.name.as_deref(), Some("demo"));
		assert_eq!(config.on_migrate_timeout, Duration::from_secs(30));
		assert_eq!(config.sleep_grace_period, Duration::from_secs(12));
		assert!(config.sleep_grace_period_overridden);
		assert_eq!(config.max_queue_size, 42);
		assert_eq!(config.preload_max_workflow_bytes, Some(1024));
	}

	#[test]
	fn actor_config_from_input_keeps_defaults_for_missing_fields() {
		let config = ActorConfig::from_input(ActorConfigInput::default());
		let default = ActorConfig::default();

		assert_eq!(config.name, default.name);
		assert_eq!(config.icon, default.icon);
		assert_eq!(config.state_save_interval, default.state_save_interval);
		assert_eq!(config.create_vars_timeout, default.create_vars_timeout);
		assert_eq!(
			config.create_conn_state_timeout,
			default.create_conn_state_timeout,
		);
		assert_eq!(
			config.on_before_connect_timeout,
			default.on_before_connect_timeout,
		);
		assert_eq!(config.on_connect_timeout, default.on_connect_timeout);
		assert_eq!(config.on_migrate_timeout, default.on_migrate_timeout);
		assert_eq!(config.on_destroy_timeout, default.on_destroy_timeout);
		assert_eq!(config.action_timeout, default.action_timeout);
		assert_eq!(config.wait_until_timeout, default.wait_until_timeout);
		assert_eq!(config.sleep_timeout, default.sleep_timeout);
		assert_eq!(config.no_sleep, default.no_sleep);
		assert_eq!(config.sleep_grace_period, default.sleep_grace_period);
		assert_eq!(
			config.sleep_grace_period_overridden,
			default.sleep_grace_period_overridden,
		);
		assert_eq!(
			config.connection_liveness_timeout,
			default.connection_liveness_timeout,
		);
		assert_eq!(
			config.connection_liveness_interval,
			default.connection_liveness_interval,
		);
		assert_eq!(config.max_queue_size, default.max_queue_size);
		assert_eq!(
			config.max_queue_message_size,
			default.max_queue_message_size,
		);
		assert_eq!(
			config.max_incoming_message_size,
			default.max_incoming_message_size,
		);
		assert_eq!(
			config.max_outgoing_message_size,
			default.max_outgoing_message_size,
		);
		assert_eq!(
			config.lifecycle_command_inbox_capacity,
			default.lifecycle_command_inbox_capacity,
		);
		assert_eq!(
			config.dispatch_command_inbox_capacity,
			default.dispatch_command_inbox_capacity,
		);
		assert_eq!(
			config.lifecycle_event_inbox_capacity,
			default.lifecycle_event_inbox_capacity,
		);
		assert_eq!(
			config.preload_max_workflow_bytes,
			default.preload_max_workflow_bytes,
		);
		assert_eq!(
			config.preload_max_connections_bytes,
			default.preload_max_connections_bytes,
		);
		assert!(matches!(
			config.can_hibernate_websocket,
			super::CanHibernateWebSocket::Bool(false),
		));
		assert!(config.overrides.is_none());
	}

	#[test]
	fn actor_config_effective_sleep_grace_period_uses_default() {
		let config = ActorConfig::default();

		assert_eq!(
			config.effective_sleep_grace_period(),
			Duration::from_secs(15),
		);
	}

	#[test]
	fn actor_config_effective_sleep_grace_period_uses_explicit_value() {
		let config = ActorConfig {
			wait_until_timeout: Duration::from_secs(8),
			sleep_grace_period: Duration::from_secs(20),
			sleep_grace_period_overridden: true,
			..ActorConfig::default()
		};

		assert_eq!(
			config.effective_sleep_grace_period(),
			Duration::from_secs(20),
		);
	}
}
