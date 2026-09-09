//! Two-node saturation harness from the pool starvation investigation.
//!
//! Two real drivers share one Postgres and one NATS, each with its normal 64-connection pool. Every
//! one of the 128 transactions takes a read snapshot (and so a pooled connection) before any of them
//! submits a commit, which is the condition under which a retained connection makes the commit path
//! unable to make progress.
//!
//! Requires the disposable containers named by `REPRO_PG_PORT` and `REPRO_NATS_PORT`. It creates the
//! driver schema and writes test data, so it must never be pointed at a database holding anything
//! worth keeping. `REPRO_DROP_FIX=1` selects the fixed-code assertions; it changes no source.

use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use futures_util::future::join_all;
use tokio::sync::{mpsc, watch};
use tokio_postgres::NoTls;
use universaldb::{
	Database,
	driver::postgres::{NatsConfig, PostgresConfig},
	utils::IsolationLevel::*,
};

/// Transactions per driver. Matches the driver's default pool size, so each driver's pool is exactly
/// saturated by its own share of the workload.
const PER_NODE: usize = 64;
const TOTAL: usize = PER_NODE * 2;

/// How long the original-code mode observes the stalled state. Beyond both the 5 s transaction body
/// timeout and the 10 s leadership lease.
const STALL_OBSERVATION: Duration = Duration::from_secs(12);

const WARMUP_KEY: &[u8] = b"warmup";
const WARMUP_VALUE: &[u8] = b"warm";

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

fn env_port(name: &str) -> u16 {
	std::env::var(name)
		.unwrap_or_else(|_| panic!("{name} must be set; start the disposable containers first"))
		.parse()
		.unwrap_or_else(|_| panic!("{name} must be a port number"))
}

fn connection_string() -> String {
	format!(
		"postgres://postgres@127.0.0.1:{}/postgres?sslmode=disable",
		env_port("REPRO_PG_PORT")
	)
}

fn nats_config() -> NatsConfig {
	NatsConfig {
		addresses: vec![format!("127.0.0.1:{}", env_port("REPRO_NATS_PORT"))],
		username: None,
		password: None,
		client_capacity: 128,
		subscription_capacity: 1024,
	}
}

async fn make_db(nats: &NatsConfig) -> Database {
	let mut config = PostgresConfig::new(connection_string());
	config.nats = Some(nats.clone());
	let driver = universaldb::driver::PostgresDatabaseDriver::new_with_config(test_config(), config)
		.await
		.unwrap();
	let db = Database::new(Arc::new(driver));
	// One outer attempt only, so the commit path's own resend cycle is what the run observes rather
	// than an outer retry masking it.
	db.txn_retry_limit(1).unwrap();
	db
}

/// Observer connection, opened outside both driver pools so the database stays inspectable while
/// they are saturated.
async fn connect_observer() -> tokio_postgres::Client {
	let (client, connection) = tokio_postgres::connect(&connection_string(), NoTls)
		.await
		.unwrap();
	tokio::spawn(async move {
		let _ = connection.await;
	});
	client
}

/// Sessions sitting inside an open transaction: the read snapshots the workload is holding.
async fn open_snapshot_count(observer: &tokio_postgres::Client) -> i64 {
	observer
		.query_one(
			"SELECT count(*) FROM pg_stat_activity
			 WHERE datname = current_database()
			   AND state = 'idle in transaction'",
			&[],
		)
		.await
		.unwrap()
		.get(0)
}

async fn lease_expired(observer: &tokio_postgres::Client) -> bool {
	observer
		.query_one("SELECT expires_at < now() FROM udb_lease WHERE id = 1", &[])
		.await
		.map(|row| row.get::<_, bool>(0))
		.unwrap_or(false)
}

