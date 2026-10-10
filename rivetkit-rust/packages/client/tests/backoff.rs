use rivetkit_client::{Backoff, BackoffConfig, Client, ClientConfig};
use std::time::Duration;

#[test]
fn default_backoff_config_matches_standard_policy() {
	let config = BackoffConfig::default();
	assert_eq!(config.initial_delay, Duration::from_secs(1));
	assert_eq!(config.max_delay, Duration::from_secs(30));
	assert_eq!(config.multiplier, 2.0);
	assert_eq!(config.max_retries, None);
	assert_eq!(config.jitter_factor, 0.0);
}

#[test]
fn backoff_builder_methods_work_as_expected() {
	let config = BackoffConfig::new(Duration::from_millis(500), Duration::from_secs(10))
		.multiplier(1.5)
		.max_retries(Some(5))
		.jitter(true);

	assert_eq!(config.initial_delay, Duration::from_millis(500));
	assert_eq!(config.max_delay, Duration::from_secs(10));
	assert_eq!(config.multiplier, 1.5);
	assert_eq!(config.max_retries, Some(5));
	assert_eq!(config.jitter_factor, 0.2);

	let custom_jitter = config.jitter_factor(0.35);
	assert_eq!(custom_jitter.jitter_factor, 0.35);

	// Test silent clamping on invalid builder inputs
	let clamped_multiplier = BackoffConfig::default().multiplier(0.1);
	assert_eq!(clamped_multiplier.multiplier, 1.0);

	let clamped_jitter_low = BackoffConfig::default().jitter_factor(-1.0);
	assert_eq!(clamped_jitter_low.jitter_factor, 0.0);

	let clamped_jitter_high = BackoffConfig::default().jitter_factor(3.0);
	assert_eq!(clamped_jitter_high.jitter_factor, 1.0);
}

#[test]
fn backoff_exponential_growth_and_capping() {
	let config =
		BackoffConfig::new(Duration::from_millis(100), Duration::from_millis(600)).multiplier(2.0);
	let mut backoff = Backoff::from_config(config);

	assert_eq!(backoff.attempt(), 0);
	assert_eq!(backoff.delay(), Duration::from_millis(100));

	// Attempt 1: yields 100ms, advances next base to 200ms
	let dur1 = backoff.step().expect("step 1");
	assert_eq!(dur1, Duration::from_millis(100));
	assert_eq!(backoff.attempt(), 1);
	assert_eq!(backoff.delay(), Duration::from_millis(200));

	// Attempt 2: yields 200ms, advances next base to 400ms
	let dur2 = backoff.step().expect("step 2");
	assert_eq!(dur2, Duration::from_millis(200));
	assert_eq!(backoff.attempt(), 2);
	assert_eq!(backoff.delay(), Duration::from_millis(400));

	// Attempt 3: yields 400ms, advances next base capped at 600ms
	let dur3 = backoff.step().expect("step 3");
	assert_eq!(dur3, Duration::from_millis(400));
	assert_eq!(backoff.attempt(), 3);
	assert_eq!(backoff.delay(), Duration::from_millis(600));

	// Attempt 4: yields 600ms, remains capped at 600ms
	let dur4 = backoff.step().expect("step 4");
	assert_eq!(dur4, Duration::from_millis(600));
	assert_eq!(backoff.attempt(), 4);
	assert_eq!(backoff.delay(), Duration::from_millis(600));
}

#[test]
fn backoff_respects_max_retries() {
	let config = BackoffConfig::new(Duration::from_millis(10), Duration::from_millis(100))
		.max_retries(Some(3));
	let mut backoff = Backoff::from_config(config);

	assert!(backoff.can_retry());
	assert!(backoff.step().is_some()); // attempt 1
	assert!(backoff.can_retry());
	assert!(backoff.step().is_some()); // attempt 2
	assert!(backoff.can_retry());
	assert!(backoff.step().is_some()); // attempt 3

	// At 3 attempts, max is reached
	assert!(!backoff.can_retry());
	assert!(backoff.step().is_none());
	assert_eq!(backoff.attempt(), 3);
}

#[test]
fn backoff_reset_restores_initial_state() {
	let config = BackoffConfig::new(Duration::from_millis(100), Duration::from_millis(800));
	let mut backoff = Backoff::from_config(config);

	backoff.step();
	backoff.step();
	backoff.step();
	assert_eq!(backoff.attempt(), 3);
	assert!(backoff.delay() > Duration::from_millis(100));

	backoff.reset();
	assert_eq!(backoff.attempt(), 0);
	assert_eq!(backoff.delay(), Duration::from_millis(100));
	assert!(backoff.can_retry());
}

