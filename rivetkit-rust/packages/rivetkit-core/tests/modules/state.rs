use super::*;

mod moved_tests {
	use std::sync::{Arc, Mutex};
	use std::time::Duration;

	use tokio::sync::mpsc;

	use crate::actor::callbacks::StateDelta;
	use crate::actor::connection::{
		ConnHandle, HibernatableConnectionMetadata, decode_persisted_connection,
		make_connection_key,
	};
	use crate::actor::config::ActorConfig;
	use crate::actor::context::tests::new_with_kv;
	use crate::actor::task::LifecycleEvent;
	use crate::kv::tests::new_in_memory;
	use crate::ActorContext;

	use super::{
		PERSIST_DATA_KEY, PersistedActor, PersistedScheduleEvent, ActorState,
		decode_persisted_actor, encode_persisted_actor, throttled_save_delay,
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
		let decoded =
			decode_persisted_actor(&encoded).expect("persisted actor should decode");

		assert_eq!(decoded, actor);
	}

	#[test]
	fn persist_data_key_matches_typescript_layout() {
		assert_eq!(super::PERSIST_DATA_KEY, &[1]);
	}

	#[test]
	fn throttled_save_delay_uses_remaining_interval() {
		let delay = throttled_save_delay(
			Duration::from_secs(1),
			Duration::from_millis(250),
			None,
		);

		assert_eq!(delay, Duration::from_millis(750));
	}

	#[tokio::test]
	async fn request_save_coalesces_and_escalates_to_immediate() {
		let state = ActorState::new(
			new_in_memory(),
			ActorConfig {
				lifecycle_event_inbox_capacity: 4,
				..ActorConfig::default()
			},
		);
		let (events_tx, mut events_rx) = mpsc::channel(4);
		state.configure_lifecycle_events(Some(events_tx));

		state.request_save(false);
		state.request_save(false);
		state.request_save(true);
		state.request_save(true);

		assert_eq!(
			events_rx.try_recv().expect("first save event should exist"),
			LifecycleEvent::SaveRequested { immediate: false }
		);
		assert_eq!(
			events_rx.try_recv().expect("immediate save event should exist"),
			LifecycleEvent::SaveRequested { immediate: true }
		);
		assert!(events_rx.try_recv().is_err(), "save requests should coalesce");
		assert!(state.save_requested());
		assert!(state.save_requested_immediate());
	}

	#[tokio::test]
	async fn request_save_within_uses_requested_deadline() {
		let state = ActorState::new(
			new_in_memory(),
			ActorConfig {
				state_save_interval: Duration::from_secs(5),
				lifecycle_event_inbox_capacity: 4,
				..ActorConfig::default()
			},
		);
		let (events_tx, mut events_rx) = mpsc::channel(4);
		state.configure_lifecycle_events(Some(events_tx));

		let now = std::time::Instant::now();
		state.request_save_within(25);

		assert_eq!(
			events_rx.try_recv().expect("save-within event should exist"),
			LifecycleEvent::SaveRequested { immediate: false }
		);
		assert!(
			state.compute_save_deadline(false) <= now + Duration::from_millis(50),
			"save-within should bypass the normal throttle window"
		);
	}

	#[tokio::test]
	async fn request_save_hooks_observe_all_requests() {
		let state = ActorState::new(new_in_memory(), ActorConfig::default());
		let observed = Arc::new(Mutex::new(Vec::new()));
		state.on_request_save(Box::new({
			let observed = observed.clone();
			move |immediate| {
				observed
					.lock()
					.expect("request-save hook log lock poisoned")
					.push(immediate);
			}
		}));

		state.request_save(false);
		state.request_save(true);
		state.request_save_within(10);

		assert_eq!(
			observed
				.lock()
				.expect("request-save hook log lock poisoned")
				.as_slice(),
			[false, true, false]
		);
	}

