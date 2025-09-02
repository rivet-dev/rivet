use anyhow::*;
use rivet_error::RivetError;
use rivet_test_deps_docker::{TestDatabase, TestPubSub};
use std::{
	sync::Arc,
	time::{Duration, Instant},
};
use universalpubsub::{NextOutput, PubSub};
use uuid::Uuid;

fn setup_logging() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter("debug")
		.with_ansi(false)
		.with_test_writer()
		.try_init();
}

#[tokio::test]
async fn test_nats_driver() {
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
	let pubsub = PubSub::new(Arc::new(driver));

	test_inner(&pubsub).await;
}

#[tokio::test]
async fn test_postgres_driver_with_memory() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (db_config, docker_config) = TestDatabase::Postgres.config(test_id, 1).await.unwrap();
	let mut docker = docker_config.unwrap();
	docker.start().await.unwrap();
	tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

	let rivet_config::config::Database::Postgres(pg) = db_config else {
		unreachable!();
	};
	let url = pg.url.read().clone();

	let driver = universalpubsub::driver::postgres::PostgresDriver::connect(url, true)
		.await
		.unwrap();
	let pubsub = PubSub::new(Arc::new(driver));

	test_inner(&pubsub).await;
}

#[tokio::test]
async fn test_postgres_driver_without_memory() {
	setup_logging();

	let test_id = Uuid::new_v4();
	let (db_config, docker_config) = TestDatabase::Postgres.config(test_id, 1).await.unwrap();
	let mut docker = docker_config.unwrap();
	docker.start().await.unwrap();
	tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

	let rivet_config::config::Database::Postgres(pg) = db_config else {
		unreachable!();
	};
	let url = pg.url.read().clone();

	let driver = universalpubsub::driver::postgres::PostgresDriver::connect(url, false)
		.await
		.unwrap();
	let pubsub = PubSub::new(Arc::new(driver));

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
	test_request_timeout(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_request_timeout completed");

	let start = Instant::now();
	test_no_responders(&pubsub).await.unwrap();
	tracing::info!(duration_ms = ?start.elapsed().as_millis(), "test_no_responders completed");
}

async fn test_basic_pub_sub(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing basic pub/sub");

	// Subscribe to a subject
	let mut subscriber = pubsub.subscribe("test.subject").await?;

	// Publish a message
	let message = b"Hello, World!";
	pubsub.publish("test.subject", message).await?;
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
	pubsub.publish("test.multi", message).await?;
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
	pubsub.publish("test.unsub", message).await?;
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
	pubsub.publish("test.unsub", message2).await?;
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

async fn test_no_responders(pubsub: &PubSub) -> Result<()> {
	tracing::info!("testing no responders error");

	let result = pubsub
		.request("test.no_responders", b"no one listening")
		.await;
	assert!(
		result.is_err(),
		"Expected request to fail with no responders"
	);

	let err = result.err().unwrap();
	let err = err
		.downcast_ref::<RivetError>()
		.expect("expected errors::Ups");
	assert_eq!(err.group(), "ups");
	assert_eq!(err.code(), "no_responders");

	Ok(())
}
