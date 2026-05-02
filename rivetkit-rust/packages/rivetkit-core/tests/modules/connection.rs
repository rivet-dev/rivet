use super::*;

mod moved_tests {
	use std::collections::BTreeMap;
	use std::sync::Arc;
	use std::sync::Mutex;
	use std::time::Duration;

	use anyhow::Result;
	use tokio::sync::oneshot;
	use tokio::time::sleep;

	use super::{
		ConnHandle, ConnectionManager, EventSendCallback, HibernatableConnectionMetadata,
		OutgoingEvent, PersistedConnection, decode_persisted_connection,
		encode_persisted_connection,
	};
	use crate::actor::callbacks::ActorInstanceCallbacks;
	use crate::actor::config::ActorConfig;
	use crate::actor::context::ActorContext;
	use crate::actor::keys::make_connection_key;
	use crate::actor::context::tests::new_with_kv;

	const PERSISTED_CONNECTION_HEX: &str = "040006636f6e6e2d310201020203040107757064617465640401020304040506070809000a00032f77730106782d746573740131";

	fn hex(bytes: &[u8]) -> String {
		bytes.iter().map(|byte| format!("{byte:02x}")).collect()
	}

	#[test]
	fn send_uses_configured_event_sender() {
		let sent = Arc::new(Mutex::new(Vec::<OutgoingEvent>::new()));
		let sent_clone = sent.clone();
		let conn = ConnHandle::new("conn-1", b"params".to_vec(), b"state".to_vec(), true);
		let sender: EventSendCallback = Arc::new(move |event| {
			sent_clone
				.lock()
				.expect("sent events lock poisoned")
				.push(event);
			Ok(())
		});

		conn.configure_event_sender(Some(sender));
		conn.send("updated", b"payload");

		assert_eq!(
			*sent.lock().expect("sent events lock poisoned"),
			vec![OutgoingEvent {
				name: "updated".to_owned(),
				args: b"payload".to_vec(),
			}]
		);
		assert_eq!(conn.params(), b"params");
		assert_eq!(conn.state(), b"state");
		assert!(conn.is_hibernatable());
	}

	#[tokio::test]
	async fn disconnect_returns_configuration_error_without_handler() {
		let conn = ConnHandle::default();
		let error = conn
			.disconnect(None)
			.await
			.expect_err("disconnect should fail without a handler");

		assert!(
			error
				.to_string()
				.contains("connection disconnect handler is not configured")
		);
	}

	#[tokio::test]
	async fn disconnect_uses_configured_handler() -> Result<()> {
		let conn = ConnHandle::new("conn-1", Vec::new(), Vec::new(), false);
		conn.configure_disconnect_handler(Some(Arc::new(|reason| {
			Box::pin(async move {
				assert_eq!(reason.as_deref(), Some("bye"));
				Ok(())
			})
		})));

		conn.disconnect(Some("bye")).await
	}

	#[test]
	fn persisted_connection_round_trips_with_embedded_version() {
		let mut headers = BTreeMap::new();
		headers.insert("x-test".to_owned(), "1".to_owned());
		let persisted = PersistedConnection {
			id: "conn-1".to_owned(),
			parameters: vec![1, 2],
			state: vec![3, 4],
			subscriptions: vec![super::PersistedSubscription {
				event_name: "updated".to_owned(),
			}],
			gateway_id: vec![1, 2, 3, 4],
			request_id: vec![5, 6, 7, 8],
			server_message_index: 9,
			client_message_index: 10,
			request_path: "/ws".to_owned(),
			request_headers: headers,
		};

		let encoded =
			encode_persisted_connection(&persisted).expect("persisted connection should encode");
		assert_eq!(hex(&encoded), PERSISTED_CONNECTION_HEX);
		let decoded =
			decode_persisted_connection(&encoded).expect("persisted connection should decode");

		assert_eq!(decoded, persisted);
	}

	#[test]
	fn make_connection_key_matches_typescript_layout() {
		assert_eq!(make_connection_key("conn-1"), b"\x02conn-1".to_vec());
	}

	#[tokio::test]
	async fn connect_runs_connection_lifecycle_callbacks() -> Result<()> {
		let ctx = ActorContext::default();
		let manager = ConnectionManager::default();

		let before_called = Arc::new(Mutex::new(false));
		let before_called_clone = before_called.clone();
		let connect_called = Arc::new(Mutex::new(false));
		let connect_called_clone = connect_called.clone();

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_before_connect = Some(Box::new(move |request| {
			let before_called = before_called_clone.clone();
			Box::pin(async move {
				assert_eq!(request.params, b"params".to_vec());
				*before_called.lock().expect("before connect lock poisoned") = true;
				Ok(())
			})
		}));
		callbacks.on_connect = Some(Box::new(move |request| {
			let connect_called = connect_called_clone.clone();
			Box::pin(async move {
				assert_eq!(request.conn.params(), b"params".to_vec());
				*connect_called.lock().expect("connect lock poisoned") = true;
				Ok(())
			})
		}));

		manager.configure_runtime(ActorConfig::default(), Arc::new(callbacks));
		let conn = manager
			.connect_with_state(&ctx, b"params".to_vec(), false, None, async {
				Ok(b"state".to_vec())
			})
			.await?;

		assert_eq!(conn.state(), b"state".to_vec());
		assert!(*before_called.lock().expect("before connect lock poisoned"));
		assert!(*connect_called.lock().expect("connect lock poisoned"));
		assert_eq!(manager.list().len(), 1);

		Ok(())
	}