#[test]
fn backoff_with_jitter_stays_within_bounds() {
	let config =
		BackoffConfig::new(Duration::from_millis(1000), Duration::from_secs(10)).jitter_factor(0.2); // +/- 20%
	let mut backoff = Backoff::from_config(config);

	let mut seen_values = std::collections::HashSet::new();
	for _ in 0..50 {
		let dur = backoff.step().expect("step");
		// 1000ms +/- 20% = [800ms, 1200ms]
		assert!(
			dur >= Duration::from_millis(800) && dur <= Duration::from_millis(1200),
			"duration {dur:?} outside jitter range [800ms, 1200ms]"
		);
		seen_values.insert(dur.as_millis());
		backoff.reset();
	}

	assert!(
		seen_values.len() > 1,
		"expected multiple distinct jitter durations, got only {}",
		seen_values.len()
	);
}

#[test]
fn backoff_with_jitter_clamped_to_max_delay() {
	let config = BackoffConfig::new(Duration::from_millis(500), Duration::from_millis(500))
		.multiplier(2.0)
		.jitter_factor(0.5); // base is 500ms, +/- 50% jitter would reach 750ms without clamping
	let mut backoff = Backoff::from_config(config);

	let mut seen_values = std::collections::HashSet::new();
	for _ in 0..100 {
		let dur = backoff.step().expect("step");
		assert!(
			dur <= Duration::from_millis(500),
			"jittered duration {dur:?} exceeded max_delay of 500ms"
		);
		seen_values.insert(dur.as_millis());
		backoff.reset();
	}

	assert!(
		seen_values.len() > 1,
		"expected multiple distinct jitter durations below cap, got only {}",
		seen_values.len()
	);
}

#[test]
fn client_config_reconnect_builder_integration() {
	let client_config = ClientConfig::new("http://127.0.0.1:6420")
		.reconnect_delays(Duration::from_millis(250), Duration::from_secs(15));

	let backoff = client_config
		.reconnect_backoff
		.as_ref()
		.expect("backoff set");
	assert_eq!(backoff.initial_delay, Duration::from_millis(250));
	assert_eq!(backoff.max_delay, Duration::from_secs(15));

	let disabled_config = ClientConfig::new("http://127.0.0.1:6420").disable_reconnect();
	let disabled_backoff = disabled_config
		.reconnect_backoff
		.as_ref()
		.expect("backoff set");
	assert_eq!(disabled_backoff.max_retries, Some(0));

	let client = Client::new(client_config);
	let handle = client
		.get("test", vec!["key1".to_string()], Default::default())
		.expect("handle");
	assert_eq!(
		handle.reconnect_backoff().initial_delay,
		Duration::from_millis(250)
	);
	assert_eq!(
		handle.reconnect_backoff().max_delay,
		Duration::from_secs(15)
	);
}

#[test]
fn direct_struct_construction_is_normalized_on_backoff_creation() {
	let custom = BackoffConfig {
		initial_delay: Duration::from_secs(5),
		max_delay: Duration::from_millis(500), // invalid: max < initial
		multiplier: 0.2,                       // invalid: < 1.0
		max_retries: Some(2),
		jitter_factor: 10.0, // invalid: > 1.0
	};

	let mut backoff = Backoff::from_config(custom);
	assert_eq!(backoff.config().max_delay, Duration::from_secs(5));
	assert_eq!(backoff.config().multiplier, 1.0);
	assert_eq!(backoff.config().jitter_factor, 1.0);

	// Ensure step() works reliably with normalized config
	let dur1 = backoff.step().expect("step 1");
	assert!(dur1 <= Duration::from_secs(10));
}

#[tokio::test]
async fn integration_max_retries_stops_reconnect_loop() {
	use axum::{http::StatusCode, routing::any, Router};
	use rivetkit_client::GetOrCreateOptions;
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	};
	use tokio::{net::TcpListener, time::sleep};

	let attempts = Arc::new(AtomicUsize::new(0));
	let app = Router::new().route(
		"/gateway/{actor_id}/connect",
		any({
			let attempts = attempts.clone();
			move || {
				attempts.fetch_add(1, Ordering::SeqCst);
				async { StatusCode::INTERNAL_SERVER_ERROR }
			}
		}),
	);

	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	let client_config = ClientConfig::new(format!("http://{addr}"))
		.disable_metadata_lookup(true)
		.reconnect_backoff(
			BackoffConfig::default()
				.initial_delay(Duration::from_millis(5))
				.max_delay(Duration::from_millis(20))
				.max_retries(Some(2)), // 1 initial + 2 retries = 3 total attempts
		);

	let client = Client::new(client_config);
	let actor = client
		.get_or_create(
			"test-actor",
			vec!["key1".to_string()],
			GetOrCreateOptions::default(),
		)
		.unwrap();

	let _conn = actor.connect();

	// Wait for reconnect attempts to complete (5ms + 10ms + buffer)
	sleep(Duration::from_millis(150)).await;

	// Assert exactly 3 connection attempts: initial + 2 retries
	assert_eq!(
		attempts.load(Ordering::SeqCst),
		3,
		"expected exactly 3 attempts (1 initial + 2 retries)"
	);

	// Ensure no further reconnection attempts occur
	sleep(Duration::from_millis(100)).await;
	assert_eq!(attempts.load(Ordering::SeqCst), 3);

	server.abort();
}

