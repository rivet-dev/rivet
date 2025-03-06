//! Run with:
//!
//! ```sh
//! cargo bench -q -p rivet-pools --bench sqlite_lifecycle
//! ````
//!
//! Or for faster iteration:
//!
//! ```sh
//! cargo bench -q -p rivet-pools --bench sqlite_lifecycle --profile dev
//! ````

use divan::{Bencher};
use rivet_pools::Pools;
use uuid::Uuid;
use sqlx::Connection;

const ROW_COUNTS: &[usize] = &[1, 1_000, 100_000];
const SAMPLE_SIZE: u32 = 1;
const SAMPLE_COUNT: u32 = 10;

fn main() {
	divan::main();
}

// Helper to create a database of given size
async fn setup_database(pools: &Pools, name: &str, row_count: usize) -> usize {
	let db = pools.sqlite_with_auto_snapshot(name, false, false).await.unwrap();

	// Insert data
	{
		let mut conn = db.conn().await.unwrap();

		// Create test table
		sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
			.execute(&mut *conn)
			.await
			.unwrap();

		// Insert test data in batches
		let mut tx = conn.begin().await.unwrap();
		for i in 0..row_count {
			sqlx::query("INSERT INTO test (value) VALUES (?)")
				.bind(format!("test_value_{}", i))
				.execute(&mut *tx)
				.await
				.unwrap();
		}
		tx.commit().await.unwrap();
	}

    // Get size
    let size = db.debug_db_size().await.unwrap();

	// Evict (will cause deadlock with conn if not dropped)
	pools.sqlite_manager().evict(name, false).await.unwrap();

    size as usize
}

struct TestSetup {
	rt: tokio::runtime::Runtime,
	pools: Pools,
	db_name: String,
    db_size: usize,
}

// Helper to setup test environment
fn setup_test(row_count: usize) -> TestSetup {
	let rt = tokio::runtime::Runtime::new().unwrap();

	// Setup pools and database
	let (pools, db_name, db_size) = rt.block_on(async {
		let mut root = rivet_config::config::Root::default();
		root.server.as_mut().unwrap().foundationdb = Some(Default::default());
		let config = rivet_config::Config::from_root(root);
		let pools = Pools::test(config).await.unwrap();

		// Flush FDB before test
		pools
			.fdb()
			.unwrap()
			.run(|tx, _| async move {
				tx.clear_range(b"\x00", b"\xFF");
				Ok(())
			})
			.await
			.unwrap();

		// Setup database
		let db_name = format!("bench_db_{}_{}", row_count, Uuid::new_v4());
		let db_size = setup_database(&pools, &db_name, row_count).await;

		(pools, db_name, db_size)
	});

	TestSetup {
		rt,
		pools,
		db_name,
        db_size,
	}
}

#[divan::bench(args = ROW_COUNTS, sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn read_database(bencher: Bencher, row_count: usize) {
	bencher
		.with_inputs(|| setup_test(row_count))
		.bench_values(|setup| {
			setup.rt.block_on(async {
				setup.pools.sqlite(&setup.db_name, false).await.unwrap();
			})
		});
}

#[divan::bench(args = ROW_COUNTS, sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn snapshot_database(bencher: Bencher, row_count: usize) {
	bencher
		.with_inputs(|| {
			let setup = setup_test(row_count);
			let db = setup.rt.block_on(async { setup.pools.sqlite(&setup.db_name, false).await.unwrap() });
            (setup, db)
		})
		.bench_values(|(setup, db)| {
			setup.rt.block_on(async {
				db.snapshot().await.unwrap();
			})
		});
}

#[divan::bench(args = ROW_COUNTS, sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn evict_database(bencher: Bencher, row_count: usize) {
	bencher
		.with_inputs(|| {
			let setup = setup_test(row_count);

			// Load database in order to be evicted in bench
			setup.rt.block_on(async { setup.pools.sqlite(&setup.db_name, false).await.unwrap() });

			setup
		})
		.bench_values(|setup| {
			setup.rt.block_on(async {
				setup.pools.sqlite_manager().evict(&setup.db_name, false).await.unwrap();
			})
		});
}

#[divan::bench(args = ROW_COUNTS, sample_size = SAMPLE_SIZE, sample_count = SAMPLE_COUNT)]
fn full_lifecycle(bencher: Bencher, row_count: usize) {
	bencher
		.with_inputs(|| setup_test(row_count))
		.bench_values(|setup| {
			setup.rt.block_on(async {
				// Load
				let db = setup.pools.sqlite(&setup.db_name, false).await.unwrap();

				// Read from database
				{
					let mut conn = db.conn().await.unwrap();
					let _rows = sqlx::query("SELECT COUNT(*) FROM test")
						.fetch_one(&mut *conn)
						.await
						.unwrap();
				}

				// Evict database (conn must be dropped)
				setup.pools.sqlite_manager().evict(&setup.db_name, false).await.unwrap();
			})
		});
}
