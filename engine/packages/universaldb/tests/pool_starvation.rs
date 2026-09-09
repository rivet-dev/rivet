//! Regression coverage for connection pool starvation across commit submission.
//!
//! A follower transaction task pins one pooled connection for its read snapshot. If it keeps that
//! connection while awaiting the leader's verdict on its commit, enough concurrent writes hold every
//! slot in the pool the leader drain loop itself draws from, and no commit can ever be applied.

use std::{sync::Arc, time::Duration};

use futures_util::future::join_all;
use rivet_test_deps_docker::{TestDatabase, TestPubSub};
use tokio::sync::Barrier;
use tokio_postgres::NoTls;
use universaldb::{
	Database,
	driver::postgres::{NatsConfig, PostgresConfig},
	utils::IsolationLevel::*,
};
use uuid::Uuid;

/// Pool size used by every test here. Small enough that a handful of transactions exhaust the pool,
/// which is what makes the starvation deterministic rather than load-dependent.
const POOL_MAX_SIZE: usize = 4;

/// A config carrying the compiled universaldb commit protocol version.
///
/// `RuntimeProtocols::default()` reports version 0 so that a process which never negotiated cannot
/// silently reach the wire, which means a test has to supply the real version itself.
fn test_config() -> rivet_config::Config {
	rivet_config::Config::from_root_with_build_meta(
		rivet_config::config::Root::default(),
		rivet_config::BuildMeta::default(),
		rivet_config::RuntimeProtocols {
			universaldb_commit: rivet_config::RuntimeProtocol::new(
				rivet_config::RuntimeProtocolKind::UniversaldbCommit,
				rivet_universaldb_commit::PROTOCOL_VERSION,
			),
			..Default::default()
		},
	)
}

fn init_tracing() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter("info")
		.with_test_writer()
		.try_init();
}

/// Boot a Postgres container and return its connection string plus the docker handle, which must be
/// kept alive for the duration of the test.
async fn setup_postgres() -> (String, rivet_test_deps_docker::DockerRunConfig) {
	let (db_config, docker_config) = TestDatabase::Postgres
		.config(Uuid::new_v4(), 1)
		.await
		.unwrap();
	let mut docker_config = docker_config.unwrap();
	docker_config.start().await.unwrap();

	TestDatabase::Postgres
		.wait_for_ready(&docker_config)
		.await
		.unwrap();

	let rivet_config::config::Database::Postgres(postgres_config) = db_config else {
		unreachable!();
	};

	(postgres_config.url.read().clone(), docker_config)
}

/// Boot a NATS container and return the multi-node UniversalDB NATS config plus the docker handle.
async fn setup_nats() -> (NatsConfig, rivet_test_deps_docker::DockerRunConfig) {
	let (pubsub_config, docker_config) = TestPubSub::Nats.config(Uuid::new_v4(), 1).await.unwrap();
	let mut docker_config = docker_config.unwrap();
	docker_config.start().await.unwrap();
	tokio::time::sleep(Duration::from_secs(1)).await;

	let rivet_config::config::PubSub::Nats(nats) = pubsub_config else {
		unreachable!();
	};
	let config = NatsConfig {
		addresses: nats.addresses.clone(),
		username: nats.username.clone(),
		password: nats.password.as_ref().map(|p| p.read().clone()),
		client_capacity: nats.client_capacity,
		subscription_capacity: nats.subscription_capacity,
	};
	(config, docker_config)
}

/// Build a Postgres-backed `Database` with a deliberately tiny follower pool.
async fn make_db(connection_string: &str, nats: Option<&NatsConfig>) -> Database {
	let mut config = PostgresConfig::new(connection_string.to_string());
	config.nats = nats.cloned();
	config.pool_max_size = Some(POOL_MAX_SIZE);
	let driver =
		universaldb::driver::PostgresDatabaseDriver::new_with_config(test_config(), config)
			.await
			.unwrap();
	Database::new(Arc::new(driver))
}

/// Raw verification connection used to inspect lease state out of band.
async fn connect_raw(connection_string: &str) -> tokio_postgres::Client {
	let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
		.await
		.unwrap();
	tokio::spawn(async move {
		let _ = connection.await;
	});
	client
}

/// Spawn one write transaction per pool slot, each of which reads (so its task checks out a
/// connection) and then waits on a shared barrier before returning. The barrier guarantees every
/// slot is held before any of the closures return, so all of the commits enter submission with an
/// empty pool.
fn spawn_barriered_writers(db: &Arc<Database>) -> Vec<tokio::task::JoinHandle<()>> {
	let barrier = Arc::new(Barrier::new(POOL_MAX_SIZE));

	(0..POOL_MAX_SIZE)
		.map(|i| {
			let db = db.clone();
			let barrier = barrier.clone();
			tokio::spawn(async move {
				let key = key_for(i);
				db.txn("test_pool_starvation", move |tx| {
					let barrier = barrier.clone();
					let key = key.clone();
					async move {
						tx.get(&key, Serializable).await?;
						barrier.wait().await;
						tx.set(&key, value_for(i).as_slice());
						Ok(())
					}
				})
				.await
				.unwrap();
			})
		})
		.collect()
}