#[tokio::test]
async fn integration_disable_reconnect_attempts_once() {
	use axum::{http::StatusCode, routing::any, Router};
	use rivetkit_client::GetOrCreateOptions;
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	};
	use tokio::{net::TcpListener, time::sleep};

	let attempts = Arc::new(AtomicUsize::new(0));
	let app = Router::new().route(
		"/gateway/{actor_id}/connect",
		any({
			let attempts = attempts.clone();
			move || {
				attempts.fetch_add(1, Ordering::SeqCst);
				async { StatusCode::INTERNAL_SERVER_ERROR }
			}
		}),
	);

	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	let client_config = ClientConfig::new(format!("http://{addr}"))
		.disable_metadata_lookup(true)
		.disable_reconnect();

	let client = Client::new(client_config);
	let actor = client
		.get_or_create(
			"test-actor",
			vec!["key1".to_string()],
			GetOrCreateOptions::default(),
		)
		.unwrap();

	let _conn = actor.connect();

	// Wait and verify only the initial attempt occurred
	sleep(Duration::from_millis(150)).await;
	assert_eq!(
		attempts.load(Ordering::SeqCst),
		1,
		"disable_reconnect should allow the initial attempt but zero retries"
	);

	server.abort();
}

#[tokio::test]
async fn integration_handle_connect_with_backoff_override() {
	use axum::{http::StatusCode, routing::any, Router};
	use rivetkit_client::GetOrCreateOptions;
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	};
	use tokio::{net::TcpListener, time::sleep};

	let attempts = Arc::new(AtomicUsize::new(0));
	let app = Router::new().route(
		"/gateway/{actor_id}/connect",
		any({
			let attempts = attempts.clone();
			move || {
				attempts.fetch_add(1, Ordering::SeqCst);
				async { StatusCode::INTERNAL_SERVER_ERROR }
			}
		}),
	);

	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	// Client has disable_reconnect by default
	let client = Client::new(
		ClientConfig::new(format!("http://{addr}"))
			.disable_metadata_lookup(true)
			.disable_reconnect(),
	);

	let actor = client
		.get_or_create(
			"test-actor",
			vec!["key1".to_string()],
			GetOrCreateOptions::default(),
		)
		.unwrap();

	// Per-connection override allowing 1 retry (2 attempts total)
	let _conn = actor.connect_with_backoff(
		BackoffConfig::default()
			.initial_delay(Duration::from_millis(5))
			.max_retries(Some(1)),
	);

	sleep(Duration::from_millis(150)).await;
	assert_eq!(
		attempts.load(Ordering::SeqCst),
		2,
		"connect_with_backoff override should permit 2 attempts (1 initial + 1 retry)"
	);

	server.abort();
}

#[tokio::test]
async fn integration_disable_reconnect_stops_after_connection_closes() {
	use axum::{
		extract::ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade},
		routing::any,
		Router,
	};
	use futures_util::SinkExt;
	use rivetkit_client::GetOrCreateOptions;
	use rivetkit_client_protocol as wire;
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	};
	use tokio::{net::TcpListener, time::sleep};
	use vbare::OwnedVersionedData;

	let connect_count = Arc::new(AtomicUsize::new(0));
	let connect_count_handler = connect_count.clone();

	let app = Router::new().route(
		"/gateway/{actor_id}/connect",
		any(move |ws: WebSocketUpgrade| {
			let count = connect_count_handler.clone();
			async move {
				count.fetch_add(1, Ordering::SeqCst);
				ws.protocols(["rivet"])
					.on_upgrade(|mut socket: WebSocket| async move {
						let payload = wire::versioned::ToClient::wrap_latest(wire::ToClient {
							body: wire::ToClientBody::Init(wire::Init {
								actor_id: "test-actor".to_owned(),
								connection_id: "conn-1".to_owned(),
							}),
						})
						.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
						.unwrap();
						socket
							.send(AxumWsMessage::Binary(payload.into()))
							.await
							.unwrap();
						// Close connection from server side
						let _ = socket.close().await;
					})
			}
		}),
	);

	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	let client_config = ClientConfig::new(format!("http://{addr}"))
		.disable_metadata_lookup(true)
		.disable_reconnect();

	let client = Client::new(client_config);
	let actor = client
		.get_or_create(
			"test-actor",
			vec!["key1".to_string()],
			GetOrCreateOptions::default(),
		)
		.unwrap();

	let _conn = actor.connect();

	// Wait for the connection to establish, close, and verify no reconnection occurs
	sleep(Duration::from_millis(200)).await;

	assert_eq!(
		connect_count.load(Ordering::SeqCst),
		1,
		"disable_reconnect should not reconnect after established connection closes"
	);

	server.abort();
}

