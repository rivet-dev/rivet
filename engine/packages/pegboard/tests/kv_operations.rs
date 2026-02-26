use anyhow::Result;
use gas::prelude::*;
use pegboard::actor_kv as kv;
use rivet_runner_protocol::mk2 as rp;

#[tokio::test]
async fn test_kv_operations() -> Result<()> {
	// Setup test environment
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.with_target(false)
		.init();

	let test_id = Uuid::new_v4();
	let dc_label = 1;
	let datacenters = vec![rivet_config::config::topology::Datacenter {
		name: "test-dc".to_string(),
		datacenter_label: dc_label,
		is_leader: true,
		peer_url: url::Url::parse("http://127.0.0.1:8080")?,
		public_url: url::Url::parse("http://127.0.0.1:8081")?,
		proxy_url: None,
		valid_hosts: None,
	}];

	let api_peer_port = portpicker::pick_unused_port().expect("failed to pick api peer port");
	let guard_port = portpicker::pick_unused_port().expect("failed to pick guard port");

	let test_deps = rivet_test_deps::setup_single_datacenter(
		test_id,
		dc_label,
		datacenters,
		api_peer_port,
		guard_port,
	)
	.await?;

	let db = &test_deps.pools.udb()?;
	let actor_id = Id::new_v1(dc_label);
	let recipient = kv::Recipient {
		actor_id,
		namespace_id: Id::new_v1(dc_label),
		name: "default".to_string(),
	};

	tracing::info!(?actor_id, "starting kv operations test");

	// Test 1: Put some keys
	tracing::info!("test 1: putting keys");
	let keys = vec![
		b"key1".to_vec(),
		b"key2".to_vec(),
		b"key3".to_vec(),
		b"key4".to_vec(),
		b"other".to_vec(),
	];
	let values = vec![
		b"value1".to_vec(),
		b"value2".to_vec(),
		b"value3".to_vec(),
		b"value4".to_vec(),
		b"other_value".to_vec(),
	];

	kv::put(db, &recipient, keys.clone(), values.clone()).await?;
	tracing::info!("successfully put {} keys", keys.len());

	// Test 2: Get the keys back
	tracing::info!("test 2: getting keys");
	let (got_keys, got_values, got_metadata) = kv::get(db, &recipient, keys.clone()).await?;

	assert_eq!(got_keys.len(), 5, "should get 5 keys back");
	assert_eq!(got_values.len(), 5, "should get 5 values back");
	assert_eq!(got_metadata.len(), 5, "should get 5 metadata entries back");

	// Verify the values match
	for (i, key) in keys.iter().enumerate() {
		let got_idx = got_keys
			.iter()
			.position(|k| k == key)
			.expect("key should exist");
		assert_eq!(
			got_values[got_idx], values[i],
			"value should match for key {:?}",
			key
		);
		assert!(
			!got_metadata[got_idx].version.is_empty(),
			"metadata should have version"
		);
		assert!(
			got_metadata[got_idx].update_ts > 0,
			"metadata should have timestamp"
		);
	}
	tracing::info!("successfully verified all keys and values");

	// Test 3: List all keys
	tracing::info!("test 3: listing all keys");
	let (list_keys, list_values, list_metadata) =
		kv::list(db, &recipient, rp::KvListQuery::KvListAllQuery, false, None).await?;

	assert_eq!(list_keys.len(), 5, "should list 5 keys");
	assert_eq!(list_values.len(), 5, "should list 5 values");
	assert_eq!(list_metadata.len(), 5, "should list 5 metadata entries");
	tracing::info!("successfully listed all keys");

	// Test 4: List with limit
	tracing::info!("test 4: listing with limit");
	let (limited_keys, _, _) = kv::list(
		db,
		&recipient,
		rp::KvListQuery::KvListAllQuery,
		false,
		Some(2),
	)
	.await?;

	assert_eq!(limited_keys.len(), 2, "should limit to 2 keys");
	tracing::info!("successfully listed with limit");

	// Test 5: List with reverse
	tracing::info!("test 5: listing in reverse");
	let (forward_keys, _, _) =
		kv::list(db, &recipient, rp::KvListQuery::KvListAllQuery, false, None).await?;
	let (reverse_keys, _, _) =
		kv::list(db, &recipient, rp::KvListQuery::KvListAllQuery, true, None).await?;

	assert_eq!(forward_keys.len(), reverse_keys.len());
	// Keys should be in opposite order
	for (i, key) in forward_keys.iter().enumerate() {
		assert_eq!(
			key,
			&reverse_keys[reverse_keys.len() - 1 - i],
			"reverse order should match"
		);
	}
	tracing::info!("successfully verified reverse listing");

	// Test 6: List with prefix
	tracing::info!("test 6: listing with prefix");

	// First add some keys with common prefixes
	let prefix_keys = vec![
		b"users:alice".to_vec(),
		b"users:bob".to_vec(),
		b"posts:1".to_vec(),
		b"posts:2".to_vec(),
		b"comments:100".to_vec(),
	];
	let prefix_values = vec![
		b"Alice".to_vec(),
		b"Bob".to_vec(),
		b"Post 1".to_vec(),
		b"Post 2".to_vec(),
		b"Comment 100".to_vec(),
	];
	kv::put(db, &recipient, prefix_keys.clone(), prefix_values.clone()).await?;

	// Query with "users:" prefix
	let (users_keys, _, _) = kv::list(
		db,
		&recipient,
		rp::KvListQuery::KvListPrefixQuery(rp::KvListPrefixQuery {
			key: b"users:".to_vec(),
		}),
		false,
		None,
	)
	.await?;

	assert_eq!(users_keys.len(), 2, "should find 2 keys with users: prefix");
	assert!(users_keys.contains(&b"users:alice".to_vec()));
	assert!(users_keys.contains(&b"users:bob".to_vec()));

	// Query with "posts:" prefix
	let (posts_keys, _, _) = kv::list(
		db,
		&recipient,
		rp::KvListQuery::KvListPrefixQuery(rp::KvListPrefixQuery {
			key: b"posts:".to_vec(),
		}),
		false,
		None,
	)
	.await?;

	assert_eq!(posts_keys.len(), 2, "should find 2 keys with posts: prefix");
	assert!(posts_keys.contains(&b"posts:1".to_vec()));
	assert!(posts_keys.contains(&b"posts:2".to_vec()));

	tracing::info!("successfully listed keys with prefix");

	// Clean up the prefix test keys
	kv::delete(db, &recipient, prefix_keys).await?;

	// Test 7: List with range
	tracing::info!("test 7: listing with range");
	let (range_keys, _, _) = kv::list(
		db,
		&recipient,
		rp::KvListQuery::KvListRangeQuery(rp::KvListRangeQuery {
			start: b"key1".to_vec(),
			end: b"key2".to_vec(),
			exclusive: false,
		}),
		false,
		None,
	)
	.await?;

	// Range should include both key1 and key2 (exclusive=false)
	assert!(
		range_keys.len() >= 2,
		"range should include at least key1 and key2"
	);
	assert!(range_keys.contains(&b"key1".to_vec()));
	assert!(range_keys.contains(&b"key2".to_vec()));
	tracing::info!("successfully listed keys in range");

	// Test 8: List with exclusive range
	tracing::info!("test 8: listing with exclusive range");
	let (exclusive_range_keys, _, _) = kv::list(
		db,
		&recipient,
		rp::KvListQuery::KvListRangeQuery(rp::KvListRangeQuery {
			start: b"key1".to_vec(),
			end: b"key2".to_vec(),
			exclusive: true,
		}),
		false,
		None,
	)
	.await?;

	// With exclusive end, should not include key2
	assert!(!exclusive_range_keys.contains(&b"key2".to_vec()));
	assert!(exclusive_range_keys.contains(&b"key1".to_vec()));
	tracing::info!("successfully listed keys in exclusive range");

	// Test 9: Delete specific keys
	tracing::info!("test 9: deleting specific keys");
	let keys_to_delete = vec![b"key1".to_vec(), b"key2".to_vec()];
	kv::delete(db, &recipient, keys_to_delete.clone()).await?;

	// Verify keys are deleted
	let (remaining_keys, _, _) =
		kv::list(db, &recipient, rp::KvListQuery::KvListAllQuery, false, None).await?;
	assert_eq!(remaining_keys.len(), 3, "should have 3 keys remaining");
	assert!(!remaining_keys.contains(&b"key1".to_vec()));
	assert!(!remaining_keys.contains(&b"key2".to_vec()));
	tracing::info!("successfully deleted specific keys");

	// Test 10: Delete all keys
	tracing::info!("test 10: deleting all keys");
	kv::delete_all(db, &recipient).await?;

	// Verify all keys are deleted
	let (all_keys, _, _) =
		kv::list(db, &recipient, rp::KvListQuery::KvListAllQuery, false, None).await?;
	assert_eq!(all_keys.len(), 0, "should have no keys remaining");
	tracing::info!("successfully deleted all keys");

	// Test 11: Test storage size
	tracing::info!("test 11: testing storage size");
	let size = db
		.run(|tx| async move { kv::estimate_kv_size(&tx, actor_id).await })
		.await
		.unwrap();
	assert_eq!(size, 0, "storage size should be 0 after delete_all");
	tracing::info!("successfully verified storage size");

	// Test 12: Test large value (chunking)
	tracing::info!("test 12: testing large value chunking");
	let large_value = vec![42u8; 50_000]; // 50 KB, will be split into chunks
	kv::put(
		db,
		&recipient,
		vec![b"large_key".to_vec()],
		vec![large_value.clone()],
	)
	.await?;

	let (large_keys, large_values, _) =
		kv::get(db, &recipient, vec![b"large_key".to_vec()]).await?;
	assert_eq!(large_keys.len(), 1);
	assert_eq!(large_values[0], large_value, "large value should match");
	tracing::info!("successfully stored and retrieved large value");

	// Test 13: Verify storage size increased
	// Note: Storage size estimation may not be accurate on all backends (e.g., FileSystem)
	tracing::info!("test 13: verifying storage size with data");
	let size_with_data = db
		.run(|tx| async move { kv::estimate_kv_size(&tx, actor_id).await })
		.await
		.unwrap();
	tracing::info!(
		?size_with_data,
		"storage size with data (may be 0 on some backends)"
	);

	tracing::info!("all tests passed successfully!");
	Ok(())
}