fn workload_key(node: usize, index: usize) -> Vec<u8> {
	format!("pool_repro/node{node}/key{index}").into_bytes()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn two_node_saturation() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter("warn")
		.with_test_writer()
		.try_init();

	let fixed_mode = std::env::var("REPRO_DROP_FIX").is_ok();
	let nats = nats_config();
	let observer = connect_observer().await;

	let db_a = Arc::new(make_db(&nats).await);
	let db_b = Arc::new(make_db(&nats).await);

	// Ordinary writes must work before saturation, otherwise the run proves nothing.
	for db in [&db_a, &db_b] {
		db.txn("repro_warmup", |tx| async move {
			tx.set(WARMUP_KEY, WARMUP_VALUE);
			Ok(())
		})
		.await
		.unwrap();
	}

	// A latched release rather than a cyclic barrier: a transaction that retried would block forever
	// on a second barrier generation and hide the result behind a hang.
	let (ready_tx, mut ready_rx) = mpsc::channel::<()>(TOTAL);
	let (release_tx, release_rx) = watch::channel(false);

	let mut handles = Vec::with_capacity(TOTAL);
	for (node, db) in [db_a.clone(), db_b.clone()].into_iter().enumerate() {
		for index in 0..PER_NODE {
			let ready_tx = ready_tx.clone();
			let release_rx = release_rx.clone();
			let db = db.clone();
			handles.push(tokio::spawn(async move {
				db.txn("two_node_saturation", move |tx| {
					let ready_tx = ready_tx.clone();
					let mut release_rx = release_rx.clone();
					async move {
						// Forces the transaction task to open its snapshot and check out a slot.
						let warm = tx.get(WARMUP_KEY, Snapshot).await?;
						assert_eq!(warm.map(|v| v.to_vec()).as_deref(), Some(WARMUP_VALUE));
						let _ = ready_tx.send(()).await;
						while !*release_rx.borrow_and_update() {
							release_rx.changed().await.ok();
						}
						tx.set(&workload_key(node, index), b"persisted");
						Ok(())
					}
				})
				.await
			}));
		}
	}
	drop(ready_tx);

	for _ in 0..TOTAL {
		ready_rx.recv().await.expect("all transactions must open a snapshot");
	}
	let snapshots = open_snapshot_count(&observer).await;
	assert!(
		snapshots >= TOTAL as i64,
		"expected at least {TOTAL} open read snapshots before release, saw {snapshots}"
	);

	let started = Instant::now();
	release_tx.send(true).unwrap();

	if !fixed_mode {
		// Original-code mode: observe the stalled state rather than waiting for the callers.
		tokio::time::sleep(STALL_OBSERVATION).await;
		let pending = handles.iter().filter(|h| !h.is_finished()).count();
		let written: i64 = observer
			.query_one(
				"SELECT count(*) FROM kv WHERE key LIKE 'pool_repro/%'",
				&[],
			)
			.await
			.unwrap()
			.get(0);
		println!(
			"original-code mode after {}s: pending={pending}/{TOTAL} workload_rows={written} lease_expired={}",
			STALL_OBSERVATION.as_secs(),
			lease_expired(&observer).await
		);
		return;
	}

	// Fixed-code mode: every caller succeeds and every value is durable.
	let results = tokio::time::timeout(Duration::from_secs(60), join_all(handles))
		.await
		.expect("all 128 commits must complete");
	let elapsed = started.elapsed();

	let mut failures = Vec::new();
	for (i, res) in results.into_iter().enumerate() {
		match res.unwrap() {
			Ok(()) => {}
			Err(err) => failures.push(format!("txn {i}: {err}")),
		}
	}
	assert!(
		failures.is_empty(),
		"{} of {TOTAL} commits failed:\n{}",
		failures.len(),
		failures.join("\n")
	);

	let written: i64 = observer
		.query_one(
			"SELECT count(*) FROM kv WHERE key LIKE 'pool_repro/%'",
			&[],
		)
		.await
		.unwrap()
		.get(0);
	assert_eq!(written, TOTAL as i64, "every workload key must be durable");
	assert!(
		!lease_expired(&observer).await,
		"the leader must still hold its lease after the workload"
	);

	println!("fixed-code mode: {TOTAL}/{TOTAL} commits in {}ms", elapsed.as_millis());
}
