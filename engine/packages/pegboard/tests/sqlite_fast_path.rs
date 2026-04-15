use std::sync::Arc;

use anyhow::Result;
use gas::prelude::*;
use pegboard::actor_kv as kv;
use rivet_envoy_protocol as ep;
use tempfile::TempDir;
use universaldb::Database;

async fn test_db() -> Result<(Database, TempDir, kv::Recipient)> {
	let _ = tracing_subscriber::fmt::try_init();

	let temp_dir = tempfile::tempdir()?;
	let driver =
		universaldb::driver::RocksDbDatabaseDriver::new(temp_dir.path().to_path_buf()).await?;
	let db = Database::new(Arc::new(driver));
	let recipient = kv::Recipient {
		actor_id: Id::new_v1(1),
		namespace_id: Id::new_v1(1),
		name: "default".to_string(),
	};

	Ok((db, temp_dir, recipient))
}

fn sqlite_meta_key(file_tag: u8) -> Vec<u8> {
	vec![0x08, 0x01, 0x00, file_tag]
}

fn sqlite_page_key(file_tag: u8, chunk_index: u32) -> Vec<u8> {
	let mut key = vec![0x08, 0x01, 0x01, file_tag];
	key.extend_from_slice(&chunk_index.to_be_bytes());
	key
}

#[tokio::test]
async fn sqlite_write_batch_round_trips_through_generic_get() -> Result<()> {
	let (db, _temp_dir, recipient) = test_db().await?;
	let meta_value = 8192_u64.to_be_bytes().to_vec();
	let page_a = vec![0xAB; 4096];
	let page_b = vec![0xCD; 4096];

	kv::sqlite_write_batch(
		&db,
		&recipient,
		ep::KvSqliteWriteBatchRequest {
			file_tag: 0,
			meta_value: meta_value.clone(),
			page_updates: vec![
				ep::SqlitePageUpdate {
					chunk_index: 0,
					data: page_a.clone(),
				},
				ep::SqlitePageUpdate {
					chunk_index: 2,
					data: page_b.clone(),
				},
			],
			fence: ep::SqliteFastPathFence {
				expected_fence: None,
				request_fence: 1,
			},
		},
	)
	.await?;

	let keys = vec![
		sqlite_meta_key(0),
		sqlite_page_key(0, 0),
		sqlite_page_key(0, 2),
	];
	let (found_keys, found_values, found_metadata) = kv::get(&db, &recipient, keys.clone()).await?;

	assert_eq!(found_keys.len(), 3);
	assert_eq!(found_values.len(), 3);
	assert_eq!(found_metadata.len(), 3);

	for key in &keys {
		assert!(found_keys.iter().any(|candidate| candidate == key));
	}

	let meta_idx = found_keys
		.iter()
		.position(|candidate| candidate == &sqlite_meta_key(0))
		.expect("metadata key should exist");
	assert_eq!(found_values[meta_idx], meta_value);

	let page_a_idx = found_keys
		.iter()
		.position(|candidate| candidate == &sqlite_page_key(0, 0))
		.expect("page 0 should exist");
	assert_eq!(found_values[page_a_idx], page_a);

	let page_b_idx = found_keys
		.iter()
		.position(|candidate| candidate == &sqlite_page_key(0, 2))
		.expect("page 2 should exist");
	assert_eq!(found_values[page_b_idx], page_b);

	for metadata in found_metadata {
		assert_eq!(
			metadata.version,
			env!("CARGO_PKG_VERSION").as_bytes().to_vec()
		);
		assert!(metadata.update_ts > 0);
	}

	Ok(())
}
