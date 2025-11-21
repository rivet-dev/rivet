use std::{path::Path, sync::Arc};

use anyhow::Context;
use futures_util::StreamExt;
use rivet_test_deps_docker::TestDatabase;
use rocksdb::{OptimisticTransactionDB, Options, WriteOptions};
use universaldb::{
	Database,
	utils::{IsolationLevel::*, calculate_tx_retry_backoff},
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

	let mut handles = Vec::new();

	for _ in 0..64 {
		let db = db.clone();

		let handle = tokio::spawn(async move {
			db.run(|tx| async move {
				let key = vec![1, 2, 3];
				let value = vec![4, 5, 6];

				tx.get(&key, Serializable).await.unwrap();
				tx.set(&key, &value);

				Ok(())
			})
			.await
			.unwrap();

			tracing::info!("success");
		});

		handles.push(handle);
	}

	futures_util::stream::iter(handles)
		.buffer_unordered(1024)
		.for_each(|result| async {
			if let Err(err) = result {
				tracing::error!(?err, "task failed");
			}
		})
		.await;
}