#[tokio::test]
async fn integration_reconnect_after_healthy_close_respects_initial_delay() {
	use axum::{
		extract::ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade},
		routing::any,
		Router,
	};
	use futures_util::SinkExt;
	use rivetkit_client::GetOrCreateOptions;
	use rivetkit_client_protocol as wire;
	use std::sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	};
	use tokio::{
		net::TcpListener,
		sync::mpsc,
		time::{sleep, timeout, Instant},
	};
	use vbare::OwnedVersionedData;

	let attempts = Arc::new(AtomicUsize::new(0));
	let attempts_handler = attempts.clone();

	let (closed_tx, mut closed_rx) = mpsc::unbounded_channel::<Instant>();
	let closed_tx = Arc::new(tokio::sync::Mutex::new(Some(closed_tx)));

	let (reconnect_tx, mut reconnect_rx) = mpsc::unbounded_channel::<Instant>();
	let reconnect_tx = Arc::new(reconnect_tx);

	let app = Router::new().route(
		"/gateway/{actor_id}/connect",
		any(move |ws: WebSocketUpgrade| {
			let attempts = attempts_handler.clone();
			let closed_tx = closed_tx.clone();
			let reconnect_tx = reconnect_tx.clone();
			async move {
				let attempt_num = attempts.fetch_add(1, Ordering::SeqCst);
				if attempt_num == 0 {
					ws.protocols(["rivet"])
						.on_upgrade(move |mut socket: WebSocket| async move {
							let payload = wire::versioned::ToClient::wrap_latest(wire::ToClient {
								body: wire::ToClientBody::Init(wire::Init {
									actor_id: "test-actor".to_owned(),
									connection_id: "conn-1".to_owned(),
								}),
							})
							.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
							.unwrap();
							socket
								.send(AxumWsMessage::Binary(payload.into()))
								.await
								.unwrap();

							// Server closes the connection and records timestamp
							let _ = socket.close().await;
							if let Some(tx) = closed_tx.lock().await.take() {
								let _ = tx.send(Instant::now());
							}
						})
				} else {
					let _ = reconnect_tx.send(Instant::now());
					ws.protocols(["rivet"])
						.on_upgrade(|_socket: WebSocket| async move {})
				}
			}
		}),
	);

	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	let client_config = ClientConfig::new(format!("http://{addr}"))
		.disable_metadata_lookup(true)
		.reconnect_backoff(
			BackoffConfig::default()
				.initial_delay(Duration::from_millis(100))
				.max_delay(Duration::from_millis(500))
				.jitter_factor(0.0), // zero jitter for deterministic delay assertion
		);

	let client = Client::new(client_config);
	let actor = client
		.get_or_create(
			"test-actor",
			vec!["key1".to_string()],
			GetOrCreateOptions::default(),
		)
		.unwrap();

	let _conn = actor.connect();

	// Wait for the first connection to close
	let closed_at = timeout(Duration::from_secs(2), closed_rx.recv())
		.await
		.expect("first connection did not close in time")
		.expect("channel closed");

	// At 25ms, reconnect attempt must NOT have happened yet (initial_delay is 100ms)
	sleep(Duration::from_millis(25)).await;
	assert_eq!(
		attempts.load(Ordering::SeqCst),
		1,
		"reconnect must not happen immediately after healthy connection closes"
	);

	// Wait for reconnect attempt to occur
	let reconnected_at = timeout(Duration::from_secs(2), reconnect_rx.recv())
		.await
		.expect("reconnect did not occur within timeout")
		.expect("channel closed");

	let elapsed = reconnected_at.duration_since(closed_at);
	assert!(
		elapsed >= Duration::from_millis(80),
		"reconnect happened too fast ({elapsed:?}), expected >= 80ms for 100ms initial_delay"
	);

	server.abort();
}
