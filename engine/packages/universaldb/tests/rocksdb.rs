use std::{
	path::Path,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

use anyhow::Context;
use futures_util::{StreamExt, TryStreamExt};
use rand::{distributions::WeightedError, seq::SliceRandom};
use rivet_test_deps_docker::TestDatabase;
use rocksdb::{OptimisticTransactionDB, Options, WriteOptions};
use universaldb::{
	Database,
	prelude::*,
	utils::{calculate_tx_retry_backoff, end_of_key_range},
};
use uuid::Uuid;

#[tokio::test]
async fn rocksdb_native() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter("debug")
		.with_test_writer()
		.try_init();

	let db_path = Path::new("/tmp/foobar-db");
	std::fs::create_dir_all(&db_path)
		.context("failed to create database directory")
		.unwrap();

	// Configure RocksDB options
	let mut opts = Options::default();
	opts.create_if_missing(true);
	opts.set_max_open_files(10000);
	opts.set_keep_log_file_num(10);
	opts.set_max_total_wal_size(64 * 1024 * 1024); // 64MB

	// Open the OptimisticTransactionDB
	tracing::debug!(path=%db_path.display(), "opening rocksdb");
	let db: Arc<OptimisticTransactionDB> = Arc::new(
		OptimisticTransactionDB::open(&opts, db_path)
			.context("failed to open rocksdb")
			.unwrap(),
	);

	let mut handles = Vec::new();

	for _ in 0..64 {
		let db = db.clone();
		let write_opts = WriteOptions::default();
		let txn_opts = rocksdb::OptimisticTransactionOptions::default();

		let handle = tokio::spawn(async move {
			for attempt in 0..300 {
				let txn = db.transaction_opt(&write_opts, &txn_opts);

				let key = vec![1, 2, 3];
				let value = vec![4, 5, 6];

				txn.get(&key).unwrap();
				txn.put(&key, &value).unwrap();

				tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

				// Execute transaction
				let Err(error) = txn.commit() else {
					tracing::info!("success");
					return;
				};

				let err_str = error.to_string();
				if err_str.contains("conflict") || err_str.contains("Resource busy") {
					tracing::warn!(?error, "conflict");
					let backoff_ms = calculate_tx_retry_backoff(attempt);
					tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
					continue;
				}

				Err::<(), _>(error).unwrap();
			}
		});

		handles.push(handle);
	}

	futures_util::stream::iter(handles)
		.buffer_unordered(64)
		.for_each(|result| async {
			if let Err(err) = result {
				tracing::error!(?err, "task failed");
			}
		})
		.await;
}

#[tokio::test]
async fn rocksdb_udb() {
	let _ = tracing_subscriber::fmt()
		.with_env_filter("debug")
		.with_test_writer()
		.try_init();

	let test_id = Uuid::new_v4();
	let (db_config, _docker_config) = TestDatabase::FileSystem.config(test_id, 1).await.unwrap();

	let rivet_config::config::Database::FileSystem(fs_config) = db_config else {
		unreachable!()
	};

	let driver = universaldb::driver::RocksDbDatabaseDriver::new(fs_config.path)
		.await
		.unwrap();
	let db = Database::new(Arc::new(driver));

	db.run(|tx| async move {
		for i in 0..=255 {
			for j in 0..=0 {
				let key = vec![1, 2, 3, i, j];
				let value = b"";

				tx.set(&key, value);
			}
		}

		Ok(())
	})
	.await
	.unwrap();

	let mut handles = Vec::new();

	for i in 0..64 {
		let db = db.clone();

		let handle = tokio::spawn(async move {
			let tries = AtomicUsize::new(0);
			let start = Instant::now();

			let alloc = db
				.run(|tx| {
					let tries = &tries;

					async move {
						tries.fetch_add(1, Ordering::SeqCst);

						let mut stream = tx.get_ranges_keyvalues(
							universaldb::RangeOption {
								begin: KeySelector::first_greater_or_equal(vec![1, 2, 3]),
								end: KeySelector::last_less_or_equal(vec![1, 2, 3, 255, 255]),
								mode: StreamingMode::Iterator,
								..RangeOption::default()
							},
							Snapshot,
						);

						let mut chunk = Vec::with_capacity(100);

						loop {
							let empty = if chunk.len() >= 100 {
								false
							} else if let Some(entry) = stream.try_next().await? {
								chunk.push(entry);
								continue;
							} else {
								true
							};

							let entry = match chunk.choose_weighted(&mut rand::thread_rng(), |_| 1)
							{
								Ok(entry) => entry,
								Err(WeightedError::NoItem) => break,
								Err(err) => return Err(err.into()),
							};

							tx.add_conflict_range(
								entry.key(),
								&end_of_key_range(entry.key()),
								ConflictRangeType::Read,
							)?;

							tx.clear(entry.key());

							return Ok(Some(entry.key().to_vec()));

							if empty {
								break;
							} else {
								chunk.clear();
							}
						}

						Ok(None)
					}
				})
				.await?;

			let tries = tries.load(Ordering::SeqCst);
			let duration = start.elapsed();

			tracing::info!(%i, %tries, ?alloc, ?duration, "success");

			anyhow::Ok(alloc.map(|_| (tries, duration)))
		});

		handles.push(handle);
	}

	let res = futures_util::stream::iter(handles)
		.map(|result| async {
			match result.await {
				Ok(Ok(result)) => result,
				Ok(Err(err)) => {
					tracing::error!(?err, "task failed");
					None
				}
				Err(err) => {
					tracing::error!(?err, "task failed");
					None
				}
			}
		})
		.buffer_unordered(1024)
		.filter_map(|x| std::future::ready(x))
		.collect::<Vec<_>>()
		.await;

	let mut retry_distribution = Vec::<(usize, usize)>::new();
	let mut duration_buckets = vec![
		(Duration::from_micros(0), Duration::from_micros(1000), 0),
		(Duration::from_micros(1000), Duration::from_micros(2500), 0),
		(Duration::from_micros(2500), Duration::from_micros(5000), 0),
		(Duration::from_micros(5000), Duration::from_micros(10000), 0),
		(Duration::from_millis(10), Duration::from_millis(25), 0),
		(Duration::from_millis(25), Duration::from_millis(50), 0),
		(Duration::from_millis(50), Duration::from_millis(100), 0),
		(Duration::from_millis(100), Duration::from_millis(250), 0),
		(Duration::from_millis(250), Duration::from_millis(500), 0),
		(Duration::from_millis(500), Duration::from_millis(1000), 0),
		(Duration::from_millis(1000), Duration::from_millis(2500), 0),
		(Duration::from_millis(2500), Duration::MAX, 0),
	];

	for (retries, duration) in res {
		if let Some((_, count)) = retry_distribution.iter_mut().find(|(n, _)| n == &retries) {
			*count += 1;
		} else {
			retry_distribution.push((retries, 1));
		}

		for (_, end, count) in &mut duration_buckets {
			if &duration < end {
				*count += 1;
				break;
			}
		}
	}

	for (retries, count) in retry_distribution {
		println!("{retries}: {count}");
	}

	println!();

	for (start, end, count) in duration_buckets {
		if end == Duration::MAX {
			println!("{start:?}-Inf: {count}");
		} else {
			println!("{start:?}-{end:?}: {count}");
		}
	}
}