	#[tokio::test]
	async fn apply_state_deltas_writes_actor_and_connection_state() {
		let kv = new_in_memory();
		let ctx = new_with_kv("actor-1", "state-deltas", Vec::new(), "local", kv.clone());
		let conn = ConnHandle::new("conn-1", Vec::new(), vec![1, 1, 1], true);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: b"gateway".to_vec(),
			request_id: b"request".to_vec(),
			server_message_index: 3,
			client_message_index: 7,
			request_path: "/ws".to_owned(),
			request_headers: Default::default(),
		}));
		ctx.add_conn(conn.clone());

		ctx.save_state(vec![
			StateDelta::ActorState(vec![1, 2, 3]),
			StateDelta::ConnHibernation {
				conn: conn.id().into(),
				bytes: vec![9, 8, 7],
			},
		])
		.await
		.expect("delta save should succeed");

		let actor_bytes = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("actor state should load")
			.expect("actor state should be persisted");
		let persisted = decode_persisted_actor(&actor_bytes).expect("actor state should decode");
		assert_eq!(persisted.state, vec![1, 2, 3]);

		let conn_bytes = kv
			.get(&make_connection_key(conn.id()))
			.await
			.expect("connection hibernation should load")
			.expect("connection hibernation should be persisted");
		let persisted =
			decode_persisted_connection(&conn_bytes).expect("connection should decode");
		assert_eq!(persisted.state, vec![9, 8, 7]);

		ctx.save_state(vec![StateDelta::ConnHibernationRemoved(conn.id().into())])
			.await
			.expect("hibernation delete should succeed");
		assert_eq!(
			kv.get(&make_connection_key(conn.id()))
				.await
				.expect("deleted hibernation should load"),
			None
		);
	}

	#[tokio::test]
	async fn save_state_applies_actor_upsert_and_hibernation_delete_in_one_batch() {
		let kv = new_in_memory();
		let ctx = new_with_kv("actor-batch", "state-batch", Vec::new(), "local", kv.clone());

		let removed_conn = ConnHandle::new("conn-removed", Vec::new(), vec![4, 4, 4], true);
		removed_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: b"gate".to_vec(),
			request_id: b"req1".to_vec(),
			server_message_index: 1,
			client_message_index: 1,
			request_path: "/ws".to_owned(),
			request_headers: Default::default(),
		}));
		ctx.add_conn(removed_conn.clone());
		ctx.save_state(vec![StateDelta::ConnHibernation {
			conn: removed_conn.id().into(),
			bytes: vec![5, 5, 5],
		}])
		.await
		.expect("seed delete target should persist");

		let added_conn = ConnHandle::new("conn-added", Vec::new(), vec![6, 6, 6], true);
		added_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: b"gate".to_vec(),
			request_id: b"req2".to_vec(),
			server_message_index: 2,
			client_message_index: 2,
			request_path: "/ws".to_owned(),
			request_headers: Default::default(),
		}));
		ctx.add_conn(added_conn.clone());

		ctx.save_state(vec![
			StateDelta::ActorState(vec![7, 8, 9]),
			StateDelta::ConnHibernation {
				conn: added_conn.id().into(),
				bytes: vec![1, 2, 3],
			},
			StateDelta::ConnHibernationRemoved(removed_conn.id().into()),
		])
		.await
		.expect("combined delta save should succeed");

		let actor_bytes = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("actor state should load")
			.expect("actor state should be persisted");
		let persisted = decode_persisted_actor(&actor_bytes).expect("actor state should decode");
		assert_eq!(persisted.state, vec![7, 8, 9]);

		let added_bytes = kv
			.get(&make_connection_key(added_conn.id()))
			.await
			.expect("added hibernation should load")
			.expect("added hibernation should exist");
		let added =
			decode_persisted_connection(&added_bytes).expect("added hibernation should decode");
		assert_eq!(added.state, vec![1, 2, 3]);

		assert_eq!(
			kv.get(&make_connection_key(removed_conn.id()))
				.await
				.expect("removed hibernation should load"),
			None
		);
	}

	#[tokio::test]
	async fn save_state_resets_pending_request_flags() {
		let ctx = ActorContext::new_with_kv(
			"actor-1",
			"save-state-flags",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (events_tx, _events_rx) = mpsc::channel(4);
		ctx.configure_lifecycle_events(Some(events_tx));

		ctx.request_save(true);
		assert!(ctx.save_requested());
		assert!(ctx.save_requested_immediate());

		ctx.save_state(vec![StateDelta::ActorState(vec![4, 5, 6])])
			.await
			.expect("bypass save should succeed");

		assert!(!ctx.save_requested());
		assert!(!ctx.save_requested_immediate());
	}

	#[tokio::test(start_paused = true)]
	async fn flush_on_shutdown_tracks_immediate_persist_until_teardown() {
		let kv = new_in_memory();
		let state = ActorState::new(kv.clone(), ActorConfig::default());

		state
			.set_state(vec![7, 8, 9])
			.expect("state mutation should succeed");
		state.flush_on_shutdown();

		assert!(state.tracked_persist_pending());

		state.wait_for_pending_writes().await;
		assert!(!state.tracked_persist_pending());

		let actor_bytes = kv
			.get(PERSIST_DATA_KEY)
			.await
			.expect("actor state should load")
			.expect("actor state should be persisted");
		let persisted = decode_persisted_actor(&actor_bytes).expect("actor state should decode");
		assert_eq!(persisted.state, vec![7, 8, 9]);
	}
}
