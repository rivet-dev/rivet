use super::*;

pub(crate) fn new_with_kv(
	actor_id: impl Into<String>,
	name: impl Into<String>,
	key: ActorKey,
	region: impl Into<String>,
	kv: crate::kv::Kv,
) -> ActorContext {
	ActorContext::build(
		actor_id.into(),
		name.into(),
		key,
		region.into(),
		ActorConfig::default(),
		kv,
		SqliteDb::default(),
	)
}

mod moved_tests {
	use std::collections::{BTreeSet, HashMap, HashSet};
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::sync::{Arc, Mutex};
	use std::time::{Duration, SystemTime, UNIX_EPOCH};

	use anyhow::anyhow;
	use rivet_envoy_client::config::{
		BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse,
		WebSocketHandler, WebSocketSender,
	};
	use rivet_envoy_client::context::{SharedActorEntry, SharedContext, WsTxMessage};
	use rivet_envoy_client::handle::EnvoyHandle;
	use rivet_envoy_client::protocol;
	use rivet_envoy_client::tunnel::HibernatingWebSocketMetadata;
	use tokio::sync::mpsc;
	use tokio::time::{Instant, sleep};

	use super::ActorContext;
	use crate::actor::callbacks::ActorEvent;
	use crate::actor::connection::ConnHandle;
	use crate::actor::state::{PersistedActor, PersistedScheduleEvent};
	use crate::types::ListOpts;

	fn now_timestamp_ms() -> i64 {
		let duration = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_default();
		i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
	}

	struct IdleEnvoyCallbacks;

	impl EnvoyCallbacks for IdleEnvoyCallbacks {
		fn on_actor_start(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_config: protocol::ActorConfig,
			_preloaded_kv: Option<protocol::PreloadedKv>,
			_sqlite_schema_version: u32,
			_sqlite_startup_data: Option<protocol::SqliteStartupData>,
		) -> BoxFuture<anyhow::Result<()>> {
			Box::pin(async { Ok(()) })
		}

		fn on_shutdown(&self) {}

		fn fetch(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
		) -> BoxFuture<anyhow::Result<HttpResponse>> {
			Box::pin(async { anyhow::bail!("fetch should not be called in context tests") })
		}

		fn websocket(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
			_path: String,
			_headers: HashMap<String, String>,
			_is_hibernatable: bool,
			_is_restoring_hibernatable: bool,
			_sender: WebSocketSender,
		) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
			Box::pin(async {
				anyhow::bail!("websocket should not be called in context tests")
			})
		}