fn key_for(i: usize) -> Vec<u8> {
	format!("pool_starvation/key/{i}").into_bytes()
}

fn value_for(i: usize) -> Vec<u8> {
	format!("value-{i}").into_bytes()
}

async fn read_key(db: &Database, key: Vec<u8>) -> Option<Vec<u8>> {
	db.txn("test_pool_starvation_read", move |tx| {
		let key = key.clone();
		async move { Ok(tx.get(&key, Serializable).await?) }
	})
	.await
	.unwrap()
	.map(|slice| slice.to_vec())
}

async fn read_lease_expires_at(client: &tokio_postgres::Client) -> Option<std::time::SystemTime> {
	let row = client
		.query_opt("SELECT expires_at FROM udb_lease WHERE id = 1", &[])
		.await
		.unwrap()?;
	Some(row.get(0))
}

/// Single-node: every commit parks in the in-process submit with no timeout, so a retained
/// connection makes the wait cycle a permanent deadlock. The bound here is a failure mode, not a
/// tolerance: a regression fails fast instead of hanging CI.
#[tokio::test]
async fn pool_starvation_single_node() {
	init_tracing();

	let (connection_string, _docker) = setup_postgres().await;
	let db = Arc::new(make_db(&connection_string, None).await);

	let handles = spawn_barriered_writers(&db);

	tokio::time::timeout(Duration::from_secs(15), join_all(handles))
		.await
		.expect(
			"commits deadlocked: every pool slot is held by a transaction parked in commit submission",
		)
		.into_iter()
		.for_each(|res| res.unwrap());

	for i in 0..POOL_MAX_SIZE {
		assert_eq!(
			read_key(&db, key_for(i)).await,
			Some(value_for(i)),
			"every barriered write must be durable"
		);
	}
}

/// The stall is not confined to writers. deadpool's semaphore is FIFO, so an ordinary read queues
/// behind the parked commits and never gets a connection either. This is the shape of the config
/// read timeouts seen alongside the commit stalls.
#[tokio::test]
async fn pool_starvation_blocks_reads() {
	init_tracing();

	let (connection_string, _docker) = setup_postgres().await;
	let db = Arc::new(make_db(&connection_string, None).await);

	let handles = spawn_barriered_writers(&db);

	let reader_db = db.clone();
	let reader =
		tokio::spawn(
			async move { read_key(&reader_db, b"pool_starvation/unrelated".to_vec()).await },
		);

	tokio::time::timeout(Duration::from_secs(3), reader)
		.await
		.expect("an unrelated read must not queue behind commits parked in submission")
		.unwrap();

	tokio::time::timeout(Duration::from_secs(15), join_all(handles))
		.await
		.expect("commits deadlocked")
		.into_iter()
		.for_each(|res| res.unwrap());
}

/// Multi-node: the NATS submit path gives up after its resend attempts, so the cycle breaks on its
/// own after roughly 40 seconds and re-forms under sustained load. Bounding the commits at one
/// request timeout pins that symptom directly.
#[tokio::test]
async fn pool_starvation_multi_node() {
	init_tracing();

	let (connection_string, _docker) = setup_postgres().await;
	let (nats_config, _nats_docker) = setup_nats().await;
	let db = Arc::new(make_db(&connection_string, Some(&nats_config)).await);

	// Only commit once this node has won the election and cached its own lease, so the measured
	// window covers submission rather than leader discovery.
	let raw = connect_raw(&connection_string).await;
	let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
	while read_lease_expires_at(&raw).await.is_none() {
		assert!(
			tokio::time::Instant::now() < deadline,
			"no leader was elected"
		);
		tokio::time::sleep(Duration::from_millis(100)).await;
	}

	let handles = spawn_barriered_writers(&db);

	tokio::time::timeout(Duration::from_secs(5), join_all(handles))
		.await
		.expect(
			"commits must resolve within one request timeout, not after the resend attempts are exhausted",
		)
		.into_iter()
		.for_each(|res| res.unwrap());
}

/// The leader path runs on its own reserved pool, so a follower pool held at capacity by ordinary
/// transactions cannot stop the leader from renewing its lease. Without the reserved pool the
/// renewal blocks on `pool.get()` and the lease lapses after its TTL.
#[tokio::test]
async fn pool_starvation_reserved_leader_pool() {
	init_tracing();

	let (connection_string, _docker) = setup_postgres().await;
	let db = make_db(&connection_string, None).await;
	let raw = connect_raw(&connection_string).await;

	let before = read_lease_expires_at(&raw)
		.await
		.expect("single-node startup acquires the lease before returning");

	// Raw transactions are not wrapped by the transaction timeout, so holding them open pins every
	// follower slot for the whole observation window with no gap for a waiter to slip through.
	let mut held = Vec::new();
	for i in 0..POOL_MAX_SIZE {
		let tx = db.create_txn().unwrap();
		tx.get(&key_for(i), Serializable).await.unwrap();
		held.push(tx);
	}

	// Spans two renewal intervals and exceeds the lease TTL.
	tokio::time::sleep(Duration::from_secs(12)).await;

	let after = read_lease_expires_at(&raw).await.unwrap();
	assert!(
		after > before,
		"the leader must keep renewing its lease while the follower pool is exhausted"
	);

	drop(held);
}
