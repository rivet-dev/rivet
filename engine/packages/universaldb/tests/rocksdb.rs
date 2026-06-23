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

	db.txn("test_universaldbrocksdb", |tx| async move {
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
				.txn("test_universaldbrocksdb", |tx| {
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
							if chunk.len() < 100 {
								if let Some(entry) = stream.try_next().await? {
									chunk.push(entry);
									continue;
								}
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

/// A reverse range read with a limit must return the highest keys in the range,
/// matching FoundationDB semantics. Regression test for the RocksDB driver
/// applying the limit during a forward scan and only reversing afterward, which
/// returned the lowest keys instead.
#[tokio::test]
async fn rocksdb_reverse_range_with_limit() {
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

	// Seed keys [1, 2, 3, 0] through [1, 2, 3, 4].
	db.txn("seed", |tx| async move {
		for i in 0..=4u8 {
			tx.set(&[1, 2, 3, i], &[i]);
		}
		Ok(())
	})
	.await
	.unwrap();

	let collect = |reverse: bool, limit: Option<usize>| {
		let db = db.clone();
		async move {
			db.txn("scan", move |tx| async move {
				let mut stream = tx.get_ranges_keyvalues(
					RangeOption {
						mode: StreamingMode::WantAll,
						reverse,
						limit,
						..(&[1u8, 2, 3, 0][..], &[1u8, 2, 3, 5][..]).into()
					},
					Serializable,
				);
				let mut keys = Vec::new();
				while let Some(entry) = stream.try_next().await? {
					keys.push(entry.key().to_vec());
				}
				Ok(keys)
			})
			.await
			.unwrap()
		}
	};

	// Reverse with a limit returns the highest keys, highest first.
	assert_eq!(
		collect(true, Some(1)).await,
		vec![vec![1, 2, 3, 4]],
		"reverse limit 1 must return the single highest key"
	);
	assert_eq!(
		collect(true, Some(2)).await,
		vec![vec![1, 2, 3, 4], vec![1, 2, 3, 3]],
		"reverse limit 2 must return the two highest keys in descending order"
	);

	// Forward with a limit returns the lowest keys, lowest first.
	assert_eq!(
		collect(false, Some(2)).await,
		vec![vec![1, 2, 3, 0], vec![1, 2, 3, 1]],
		"forward limit 2 must return the two lowest keys in ascending order"
	);

	// Reverse without a limit returns the whole range, highest first.
	assert_eq!(
		collect(true, None).await,
		vec![
			vec![1, 2, 3, 4],
			vec![1, 2, 3, 3],
			vec![1, 2, 3, 2],
			vec![1, 2, 3, 1],
			vec![1, 2, 3, 0],
		],
		"reverse without a limit must return every key in descending order"
	);
}

/// Regression test for a RocksDB driver crash on zero-length (empty) values.
///
/// RocksDB hands back a NULL data pointer for zero-length values on some
/// platforms (observed on macOS arm64; Linux returns a non-null pointer, so this
/// test does not abort here but does on macOS). The driver previously scanned
/// ranges with rocksdb's boxing `DBIterator`, whose `Iterator::next` copies every
/// value into a `Box<[u8]>` via `copy_nonoverlapping` even when the caller only
/// needs keys (clear_range, key selectors). Copying from the null pointer
/// violates `copy_nonoverlapping`'s non-null precondition and aborts the engine
/// the instant an actor commits state. This exercises every rewritten iteration
/// path (get_range forward/reverse, get_key selectors, and the clear_range commit
/// path) over empty-valued keys.
#[tokio::test]
async fn rocksdb_empty_values() {
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

	// Seed keys [1, 2, 3, 0] through [1, 2, 3, 16] with EMPTY values.
	db.txn("seed", |tx| async move {
		for i in 0..=16u8 {
			tx.set(&[1, 2, 3, i], b"");
		}
		Ok(())
	})
	.await
	.unwrap();

	// Forward range read over empty-valued keys returns every key with an empty
	// value. Pre-fix, the boxing iterator aborted boxing the null empty value.
	let forward = db
		.txn("forward", |tx| async move {
			let mut stream = tx.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					..(&[1u8, 2, 3, 0][..], &[1u8, 2, 3, 17][..]).into()
				},
				Serializable,
			);
			let mut out = Vec::new();
			while let Some(entry) = stream.try_next().await? {
				out.push((entry.key().to_vec(), entry.value().to_vec()));
			}
			Ok(out)
		})
		.await
		.unwrap();
	assert_eq!(forward.len(), 17, "forward range must return all empty-valued keys");
	assert!(
		forward.iter().all(|(_, value)| value.is_empty()),
		"every value must round-trip as empty"
	);
	assert_eq!(forward.first().unwrap().0, vec![1, 2, 3, 0]);
	assert_eq!(forward.last().unwrap().0, vec![1, 2, 3, 16]);

	// Reverse range read over the same empty-valued keys, highest first.
	let reverse = db
		.txn("reverse", |tx| async move {
			let mut stream = tx.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					reverse: true,
					..(&[1u8, 2, 3, 0][..], &[1u8, 2, 3, 17][..]).into()
				},
				Serializable,
			);
			let mut out = Vec::new();
			while let Some(entry) = stream.try_next().await? {
				out.push(entry.key().to_vec());
			}
			Ok(out)
		})
		.await
		.unwrap();
	assert_eq!(reverse.first().unwrap(), &vec![1, 2, 3, 16]);
	assert_eq!(reverse.last().unwrap(), &vec![1, 2, 3, 0]);

	// Key selectors resolve over empty-valued keys (exercises handle_get_key).
	let selected = db
		.txn("selectors", |tx| async move {
			let geq = tx
				.get_key(&KeySelector::first_greater_or_equal(vec![1, 2, 3, 5]), Serializable)
				.await?;
			let gt = tx
				.get_key(&KeySelector::first_greater_than(vec![1, 2, 3, 5]), Serializable)
				.await?;
			let lt = tx
				.get_key(&KeySelector::last_less_than(vec![1, 2, 3, 5]), Serializable)
				.await?;
			let leq = tx
				.get_key(&KeySelector::last_less_or_equal(vec![1, 2, 3, 5]), Serializable)
				.await?;
			Ok((geq.to_vec(), gt.to_vec(), lt.to_vec(), leq.to_vec()))
		})
		.await
		.unwrap();
	assert_eq!(selected.0, vec![1, 2, 3, 5], "first_greater_or_equal");
	assert_eq!(selected.1, vec![1, 2, 3, 6], "first_greater_than");
	assert_eq!(selected.2, vec![1, 2, 3, 4], "last_less_than");
	assert_eq!(selected.3, vec![1, 2, 3, 5], "last_less_or_equal");

	// clear_range over the empty-valued keys. This is the exact path that aborts
	// on macOS the instant an actor persists state, because clear_range iterates
	// the range and (pre-fix) boxed each null empty value.
	db.txn("clear", |tx| async move {
		tx.clear_range(&[1, 2, 3, 0], &[1, 2, 3, 17]);
		Ok(())
	})
	.await
	.unwrap();

	// The range is now empty.
	let remaining = db
		.txn("verify", |tx| async move {
			let mut stream = tx.get_ranges_keyvalues(
				RangeOption {
					mode: StreamingMode::WantAll,
					..(&[1u8, 2, 3, 0][..], &[1u8, 2, 3, 17][..]).into()
				},
				Serializable,
			);
			let mut count = 0usize;
			while stream.try_next().await?.is_some() {
				count += 1;
			}
			Ok(count)
		})
		.await
		.unwrap();
	assert_eq!(remaining, 0, "clear_range must delete every key in the range");
}