		fn can_hibernate(
			&self,
			_actor_id: &str,
			_gateway_id: &protocol::GatewayId,
			_request_id: &protocol::RequestId,
			_request: &HttpRequest,
		) -> bool {
			false
		}
	}

	fn build_envoy_handle_with_live_connections(
		actor_id: &str,
		generation: u32,
		live_connections: HashSet<[u8; 8]>,
		pending_restores: Vec<HibernatingWebSocketMetadata>,
	) -> EnvoyHandle {
		let (envoy_tx, _envoy_rx) = mpsc::unbounded_channel();
		let live_tunnel_requests = Arc::new(std::sync::Mutex::new(HashMap::new()));
		{
			let mut requests = live_tunnel_requests
				.lock()
				.expect("live tunnel request registry poisoned");
			for request_key in live_connections {
				requests.insert(request_key, actor_id.to_owned());
			}
		}
		let shared = Arc::new(SharedContext {
			config: EnvoyConfig {
				version: 1,
				endpoint: "http://127.0.0.1:1".to_string(),
				token: None,
				namespace: "test".to_string(),
				pool_name: "test".to_string(),
				prepopulate_actor_names: HashMap::new(),
				metadata: None,
				not_global: true,
				debug_latency_ms: None,
				callbacks: Arc::new(IdleEnvoyCallbacks),
			},
			envoy_key: "test-envoy".to_string(),
			envoy_tx,
			actors: Arc::new(std::sync::Mutex::new(HashMap::new())),
			live_tunnel_requests,
			pending_hibernation_restores: Arc::new(std::sync::Mutex::new(
				HashMap::from([(actor_id.to_owned(), pending_restores)]),
			)),
			ws_tx: Arc::new(tokio::sync::Mutex::new(None::<mpsc::UnboundedSender<WsTxMessage>>)),
			protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
			shutting_down: std::sync::atomic::AtomicBool::new(false),
		});
		shared
			.actors
			.lock()
			.expect("shared actor registry poisoned")
			.entry(actor_id.to_owned())
			.or_insert_with(HashMap::new)
			.insert(
				generation,
				SharedActorEntry {
					handle: mpsc::unbounded_channel().0,
					active_http_request_count: Arc::new(
						rivet_util::async_counter::AsyncCounter::new(),
					),
				},
			);
		EnvoyHandle::from_shared(shared)
	}

	#[tokio::test]
	async fn kv_helpers_delegate_to_kv_wrapper() {
		let ctx = super::new_with_kv(
			"actor-1",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		ctx.kv_batch_put(&[(b"alpha".as_slice(), b"1".as_slice())])
			.await
			.expect("kv batch put should succeed");

		let values = ctx
			.kv_batch_get(&[b"alpha".as_slice()])
			.await
			.expect("kv batch get should succeed");
		assert_eq!(values, vec![Some(b"1".to_vec())]);

		let listed = ctx
			.kv_list_prefix(b"alp", ListOpts::default())
			.await
			.expect("kv list prefix should succeed");
		assert_eq!(listed, vec![(b"alpha".to_vec(), b"1".to_vec())]);

		ctx.kv_batch_delete(&[b"alpha".as_slice()])
			.await
			.expect("kv batch delete should succeed");
		let values = ctx
			.kv_batch_get(&[b"alpha".as_slice()])
			.await
			.expect("kv batch get after delete should succeed");
		assert_eq!(values, vec![None]);
	}

	#[tokio::test]
	async fn foreign_runtime_only_helpers_fail_explicitly_when_unconfigured() {
		let ctx = ActorContext::default();

		assert!(ctx.db_exec("select 1").await.is_err());
		assert!(ctx.db_query("select 1", None).await.is_err());
		assert!(ctx.db_run("select 1", None).await.is_err());
		assert!(ctx.client_call(b"call").await.is_err());
		assert!(ctx.set_alarm(Some(1)).is_err());
		assert!(ctx
			.ack_hibernatable_websocket_message(b"gateway", b"request", 1)
			.is_err());
	}

	#[tokio::test]
	async fn connection_helpers_iterate_and_disconnect_without_managed_callback() {
		let ctx = super::new_with_kv(
			"actor-conns",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		let managed_disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		let transport_disconnects = Arc::new(Mutex::new(Vec::<String>::new()));

		let conn_a = ConnHandle::new("conn-a", vec![1], vec![2], false);
		conn_a.configure_disconnect_handler(Some(Arc::new({
			let managed_disconnects = managed_disconnects.clone();
			move |_reason| {
				let managed_disconnects = managed_disconnects.clone();
				Box::pin(async move {
					managed_disconnects
						.lock()
						.expect("managed disconnect log lock poisoned")
						.push("conn-a".to_owned());
					Ok(())
				})
			}
		})));
		conn_a.configure_transport_disconnect_handler(Some(Arc::new({
			let transport_disconnects = transport_disconnects.clone();
			move |_reason| {
				let transport_disconnects = transport_disconnects.clone();
				Box::pin(async move {
					transport_disconnects
						.lock()
						.expect("transport disconnect log lock poisoned")
						.push("conn-a".to_owned());
					Ok(())
				})
			}
		})));

		let conn_b = ConnHandle::new("conn-b", vec![3], vec![4], false);
		ctx.add_conn(conn_a);
		ctx.add_conn(conn_b);

		assert_eq!(
			ctx.conns()
				.map(|conn| conn.id().to_owned())
				.collect::<Vec<_>>(),
			vec!["conn-a".to_owned(), "conn-b".to_owned()]
		);
		assert_eq!(ctx.conns().len(), 2);

		ctx.disconnect_conn("conn-a".into())
			.await
			.expect("targeted disconnect should succeed");

		assert_eq!(
			transport_disconnects
				.lock()
				.expect("transport disconnect log lock poisoned")
				.as_slice(),
			["conn-a"]
		);
		assert!(
			managed_disconnects
				.lock()
				.expect("managed disconnect log lock poisoned")
				.is_empty()
		);
		assert_eq!(
			ctx.conns()
				.map(|conn| conn.id().to_owned())
				.collect::<Vec<_>>(),
			vec!["conn-b".to_owned()]
		);
	}

	#[tokio::test]
	async fn take_pending_hibernation_changes_snapshots_removals_without_draining_core_state() {
		let ctx = super::new_with_kv(
			"actor-hibernation-pending",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		ctx.request_hibernation_transport_save("conn-updated");
		ctx.request_hibernation_transport_removal("conn-removed");

		assert_eq!(
			ctx.take_pending_hibernation_changes(),
			vec!["conn-removed".to_owned()]
		);

		let pending = ctx.0.connections.take_pending_hibernation_changes();
		assert_eq!(pending.updated, BTreeSet::from(["conn-updated".to_owned()]));
		assert_eq!(pending.removed, BTreeSet::from(["conn-removed".to_owned()]));
	}

	#[tokio::test]
	async fn hibernated_connection_is_live_checks_specific_live_registry_entry() {
		let ctx = super::new_with_kv(
			"actor-live-conn",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		ctx.configure_envoy(
			build_envoy_handle_with_live_connections(
				"actor-live-conn",
				7,
				HashSet::from([[
					1, 2, 3, 4, 5, 6, 7, 8,
				]]),
				Vec::new(),
			),
			Some(7),
		);

		assert!(
			ctx
				.hibernated_connection_is_live(&[1, 2, 3, 4], &[5, 6, 7, 8])
				.expect("matching live connection should be found")
		);
		assert!(
			!ctx
				.hibernated_connection_is_live(&[1, 2, 3, 4], &[9, 9, 9, 9])
				.expect("missing live connection should return false")
		);
	}

	#[tokio::test]
	async fn hibernated_connection_is_live_checks_pending_restore_registry_entry() {
		let ctx = super::new_with_kv(
			"actor-pending-restore-conn",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);
		ctx.configure_envoy(
			build_envoy_handle_with_live_connections(
				"actor-pending-restore-conn",
				3,
				HashSet::new(),
				vec![HibernatingWebSocketMetadata {
					gateway_id: [1, 2, 3, 4],
					request_id: [5, 6, 7, 8],
					envoy_message_index: 0,
					rivet_message_index: 0,
					path: "/ws".to_owned(),
					headers: HashMap::new(),
				}],
			),
			Some(3),
		);

		assert!(
			ctx
				.hibernated_connection_is_live(&[1, 2, 3, 4], &[5, 6, 7, 8])
				.expect("pending restore should count as a live hibernated connection")
		);
		assert!(
			!ctx
				.hibernated_connection_is_live(&[9, 9, 9, 9], &[5, 6, 7, 8])
				.expect("non-matching pending restore should return false")
		);
	}

	#[tokio::test]
	async fn disconnect_conns_continues_past_per_conn_errors() {
		let ctx = super::new_with_kv(
			"actor-disconnect-conns",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let conn_a = ConnHandle::new("conn-a", vec![1], vec![2], false);
		conn_a.configure_transport_disconnect_handler(Some(Arc::new(move |_reason| {
			Box::pin(async move { Err(anyhow!("boom-a")) })
		})));

		let conn_b = ConnHandle::new("conn-b", vec![3], vec![4], false);
		let transport_disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		conn_b.configure_transport_disconnect_handler(Some(Arc::new({
			let transport_disconnects = transport_disconnects.clone();
			move |_reason| {
				let transport_disconnects = transport_disconnects.clone();
				Box::pin(async move {
					transport_disconnects
						.lock()
						.expect("transport disconnect log lock poisoned")
						.push("conn-b".to_owned());
					Ok(())
				})
			}
		})));

		ctx.add_conn(conn_a.clone());
		ctx.add_conn(conn_b);

		let error = ctx
			.disconnect_conns(|conn| conn.id().starts_with("conn-"))
			.await
			.expect_err("bulk disconnect should surface transport failures");
		let error_text = format!("{error:#}");
		assert!(error_text.contains("conn-a"));
		assert!(
			transport_disconnects
				.lock()
				.expect("transport disconnect log lock poisoned")
				.iter()
				.any(|conn_id| conn_id == "conn-b")
		);
		assert!(ctx.conns().any(|conn| conn.id() == "conn-a"));
		assert!(!ctx.conns().any(|conn| conn.id() == "conn-b"));

		let conn_c = ConnHandle::new("conn-c", vec![5], vec![6], false);
		conn_c.configure_transport_disconnect_handler(Some(Arc::new(move |_reason| {
			Box::pin(async move { Err(anyhow!("boom-c")) })
		})));
		ctx.add_conn(conn_c);

		let error = ctx
			.disconnect_conns(|conn| conn.id() == "conn-a" || conn.id() == "conn-c")
			.await
			.expect_err("bulk disconnect should aggregate multiple failures");
		let error_text = format!("{error:#}");
		assert!(error_text.contains("conn-a"));
		assert!(error_text.contains("conn-c"));
		assert!(ctx.conns().any(|conn| conn.id() == "conn-a"));
		assert!(ctx.conns().any(|conn| conn.id() == "conn-c"));
	}

	#[tokio::test]
	async fn init_alarms_arms_local_alarm_for_persisted_schedule_state() {
		let ctx = super::new_with_kv(
			"actor-init-alarms",
			"actor",
			Vec::new(),
			"local",
			crate::kv::Kv::new_in_memory(),
		);
		let fired = Arc::new(AtomicUsize::new(0));
		ctx.schedule().set_local_alarm_callback(Some(Arc::new({
			let fired = fired.clone();
			move || {
				let fired = fired.clone();
				Box::pin(async move {
					fired.fetch_add(1, Ordering::SeqCst);
				})
			}
		})));
		ctx.load_persisted_actor(PersistedActor {
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "evt-future".to_owned(),
				timestamp_ms: now_timestamp_ms() + 20,
				action: "tick".to_owned(),
				args: vec![1],
			}],
			..PersistedActor::default()
		});

		ctx.init_alarms();

		for _ in 0..50 {
			if fired.load(Ordering::SeqCst) > 0 {
				break;
			}
			sleep(Duration::from_millis(10)).await;
		}

		assert_eq!(fired.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn drain_overdue_scheduled_events_dispatches_actions_via_actor_inbox() {
		let ctx = super::new_with_kv(
			"actor-overdue-events",
			"actor",
			Vec::new(),
			"local",
			crate::kv::Kv::new_in_memory(),
		);
		let (events_tx, mut events_rx) = mpsc::channel(4);
		ctx.configure_actor_events(Some(events_tx));
		ctx.load_persisted_actor(PersistedActor {
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "evt-overdue".to_owned(),
				timestamp_ms: now_timestamp_ms() - 1_000,
				action: "tick".to_owned(),
				args: vec![1, 2, 3],
			}],
			..PersistedActor::default()
		});

		let recv = tokio::spawn(async move {
			match events_rx.recv().await.expect("scheduled action event should arrive") {
				ActorEvent::Action {
					name,
					args,
					conn,
					reply,
				} => {
					assert_eq!(name, "tick");
					assert_eq!(args, vec![1, 2, 3]);
					assert!(conn.is_none());
					reply.send(Ok(Vec::new()));
				}
				event => panic!("unexpected event: {event:?}"),
			}
		});

		ctx
			.drain_overdue_scheduled_events()
			.await
			.expect("draining overdue scheduled events should succeed");
		recv.await.expect("scheduled action receiver should join");

		assert!(ctx.schedule().next_event().is_none());
	}

	#[tokio::test]
	async fn keep_awake_region_blocks_sleep_idle_until_guard_drops() {
		let ctx = super::new_with_kv(
			"actor-keep-awake",
			"actor",
			Vec::new(),
			"local",
			crate::kv::Kv::new_in_memory(),
		);

		let keep_awake = tokio::spawn({
			let ctx = ctx.clone();
			async move {
				ctx.keep_awake(async {
					sleep(Duration::from_millis(30)).await;
				})
				.await;
			}
		});

		for _ in 0..20 {
			if ctx.keep_awake_count() > 0 {
				break;
			}
			sleep(Duration::from_millis(1)).await;
		}

		assert_eq!(ctx.keep_awake_count(), 1);
		assert!(
			!ctx
				.wait_for_sleep_idle_window(Instant::now() + Duration::from_millis(5))
				.await
		);
		assert!(
			ctx.wait_for_sleep_idle_window(Instant::now() + Duration::from_millis(100))
				.await
		);

		keep_awake
			.await
			.expect("keep_awake task should complete");
		assert_eq!(ctx.keep_awake_count(), 0);
	}

	#[tokio::test(start_paused = true)]
	async fn sleep_requests_envoy_on_next_scheduler_tick_without_wall_clock_delay() {
		let ctx = super::new_with_kv(
			"actor-sleep-request",
			"actor",
			Vec::new(),
			"local",
			crate::kv::Kv::new_in_memory(),
		);

		assert_eq!(ctx.0.sleep.sleep_request_count(), 0);

		ctx.sleep();
		tokio::task::yield_now().await;

		assert_eq!(ctx.0.sleep.sleep_request_count(), 1);
	}
}
