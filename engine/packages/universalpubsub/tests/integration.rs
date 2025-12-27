use anyhow::*;
use futures_util::StreamExt;
use rivet_error::RivetError;
use rivet_test_deps_docker::{TestDatabase, TestPubSub};
use std::{
	sync::Arc,
	time::{Duration, Instant},
};
use universalpubsub::{NextOutput, PubSub, PublishOpts};
use uuid::Uuid;

fn setup_logging() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter("debug")
		.with_ansi(false)
		.with_test_writer()
		.try_init();
}

#[tokio::test]
async fn test_nats_driver_with_memory() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (pubsub_config, docker_config) = TestPubSub::Nats.config(test_id, 1).await.unwrap();
	let mut docker = docker_config.unwrap();
	docker.start().await.unwrap();
	tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

	let rivet_config::config::PubSub::Nats(nats) = pubsub_config else {
		unreachable!();
	};

	use std::str::FromStr;
	let server_addrs = nats
		.addresses
		.iter()
		.map(|addr| format!("nats://{addr}"))
		.map(|url| async_nats::ServerAddr::from_str(url.as_ref()))
		.collect::<Result<Vec<_>, _>>()
		.unwrap();

	let driver = universalpubsub::driver::nats::NatsDriver::connect(
		async_nats::ConnectOptions::new(),
		&server_addrs[..],
	)
	.await
	.unwrap();
	let pubsub = PubSub::new_with_memory_optimization(Arc::new(driver), true);

	test_inner(&pubsub).await;
}

#[tokio::test]
async fn test_nats_driver_without_memory() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (pubsub_config, docker_config) = TestPubSub::Nats.config(test_id, 1).await.unwrap();
	let mut docker = docker_config.unwrap();
	docker.start().await.unwrap();
	tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

	let rivet_config::config::PubSub::Nats(nats) = pubsub_config else {
		unreachable!();
	};

	use std::str::FromStr;
	let server_addrs = nats
		.addresses
		.iter()
		.map(|addr| format!("nats://{addr}"))
		.map(|url| async_nats::ServerAddr::from_str(url.as_ref()))
		.collect::<Result<Vec<_>, _>>()
		.unwrap();

	let driver = universalpubsub::driver::nats::NatsDriver::connect(
		async_nats::ConnectOptions::new(),
		&server_addrs[..],
	)
	.await
	.unwrap();
	let pubsub = PubSub::new_with_memory_optimization(Arc::new(driver), false);

	test_inner(&pubsub).await;
}

#[tokio::test]
async fn test_postgres_driver_with_memory() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (db_config, docker_config) = TestDatabase::Postgres.config(test_id, 1).await.unwrap();
	let mut docker = docker_config.unwrap();
	docker.start().await.unwrap();
	tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

	let rivet_config::config::Database::Postgres(pg) = db_config else {
		unreachable!();
	};
	let url = pg.url.read().clone();

	let driver =
		universalpubsub::driver::postgres::PostgresDriver::connect(url, true, None, None, None)
			.await
			.unwrap();
	let pubsub = PubSub::new_with_memory_optimization(Arc::new(driver), true);

	test_inner(&pubsub).await;
}

#[tokio::test]
async fn test_postgres_driver_without_memory() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (db_config, docker_config) = TestDatabase::Postgres.config(test_id, 1).await.unwrap();
	let mut docker = docker_config.unwrap();
	docker.start().await.unwrap();
	tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

	let rivet_config::config::Database::Postgres(pg) = db_config else {
		unreachable!();
	};
	let url = pg.url.read().clone();

	let driver =
		universalpubsub::driver::postgres::PostgresDriver::connect(url, false, None, None, None)
			.await
			.unwrap();
	let pubsub = PubSub::new_with_memory_optimization(Arc::new(driver), false);

	test_inner(&pubsub).await;
}

