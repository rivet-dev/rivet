use statrs::statistics::{Data, Median, OrderStatistics, Statistics};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::interval;

use chirp_workflow::prelude::*;

mod common;
use common::*;

type GlobalError = WorkflowError;

pub(crate) struct SqlStub {}

impl SqlStub {
	// For sql macro
	pub fn name(&self) -> &str {
		""
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn sqlite() {
	setup_tracing();

	let ctx =
		chirp_workflow::prelude::TestCtx::from_env::<db::DatabaseFdbSqliteNats>("sqlite", true)
			.await;
	let pools = ctx.pools().clone();

	let timings = Arc::new(Mutex::new(Vec::new()));

	// Spawn worker tasks
	for i in 0..8 {
		let timings = timings.clone();
		let pool = pools
			.sqlite(format!("chirp-workflow-sqlite-test-{i}"), false)
			.await
			.unwrap();

		tokio::spawn(async move {
			if let Err(err) = init(timings, i, pool).await {
				tracing::error!(?err);
			}
		});
	}

	// Print statistics every 5 seconds
	let timings2 = timings.clone();
	let mut interval = interval(Duration::from_secs(5));
	let handle = tokio::spawn(async move {
		loop {
			interval.tick().await;
			print_statistics(&timings2).await;
		}
	});

	tokio::select! {
		res = handle => res.unwrap(),
		res = tokio::signal::ctrl_c() => res.unwrap(),
	}
}

async fn init(
	timings: Arc<Mutex<Vec<(String, Duration)>>>,
	i: usize,
	pool: SqlitePool,
) -> GlobalResult<()> {
	sql_execute!(
		[SqlStub {}, &pool]
		"
		CREATE TABLE IF NOT EXISTS foobar (
			test INT NOT NULL
		) STRICT
		",
	)
	.await?;

	for j in 0..1000 {
		sql_execute!(
			[SqlStub {}, &pool]
			"
			INSERT INTO foobar (test)
			VALUES (?)
			",
			4
		)
		.await?;

		if j % 100 == 0 {
			tracing::info!(%i, %j);
		}
	}

	loop {
		inner(&pool, timings.clone()).await?;
	}
}

async fn inner(
	pool: &SqlitePool,
	timings: Arc<Mutex<Vec<(String, Duration)>>>,
) -> GlobalResult<()> {
	let start = Instant::now();
	sql_execute!(
		[SqlStub {}, pool]
		"
		INSERT INTO foobar (test)
		VALUES (?)
		",
		5,
	)
	.await?;

	let dt = start.elapsed();
	let timings2 = timings.clone();
	tokio::spawn(async move {
		timings2.lock().await.push(("insert".to_string(), dt));
	});

	let start = Instant::now();
	sql_fetch_all!(
		[SqlStub {}, (i64,), pool]
		"
		SELECT * FROM foobar
		",
	)
	.await?;

	let dt = start.elapsed();
	let timings2 = timings.clone();
	tokio::spawn(async move {
		timings2.lock().await.push(("select".to_string(), dt));
	});

	let start = Instant::now();
	sql_execute!(
		[SqlStub {}, pool]
		"
		DELETE FROM foobar
		WHERE rowid IN (
			SELECT rowid FROM foobar
			ORDER BY rowid
			LIMIT 1
		)
		",
	)
	.await?;

	let dt = start.elapsed();
	let timings2 = timings.clone();
	tokio::spawn(async move {
		timings2.lock().await.push(("delete".to_string(), dt));
	});

	Ok(())
}

async fn print_statistics(timings: &Arc<Mutex<Vec<(String, Duration)>>>) {
	let mut timings = timings.lock().await;
	let mut insert_times = Vec::new();
	let mut select_times = Vec::new();
	let mut delete_times = Vec::new();

	for (name, duration) in timings.iter() {
		match name.as_str() {
			"insert" => insert_times.push(duration.as_secs_f64()),
			"select" => select_times.push(duration.as_secs_f64()),
			"delete" => delete_times.push(duration.as_secs_f64()),
			_ => (),
		}
	}

	timings.clear();
	drop(timings);

	tracing::info!("Insert query statistics:");
	print_stats(&insert_times);

	tracing::info!("Select query statistics:");
	print_stats(&select_times);

	tracing::info!("Delete query statistics:");
	print_stats(&delete_times);

	tracing::info!("");
}

fn print_stats(times: &[f64]) {
	let mut data = Data::new(times.to_vec());

	if times.is_empty() {
		tracing::info!("No times available.");
		return;
	}

	let mean = times.mean();
	let std_dev = times.std_dev();
	let median = data.median();
	let p90 = data.quantile(0.9);
	let p99 = data.quantile(0.99);

	tracing::info!("Mean: {:.1} ms", mean * 1000.0);
	// tracing::info!("Standard Deviation: {:.1} ms", std_dev * 1000.0);
	// tracing::info!("Median: {:.1} ms", median * 1000.0);
	tracing::info!("    90th Percentile: {:.1} ms", p90 * 1000.0);
	tracing::info!("99th Percentile: {:.1} ms", p99 * 1000.0);
}
