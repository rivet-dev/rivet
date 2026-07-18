use super::*;

mod moved_tests {
	use std::sync::{Arc, Mutex};
	use std::time::Duration;

	use tokio::sync::mpsc;

	use rivetkit_actor_persist::{
		generated::{v1 as persist_v1, v4 as persist_v4},
		versioned as persist_versioned,
	};
	use vbare::OwnedVersionedData;

	use tokio::sync::Semaphore;

	use crate::actor::config::ActorConfig;
	use crate::actor::connection::{ConnHandle, HibernatableConnectionMetadata};
	use crate::actor::context::tests::{
		TestSqliteWriteGate, new_with_kv, new_with_kv_and_write_gate,
	};
	use crate::actor::internal_storage;
	use crate::actor::keys::{LAST_PUSHED_ALARM_KEY, PERSIST_DATA_KEY};
	use crate::actor::messages::StateDelta;
	use crate::actor::task::LifecycleEvent;
	use crate::kv::tests::new_in_memory;
	use crate::{ActorContext, RequestSaveOpts};

	use super::{
		PersistedActor, PersistedScheduleEvent, decode_last_pushed_alarm, decode_persisted_actor,
		encode_last_pushed_alarm, encode_persisted_actor, throttled_save_delay,
	};

	const PERSISTED_ACTOR_HEX: &str =
		"04000103010203010304050601076576656e742d312a000000000000000470696e6701020708";

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
				timestamp: 42,
				action: "ping".into(),
				args: Some(vec![7, 8]),
			}],
		};

		let encoded = encode_persisted_actor(&actor).expect("persisted actor should encode");
		assert_eq!(hex(&encoded), PERSISTED_ACTOR_HEX);
		let decoded = decode_persisted_actor(&encoded).expect("persisted actor should decode");

		assert_eq!(decoded, actor);
	}

	#[test]
	fn persisted_actor_decodes_old_typescript_v4_optional_schedule_args() {
		let encoded = persist_versioned::Actor::wrap_latest(persist_v4::Actor {
			input: None,
			has_initialized: true,
			state: vec![1],
			scheduled_events: vec![persist_v4::ScheduleEvent {
				event_id: "event-1".to_owned(),
				timestamp: 42,
				action: "ping".to_owned(),
				args: None,
			}],
		})
		.serialize_with_embedded_version(4)
		.expect("old TypeScript actor should encode");

		let decoded = decode_persisted_actor(&encoded).expect("old TypeScript actor should decode");
		assert_eq!(
			decoded,
			PersistedActor {
				input: None,
				has_initialized: true,
				state: vec![1],
				scheduled_events: vec![PersistedScheduleEvent {
					event_id: "event-1".to_owned(),
					timestamp: 42,
					action: "ping".to_owned(),
					args: None,
				}],
			}
		);
	}

	#[test]
	fn scheduled_empty_args_encode_as_typescript_none() {
		let actor = ActorContext::new_for_schedule_tests("actor-empty-schedule-args");
		actor.after(Duration::from_secs(1), "ping", b"");

		let encoded =
			encode_persisted_actor(&actor.persisted_actor()).expect("actor should encode");
		let bare =
			<persist_versioned::Actor as OwnedVersionedData>::deserialize_with_embedded_version(
				&encoded,
			)
			.expect("actor should decode as protocol");

		assert_eq!(bare.scheduled_events[0].args, None);
	}

	#[test]
	fn persisted_actor_decodes_old_typescript_v1_layout() {
		let payload = persist_versioned::Actor::V1(persist_v1::PersistedActor {
			input: Some(vec![1, 2]),
			has_initialized: true,
			state: vec![3, 4],
			connections: Vec::new(),
			scheduled_events: vec![persist_v1::PersistedScheduleEvent {
				event_id: "event-1".to_owned(),
				timestamp: 42,
				kind: persist_v1::PersistedScheduleEventKind::GenericPersistedScheduleEvent(
					persist_v1::GenericPersistedScheduleEvent {
						action: "ping".to_owned(),
						args: Some(vec![5, 6]),
					},
				),
			}],
		})
		.serialize_version(1)
		.expect("old TypeScript v1 actor should encode");
		let mut encoded = 1u16.to_le_bytes().to_vec();
		encoded.extend_from_slice(&payload);

		let decoded =
			decode_persisted_actor(&encoded).expect("old TypeScript v1 actor should decode");
		assert_eq!(
			decoded,
			PersistedActor {
				input: Some(vec![1, 2]),
				has_initialized: true,
				state: vec![3, 4],
				scheduled_events: vec![PersistedScheduleEvent {
					event_id: "event-1".to_owned(),
					timestamp: 42,
					action: "ping".to_owned(),
					args: Some(vec![5, 6]),
				}],
			}
		);
	}

	#[test]
	fn persist_data_key_matches_typescript_layout() {
		assert_eq!(PERSIST_DATA_KEY, &[1]);
	}

	#[test]
	fn last_pushed_alarm_key_matches_actor_kv_layout() {
		assert_eq!(LAST_PUSHED_ALARM_KEY, &[6]);
	}

	#[test]
	fn last_pushed_alarm_round_trips_with_embedded_version() {
		let encoded = encode_last_pushed_alarm(Some(123)).expect("last pushed alarm should encode");
		let decoded = decode_last_pushed_alarm(&encoded).expect("last pushed alarm should decode");
		assert_eq!(decoded, Some(123));

		let encoded_none =
			encode_last_pushed_alarm(None).expect("empty last pushed alarm should encode");
		let decoded_none =
			decode_last_pushed_alarm(&encoded_none).expect("empty last pushed alarm should decode");
		assert_eq!(decoded_none, None);
	}

	#[test]
	fn throttled_save_delay_uses_remaining_interval() {
		let delay = throttled_save_delay(Duration::from_secs(1), Duration::from_millis(250), None);

		assert_eq!(delay, Duration::from_millis(750));
	}

	#[tokio::test]
	async fn request_save_coalesces_and_escalates_to_immediate() {
		let state = ActorContext::new_for_state_tests(new_in_memory(), ActorConfig::default());
		let (events_tx, mut events_rx) = mpsc::unbounded_channel();
		state.configure_lifecycle_events(Some(events_tx));

		state.request_save(RequestSaveOpts::default());
		state.request_save(RequestSaveOpts::default());
		state.request_save(RequestSaveOpts {
			immediate: true,
			max_wait_ms: None,
		});
		state.request_save(RequestSaveOpts {
			immediate: true,
			max_wait_ms: None,
		});

		assert_eq!(
			events_rx.try_recv().expect("first save event should exist"),
			LifecycleEvent::SaveRequested { immediate: false }
		);
		assert_eq!(
			events_rx
				.try_recv()
				.expect("immediate save event should exist"),
			LifecycleEvent::SaveRequested { immediate: true }
		);
		assert!(
			events_rx.try_recv().is_err(),
			"save requests should coalesce"
		);
		assert!(state.save_requested());
		assert!(state.save_requested_immediate());
	}

	#[tokio::test]
	async fn request_save_max_wait_uses_requested_deadline() {
		let state = ActorContext::new_for_state_tests(
			new_in_memory(),
			ActorConfig {
				state_save_interval: Duration::from_secs(5),
				..ActorConfig::default()
			},
		);
		let (events_tx, mut events_rx) = mpsc::unbounded_channel();
		state.configure_lifecycle_events(Some(events_tx));

		let now = std::time::Instant::now();
		state.request_save(RequestSaveOpts {
			immediate: false,
			max_wait_ms: Some(25),
		});

		assert_eq!(
			events_rx
				.try_recv()
				.expect("save-within event should exist"),
			LifecycleEvent::SaveRequested { immediate: false }
		);
		assert!(
			state.compute_save_deadline(false) <= now + Duration::from_millis(50),
			"save-within should bypass the normal throttle window"
		);
	}

	#[tokio::test]
	async fn request_save_hooks_observe_all_requests() {
		let state = ActorContext::new_for_state_tests(new_in_memory(), ActorConfig::default());
		let observed = Arc::new(Mutex::new(Vec::new()));
		state.on_request_save(Box::new({
			let observed = observed.clone();
			move |opts| {
				observed
					.lock()
					.expect("request-save hook log lock poisoned")
					.push(opts);
			}
		}));

		state.request_save(RequestSaveOpts::default());
		state.request_save(RequestSaveOpts {
			immediate: true,
			max_wait_ms: None,
		});
		state.request_save(RequestSaveOpts {
			immediate: false,
			max_wait_ms: Some(10),
		});

		assert_eq!(
			observed
				.lock()
				.expect("request-save hook log lock poisoned")
				.as_slice(),
			[
				RequestSaveOpts::default(),
				RequestSaveOpts {
					immediate: true,
					max_wait_ms: None
				},
				RequestSaveOpts {
					immediate: false,
					max_wait_ms: Some(10)
				},
			]
		);
	}

	#[tokio::test]
	async fn apply_state_deltas_writes_actor_and_connection_state() {
		let kv = new_in_memory();
		let ctx = new_with_kv("actor-1", "state-deltas", Vec::new(), "local", kv.clone());
		let conn = ConnHandle::new("conn-1", Vec::new(), vec![1, 1, 1], true);
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
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

		let persisted = internal_storage::load_actor_snapshot(ctx.sql())
			.await
			.expect("actor state should load")
			.expect("actor state should be persisted")
			.actor;
		assert_eq!(persisted.state, vec![1, 2, 3]);

		let persisted = internal_storage::load_connections(ctx.sql())
			.await
			.expect("connection hibernation should load")
			.into_iter()
			.find(|persisted| persisted.id == conn.id())
			.expect("connection hibernation should be persisted");
		assert_eq!(persisted.state, vec![9, 8, 7]);

		ctx.save_state(vec![StateDelta::ConnHibernationRemoved(conn.id().into())])
			.await
			.expect("hibernation delete should succeed");
		assert!(
			internal_storage::load_connections(ctx.sql())
				.await
				.expect("deleted hibernation should load")
				.into_iter()
				.all(|persisted| persisted.id != conn.id())
		);
	}

	#[tokio::test]
	async fn save_state_applies_actor_upsert_and_hibernation_delete_in_one_batch() {
		let kv = new_in_memory();
		let ctx = new_with_kv(
			"actor-batch",
			"state-batch",
			Vec::new(),
			"local",
			kv.clone(),
		);

		let removed_conn = ConnHandle::new("conn-removed", Vec::new(), vec![4, 4, 4], true);
		removed_conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"req1",
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
			gateway_id: *b"gate",
			request_id: *b"req2",
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

		let persisted = internal_storage::load_actor_snapshot(ctx.sql())
			.await
			.expect("actor state should load")
			.expect("actor state should be persisted")
			.actor;
		assert_eq!(persisted.state, vec![7, 8, 9]);

		let connections = internal_storage::load_connections(ctx.sql())
			.await
			.expect("connection hibernation should load");
		let added = connections
			.iter()
			.find(|persisted| persisted.id == added_conn.id())
			.expect("added hibernation should exist");
		assert_eq!(added.state, vec![1, 2, 3]);

		assert!(
			connections
				.iter()
				.all(|persisted| persisted.id != removed_conn.id())
		);
	}

	// Covers the pending-write waiter contract: `wait_for_pending_state_writes`
	// must observe a save whose sqlite write is still in flight and only return
	// once that write completes. The remote sqlite executor gate stalls the
	// first save's connection insert, so the in-flight window is event-ordered
	// rather than timing-dependent.
	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
	async fn concurrent_save_state_calls_overlap_during_sqlite_write() {
		let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
		let release = Arc::new(Semaphore::new(0));
		let ctx = Arc::new(new_with_kv_and_write_gate(
			"actor-overlap",
			"state-overlap",
			Vec::new(),
			"local",
			new_in_memory(),
			TestSqliteWriteGate {
				sql_prefix: "INSERT OR IGNORE INTO _rivet_conns",
				entered_tx,
				release: release.clone(),
			},
		));

		let conn_1 = ConnHandle::new("conn-overlap-1", Vec::new(), vec![1], true);
		conn_1.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"rq01",
			server_message_index: 1,
			client_message_index: 1,
			request_path: "/ws".to_owned(),
			request_headers: Default::default(),
		}));
		ctx.add_conn(conn_1.clone());

		let conn_2 = ConnHandle::new("conn-overlap-2", Vec::new(), vec![2], true);
		conn_2.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: *b"gate",
			request_id: *b"rq02",
			server_message_index: 1,
			client_message_index: 1,
			request_path: "/ws".to_owned(),
			request_headers: Default::default(),
		}));
		ctx.add_conn(conn_2.clone());

		let first_save = tokio::spawn({
			let ctx = Arc::clone(&ctx);
			let conn = conn_1.id().to_owned();
			async move {
				ctx.save_state(vec![StateDelta::ConnHibernation {
					conn,
					bytes: vec![10],
				}])
				.await
				.expect("first save should succeed");
			}
		});

		entered_rx
			.recv()
			.await
			.expect("first save should reach the stalled sqlite write");

		let second_save = tokio::spawn({
			let ctx = Arc::clone(&ctx);
			let conn = conn_2.id().to_owned();
			async move {
				ctx.save_state(vec![StateDelta::ConnHibernation {
					conn,
					bytes: vec![20],
				}])
				.await
				.expect("second save should succeed");
			}
		});

		let mut wait_task = tokio::spawn({
			let ctx = Arc::clone(&ctx);
			async move {
				ctx.wait_for_pending_state_writes().await;
			}
		});
		// The first save is provably mid-write here (its insert is stalled at
		// the executor gate), so a waiter that returns within this bound did
		// not observe the in-flight write.
		assert!(
			tokio::time::timeout(Duration::from_millis(50), &mut wait_task)
				.await
				.is_err(),
			"pending-write waiters must observe the stalled in-flight write",
		);

		// One permit per gated connection insert.
		release.add_permits(2);

		first_save.await.expect("first save task should not panic");
		second_save
			.await
			.expect("second save task should not panic");
		wait_task
			.await
			.expect("pending write waiter should not panic");
		entered_rx
			.recv()
			.await
			.expect("second save should also pass the sqlite write gate");

		let connections = internal_storage::load_connections(ctx.sql())
			.await
			.expect("connection states should load");
		let conn_1_persisted = connections
			.iter()
			.find(|persisted| persisted.id == conn_1.id())
			.expect("first conn state should be persisted");
		assert_eq!(conn_1_persisted.state, vec![10]);
		let conn_2_persisted = connections
			.iter()
			.find(|persisted| persisted.id == conn_2.id())
			.expect("second conn state should be persisted");
		assert_eq!(conn_2_persisted.state, vec![20]);
	}

	#[tokio::test]
	async fn save_state_resets_pending_request_flags() {
		let ctx = new_with_kv(
			"actor-1",
			"save-state-flags",
			Vec::new(),
			"local",
			new_in_memory(),
		);
		let (events_tx, _events_rx) = mpsc::unbounded_channel();
		ctx.configure_lifecycle_events(Some(events_tx));

		ctx.request_save(RequestSaveOpts {
			immediate: true,
			max_wait_ms: None,
		});
		assert!(ctx.save_requested());
		assert!(ctx.save_requested_immediate());

		ctx.save_state(vec![StateDelta::ActorState(vec![4, 5, 6])])
			.await
			.expect("bypass save should succeed");

		assert!(!ctx.save_requested());
		assert!(!ctx.save_requested_immediate());
	}

	#[tokio::test]
	async fn flush_on_shutdown_tracks_immediate_persist_until_teardown() {
		let kv = new_in_memory();
		let state = new_with_kv("state-test", "state-test", Vec::new(), "local", kv.clone());

		state.set_initial_state(vec![7, 8, 9]);
		state.flush_on_shutdown();

		assert!(state.tracked_persist_pending());

		state.wait_for_pending_writes().await;
		assert!(!state.tracked_persist_pending());

		let persisted = internal_storage::load_actor_snapshot(state.sql())
			.await
			.expect("actor state should load")
			.expect("actor state should be persisted")
			.actor;
		assert_eq!(persisted.state, vec![7, 8, 9]);
	}
}
