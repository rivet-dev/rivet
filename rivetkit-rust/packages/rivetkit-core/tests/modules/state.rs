use super::*;

mod moved_tests {
	use std::time::Duration;

	use super::{
		PersistedActor, PersistedScheduleEvent, decode_persisted_actor, encode_persisted_actor,
		throttled_save_delay,
	};

	const PERSISTED_ACTOR_HEX: &str =
		"04000103010203010304050601076576656e742d312a000000000000000470696e67020708";

	fn hex(bytes: &[u8]) -> String {
		bytes.iter().map(|byte| format!("{byte:02x}")).collect()
	}

	#[test]
	fn persisted_actor_round_trips_with_embedded_version() {
		let actor = PersistedActor {
			input: Some(vec![1, 2, 3]),
			has_initialized: true,
			state: vec![4, 5, 6],
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "event-1".into(),
				timestamp_ms: 42,
				action: "ping".into(),
				args: vec![7, 8],
			}],
		};

		let encoded = encode_persisted_actor(&actor).expect("persisted actor should encode");
		assert_eq!(hex(&encoded), PERSISTED_ACTOR_HEX);
		let decoded = decode_persisted_actor(&encoded).expect("persisted actor should decode");

		assert_eq!(decoded, actor);
	}

	#[test]
	fn persist_data_key_matches_typescript_layout() {
		assert_eq!(super::PERSIST_DATA_KEY, &[1]);
	}

	#[test]
	fn throttled_save_delay_uses_remaining_interval() {
		let delay = throttled_save_delay(Duration::from_secs(1), Duration::from_millis(250), None);

		assert_eq!(delay, Duration::from_millis(750));
	}

	#[test]
	fn throttled_save_delay_respects_max_wait() {
		let delay = throttled_save_delay(
			Duration::from_secs(1),
			Duration::from_millis(250),
			Some(Duration::from_millis(100)),
		);

		assert_eq!(delay, Duration::from_millis(100));
	}
}