	#[tokio::test]
	async fn connect_honors_callback_and_state_timeouts() {
		let ctx = ActorContext::default();
		let manager = ConnectionManager::default();
		let mut config = ActorConfig::default();
		config.on_before_connect_timeout = Duration::from_millis(10);
		config.create_conn_state_timeout = Duration::from_millis(10);
		config.on_connect_timeout = Duration::from_millis(10);

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_before_connect = Some(Box::new(|_| {
			Box::pin(async move {
				sleep(Duration::from_millis(50)).await;
				Ok(())
			})
		}));
		manager.configure_runtime(config.clone(), Arc::new(callbacks));

		let error = manager
			.connect_with_state(&ctx, Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect_err("on_before_connect should time out");
		assert!(error.to_string().contains("`on_before_connect` timed out"));

		let manager = ConnectionManager::default();
		manager.configure_runtime(config.clone(), Arc::new(ActorInstanceCallbacks::default()));
		let error = manager
			.connect_with_state(&ctx, Vec::new(), false, None, async {
				sleep(Duration::from_millis(50)).await;
				Ok(Vec::new())
			})
			.await
			.expect_err("create_conn_state should time out");
		assert!(error.to_string().contains("`create_conn_state` timed out"));

		let manager = ConnectionManager::default();
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_connect = Some(Box::new(|_| {
			Box::pin(async move {
				sleep(Duration::from_millis(50)).await;
				Ok(())
			})
		}));
		manager.configure_runtime(config, Arc::new(callbacks));
		let error = manager
			.connect_with_state(&ctx, Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect_err("on_connect should time out");
		assert!(error.to_string().contains("`on_connect` timed out"));
	}

	#[tokio::test]
	async fn managed_disconnect_removes_connection_and_clears_subscriptions() -> Result<()> {
		let ctx = ActorContext::default();
		let manager = ConnectionManager::default();
		let (tx, rx) = oneshot::channel::<ConnHandle>();
		let tx = Arc::new(Mutex::new(Some(tx)));
		let tx_clone = tx.clone();

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_disconnect = Some(Box::new(move |request| {
			let tx = tx_clone.clone();
			Box::pin(async move {
				if let Some(tx) = tx.lock().expect("disconnect sender lock poisoned").take() {
					let _ = tx.send(request.conn.clone());
				}
				Ok(())
			})
		}));
		manager.configure_runtime(ActorConfig::default(), Arc::new(callbacks));

		let conn = manager
			.connect_with_state(&ctx, b"params".to_vec(), false, None, async {
				Ok(b"state".to_vec())
			})
			.await?;
		conn.subscribe("updated");
		conn.disconnect(Some("bye")).await?;

		let disconnected = rx.await.expect("disconnect callback should receive conn");
		assert!(disconnected.subscriptions().is_empty());
		assert!(manager.list().is_empty());

		Ok(())
	}

	#[tokio::test]
	async fn connection_lifecycle_updates_prometheus_metrics() -> Result<()> {
		let ctx = new_with_kv(
			"actor-1",
			"conn-metrics",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		let conn = ctx
			.connect_conn(Vec::new(), false, None, async { Ok(Vec::new()) })
			.await?;
		conn.disconnect(None).await?;

		let metrics = ctx.render_metrics().expect("render metrics");
		let active_line = metrics
			.lines()
			.find(|line| line.starts_with("active_connections"))
			.expect("active connections metric line");
		let total_line = metrics
			.lines()
			.find(|line| line.starts_with("connections_total"))
			.expect("connections total metric line");

		assert!(active_line.ends_with(" 0"));
		assert!(total_line.ends_with(" 1"));
		Ok(())
	}

	#[test]
	fn restored_connection_keeps_hibernation_metadata() {
		let conn = ConnHandle::new("conn-1", b"params".to_vec(), b"state".to_vec(), true);
		conn.subscribe("updated");
		conn.configure_hibernation(Some(HibernatableConnectionMetadata {
			gateway_id: vec![1, 2, 3, 4],
			request_id: vec![5, 6, 7, 8],
			server_message_index: 9,
			client_message_index: 10,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::from([("x-test".to_owned(), "1".to_owned())]),
		}));

		let persisted = conn.persisted().expect("connection should persist");
		let restored = ConnHandle::from_persisted(persisted.clone());

		assert_eq!(restored.persisted(), Some(persisted));
		assert!(restored.is_subscribed("updated"));
	}
}