#[tokio::test]
async fn test_memory_driver() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (pubsub_config, _docker_config) = TestPubSub::Memory.config(test_id, 1).await.unwrap();
	let rivet_config::config::PubSub::Memory(memory) = pubsub_config else {
		unreachable!();
	};

	let driver = universalpubsub::driver::memory::MemoryDriver::new(memory.channel);
	let pubsub = PubSub::new(Arc::new(driver));

	test_inner(&pubsub).await;
}

async fn test_inner(pubsub: &PubSub) {
	let start = Instant::now();
	test_basic_pub_sub(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_basic_pub_sub completed");

	let start = Instant::now();
	test_multiple_subscribers(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_multiple_subscribers completed");

	let start = Instant::now();
	test_unsubscribe(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_unsubscribe completed");

	let start = Instant::now();
	test_request_response(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_request_response completed");

	let start = Instant::now();
	test_multiple_request_response(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_multiple_request_response completed");

	let start = Instant::now();
	test_request_timeout(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_request_timeout completed");

	let start = Instant::now();
	test_large_payloads(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_large_payloads completed");
}

async fn test_basic_pub_sub(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing basic pub/sub");

	// Subscribe to a subject
	let mut subscriber = pubsub.subscribe("test.subject").await?;

	// Publish a message
	let message = b"Hello, World!";
	pubsub
		.publish("test.subject", message, PublishOpts::broadcast())
		.await?;
	pubsub.flush().await?;

	// Receive the message
	match subscriber.next().await? {
		NextOutput::Message(msg) => {
			assert_eq!(msg.payload, message);
		}
		NextOutput::Unsubscribed => {
			panic!("unexpected unsubscribe");
		}
	}

	Ok(())
}

async fn test_multiple_subscribers(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing multiple subscribers");

	// Create multiple subscribers
	let mut sub1 = pubsub.subscribe("test.multi").await?;
	let mut sub2 = pubsub.subscribe("test.multi").await?;

	// Publish a message
	let message = b"Broadcast message";
	pubsub
		.publish("test.multi", message, PublishOpts::broadcast())
		.await?;
	pubsub.flush().await?;

	// Both subscribers should receive the message
	match sub1.next().await? {
		NextOutput::Message(msg) => {
			assert_eq!(msg.payload, message);
		}
		NextOutput::Unsubscribed => {
			panic!("unexpected unsubscribe for sub1");
		}
	}

	match sub2.next().await? {
		NextOutput::Message(msg) => {
			assert_eq!(msg.payload, message);
		}
		NextOutput::Unsubscribed => {
			panic!("unexpected unsubscribe for sub2");
		}
	}

	Ok(())
}

async fn test_unsubscribe(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing unsubscribe");

	// Subscribe to a subject
	let mut subscriber = pubsub.subscribe("test.unsub").await?;

	// Publish a message
	let message = b"First message";
	pubsub
		.publish("test.unsub", message, PublishOpts::broadcast())
		.await?;
	pubsub.flush().await?;

	// Receive the first message
	match subscriber.next().await? {
		NextOutput::Message(msg) => {
			assert_eq!(msg.payload, message);
		}
		NextOutput::Unsubscribed => {
			panic!("unexpected unsubscribe");
		}
	}

	// Drop the subscriber to unsubscribe
	drop(subscriber);

	// Create a new subscriber and verify it works
	let mut new_subscriber = pubsub.subscribe("test.unsub").await?;

	// Publish another message
	let message2 = b"Second message";
	pubsub
		.publish("test.unsub", message2, PublishOpts::broadcast())
		.await?;
	pubsub.flush().await?;

	// New subscriber should receive the message
	match new_subscriber.next().await? {
		NextOutput::Message(msg) => {
			assert_eq!(msg.payload, message2);
		}
		NextOutput::Unsubscribed => {
			panic!("unexpected unsubscribe for new subscriber");
		}
	}

	Ok(())
}

async fn test_request_response(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing base request/response");

	let payload = b"request payload";

	{
		let pubsub = pubsub.clone();
		let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
		tokio::spawn(async move {
			let mut sub = pubsub.subscribe("test.request_response").await.unwrap();
			ready_tx.send(()).unwrap();
			while let NextOutput::Message(msg) = sub.next().await.unwrap() {
				// Reply with the same payload back
				let _ = msg.reply(&msg.payload).await;
			}
		});
		ready_rx.await.unwrap();
	}

	let req = pubsub
		.request("test.request_response", payload)
		.await
		.unwrap();
	assert_eq!(req.payload, payload);

	Ok(())
}

async fn test_multiple_request_response(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing multiple request/response");

	// TODO: This fails on postgres for high numbers (too many clients)
	futures_util::stream::iter(0..5)
		.map(|i| async move {
			let mut payload = b"request payload ".to_vec();
			payload.extend(i.to_string().as_bytes());

			{
				let pubsub = pubsub.clone();
				let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
				tokio::spawn(async move {
					let mut sub = pubsub.subscribe("test.request_response").await.unwrap();
					ready_tx.send(()).unwrap();
					while let NextOutput::Message(msg) = sub.next().await.unwrap() {
						// Reply with the same payload back
						let _ = msg.reply(&msg.payload).await;
					}
				});
				ready_rx.await.unwrap();
			}

			let req = pubsub
				.request("test.request_response", &payload)
				.await
				.unwrap();
			assert_eq!(req.payload, payload);
		})
		.buffer_unordered(50)
		.collect::<()>()
		.await;

	Ok(())
}

async fn test_request_timeout(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing request timeout");

	// Requires a subscriber to ensure we don't get NoResponders error
	let mut _sub = pubsub.subscribe("test.request_timeout").await?;

	let payload = b"slow request";
	let timeout = Duration::from_millis(50);

	let result = pubsub
		.request_with_timeout("test.request_timeout", payload, timeout)
		.await;

	let err = result.err().unwrap();
	let err = err
		.downcast_ref::<RivetError>()
		.expect("expected errors::Ups");
	assert_eq!(err.group(), "ups");
	assert_eq!(err.code(), "request_timeout");

	Ok(())
}

async fn test_large_payloads(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing large payloads with chunking");

	// Use a base size that works with all drivers
	// Postgres has the smallest limit at 8KB
	let base_size = 5000; // Use 5KB as base to ensure we're under the limit

	// Test 1x max size
	test_payload_size(pubsub, base_size, "1x").await?;

	// Test 2x max size
	test_payload_size(pubsub, base_size * 2, "2x").await?;

	// Test 2.5x max size
	test_payload_size(pubsub, (base_size as f64 * 2.5) as usize, "2.5x").await?;

	Ok(())
}

async fn test_payload_size(pubsub: &PubSub, size: usize, label: &str) -> Result<()> {
	tracing::info!(size, label, "testing payload size");

	// Create a payload of the specified size
	let mut payload = vec![0u8; size];
	// Fill with a pattern to verify integrity
	for (i, byte) in payload.iter_mut().enumerate() {
		*byte = (i % 256) as u8;
	}

	// Subscribe to a test subject
	let mut subscriber = pubsub.subscribe(&format!("test.large.{}", label)).await?;

	// Publish the large message
	pubsub
		.publish(
			&format!("test.large.{}", label),
			&payload,
			PublishOpts::broadcast(),
		)
		.await?;
	pubsub.flush().await?;

	// Receive and verify the message
	match subscriber.next().await? {
		NextOutput::Message(msg) => {
			assert_eq!(
				msg.payload.len(),
				size,
				"payload size mismatch for {}",
				label
			);
			assert_eq!(
				msg.payload, payload,
				"payload content mismatch for {}",
				label
			);
		}
		NextOutput::Unsubscribed => {
			panic!("unexpected unsubscribe for {}", label);
		}
	}

	Ok(())
}
