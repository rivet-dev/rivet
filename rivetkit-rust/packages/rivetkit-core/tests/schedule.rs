use super::*;

mod moved_tests {
	use std::collections::HashMap;
	use std::sync::Mutex as EnvoySharedMutex;
	use std::sync::atomic::AtomicBool;

	use rivet_envoy_client::config::{
		BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
		WebSocketSender,
	};
	use rivet_envoy_client::context::{SharedContext, WsTxMessage};
	use rivet_envoy_client::envoy::ToEnvoyMessage;
	use rivet_envoy_client::protocol;
	use tokio::sync::mpsc;

	use super::*;

	struct IdleEnvoyCallbacks;

	impl EnvoyCallbacks for IdleEnvoyCallbacks {
		fn on_actor_start(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_config: protocol::ActorConfig,
			_preloaded_kv: Option<protocol::PreloadedKv>,
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
			Box::pin(async { anyhow::bail!("fetch should not run in schedule tests") })
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
			Box::pin(async { anyhow::bail!("websocket should not run in schedule tests") })
		}

		fn can_hibernate(
			&self,
			_actor_id: &str,
			_gateway_id: &protocol::GatewayId,
			_request_id: &protocol::RequestId,
			_request: &HttpRequest,
		) -> BoxFuture<anyhow::Result<bool>> {
			Box::pin(async { Ok(false) })
		}
	}

	fn test_envoy_handle() -> (EnvoyHandle, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		let (envoy_tx, envoy_rx) = mpsc::unbounded_channel();
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
			actors: Arc::new(EnvoySharedMutex::new(HashMap::new())),
			live_tunnel_requests: Arc::new(EnvoySharedMutex::new(HashMap::new())),
			pending_hibernation_restores: Arc::new(EnvoySharedMutex::new(HashMap::new())),
			ws_tx: Arc::new(tokio::sync::Mutex::new(
				None::<mpsc::UnboundedSender<WsTxMessage>>,
			)),
			protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
			shutting_down: AtomicBool::new(false),
			stopped_tx: tokio::sync::watch::channel(true).0,
		});

		(EnvoyHandle::from_shared(shared), envoy_rx)
	}

	fn recv_alarm_now(
		rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>,
		expected_actor_id: &str,
		expected_generation: Option<u32>,
	) -> Option<i64> {
		match rx.try_recv() {
			Ok(ToEnvoyMessage::SetAlarm {
				actor_id,
				generation,
				alarm_ts,
				ack_tx,
			}) => {
				assert_eq!(actor_id, expected_actor_id);
				assert_eq!(generation, expected_generation);
				if let Some(ack_tx) = ack_tx {
					let _ = ack_tx.send(());
				}
				alarm_ts
			}
			Ok(_) => panic!("expected set_alarm envoy message"),
			Err(error) => panic!("expected set_alarm envoy message, got {error:?}"),
		}
	}

	fn assert_no_alarm(rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		assert!(matches!(
			rx.try_recv(),
			Err(mpsc::error::TryRecvError::Empty)
		));
	}

	#[test]
	fn sync_alarm_skips_driver_push_until_schedule_changes() {
		let schedule = ActorContext::new_for_schedule_tests("actor-schedule-dirty");
		let (handle, mut rx) = test_envoy_handle();
		schedule.configure_schedule_envoy(handle, Some(7));

		schedule.sync_alarm_logged();
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-schedule-dirty", Some(7)),
			None
		);

		schedule.sync_alarm_logged();
		assert_no_alarm(&mut rx);

		schedule.at(123, "tick", b"args");
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-schedule-dirty", Some(7)),
			Some(123)
		);

		schedule.sync_alarm_logged();
		assert_no_alarm(&mut rx);

		let event_id = schedule
			.next_event()
			.expect("scheduled event should exist")
			.event_id;
		assert!(schedule.cancel_scheduled_event(&event_id));
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-schedule-dirty", Some(7)),
			None
		);

		schedule.sync_alarm_logged();
		assert_no_alarm(&mut rx);
	}

	#[test]
	fn sync_future_alarm_uses_dirty_since_push_gate() {
		let schedule = ActorContext::new_for_schedule_tests("actor-future-alarm-dirty");
		let (handle, mut rx) = test_envoy_handle();
		schedule.configure_schedule_envoy(handle, Some(8));

		let future_ts = now_timestamp_ms() + 60_000;
		schedule.set_scheduled_events(vec![PersistedScheduleEvent {
			event_id: "event-1".to_owned(),
			timestamp: future_ts,
			action: "tick".to_owned(),
			args: Some(vec![1, 2, 3]),
		}]);

		schedule.sync_future_alarm_logged();
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-future-alarm-dirty", Some(8)),
			Some(future_ts)
		);

		schedule.sync_future_alarm_logged();
		assert_no_alarm(&mut rx);
	}
}
