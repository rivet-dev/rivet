use global_error::prelude::*;
use sqlx::Row;
use tracing_subscriber::prelude::*;
use uuid::Uuid;
use std::sync::Once;
use crate::Pools;

static SETUP_TRACING: Once = Once::new();

fn setup_tracing() {
	SETUP_TRACING.call_once(|| {
		tracing_subscriber::registry()
			.with(
				tracing_logfmt::builder()
					.layer()
					.with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
			)
			.init();
	});
}

async fn setup_test_db() -> GlobalResult<(Pools, String)> {
	setup_tracing();

    let mut root = rivet_config::config::Root::default();
    root.server.as_mut().unwrap().foundationdb = Some(Default::default());
    let config = rivet_config::Config::from_root(root);

    let pools = Pools::test(config).await?;
    let db_name = format!("test_{}", Uuid::new_v4());

    Ok((pools, db_name))
}

#[tokio::test]
async fn sqlite_pool_lifecycle() -> GlobalResult<()> {
    let (pools, db_name) = setup_test_db().await?;

    // Create and write to database
	let db = pools.sqlite(&db_name, false).await?;
	{
		let mut conn = db.conn().await?;

		// Create test table
		sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
			.execute(&mut *conn)
			.await?;

		// Insert test data
		sqlx::query("INSERT INTO test (value) VALUES (?)")
			.bind("test_value")
			.execute(&mut *conn)
			.await?;
	}

	// Snapshot the database
	db.snapshot().await?;

	// Evict the database from memory
	pools.sqlite_manager().evict(&db_name).await?;

	// Load database back and verify data
	let db = pools.sqlite(&db_name, true).await?;
	{
		let mut conn = db.conn().await?;

		// Query and verify data
		let row = sqlx::query("SELECT value FROM test WHERE id = 1")
			.fetch_one(&mut *conn)
			.await?;

		let value: String = row.get(0);
		assert_eq!(value, "test_value");
	}

	Ok(())
}

#[tokio::test]
async fn sqlite_snapshot_idempotence() -> GlobalResult<()> {
    let (pools, db_name) = setup_test_db().await?;

    // Create initial database
	let db = pools.sqlite(&db_name, false).await?;
	{
		let mut conn = db.conn().await?;
		sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
			.execute(&mut *conn)
			.await?;
	}

	// First snapshot should return true since we made changes
	let snapshot_result = db.snapshot().await?;
	assert!(
		snapshot_result,
		"First snapshot should return true due to table creation"
	);

	// Second snapshot with no changes should return false
	let snapshot_result = db.snapshot().await?;
	assert!(
		!snapshot_result,
		"Second snapshot should return false since no changes were made"
	);

	// Make a change to the database
	{
		let mut conn = db.conn().await?;
		sqlx::query("INSERT INTO test (value) VALUES (?)")
			.bind("test_value")
			.execute(&mut *conn)
			.await?;
	}

	// Third snapshot should return true due to the INSERT
	let snapshot_result = db.snapshot().await?;
	assert!(
		snapshot_result,
		"Third snapshot should return true due to INSERT"
	);

	// Fourth snapshot with no new changes should return false
	let snapshot_result = db.snapshot().await?;
	assert!(
		!snapshot_result,
		"Fourth snapshot should return false since no new changes were made"
	);

	Ok(())
}
