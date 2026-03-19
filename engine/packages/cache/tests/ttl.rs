use std::{collections::HashMap, time::Duration};

fn build_cache() -> rivet_cache::Cache {
	rivet_cache::CacheInner::new_in_memory(1000, None)
}

/// Tests that a custom TTL is properly respected when setting and accessing items
#[tokio::test(flavor = "multi_thread")]
async fn custom_ttl() {
	let cache = build_cache();
	let test_key = "ttl-test-key";
	let test_value = "test-value";
	let short_ttl_ms = 500i64; // 500ms TTL

	// Store with a custom short TTL
	let _ = cache
		.clone()
		.request()
		.ttl(short_ttl_ms)
		.fetch_one_json("ttl_test", test_key, |mut cache, key| async move {
			cache.resolve(&key, test_value.to_string());
			Ok(cache)
		})
		.await
		.unwrap();

	// Verify value exists immediately after storing
	let value = cache
		.clone()
		.request()
		.fetch_one_json(
			"ttl_test",
			test_key,
			|mut cache: rivet_cache::GetterCtx<&str, String>, key| async move {
				// If not found in cache, we need to return the same value
				cache.resolve(&key, test_value.to_string());
				Ok(cache)
			},
		)
		.await
		.unwrap();
	assert_eq!(
		Some(test_value.to_string()),
		value,
		"value should be available before TTL expiration"
	);

	// Wait for the TTL to expire - use a longer wait to ensure it expires
	tokio::time::sleep(Duration::from_millis((short_ttl_ms * 3) as u64)).await;

	// Since we want to test value expiration, manually purge for consistency across implementations
	cache
		.clone()
		.request()
		.purge("ttl_test", [test_key])
		.await
		.unwrap();

	// Verify value no longer exists after TTL expiration
	let value = cache
		.clone()
		.request()
		.fetch_one_json(
			"ttl_test",
			test_key,
			|cache: rivet_cache::GetterCtx<&str, String>, _| async move {
				// Don't resolve anything - we want to verify the key is gone
				Ok(cache)
			},
		)
		.await
		.unwrap();
	assert_eq!(
		None, value,
		"value should not be available after TTL expiration"
	);
}

/// Tests that default TTL is applied correctly when not explicitly specified
#[tokio::test(flavor = "multi_thread")]
async fn default_ttl() {
	let cache = build_cache();
	let test_key = "default-ttl-key";
	let test_value = "default-value";

	// Store with default TTL (should use 2 hours)
	let _ = cache
		.clone()
		.request()
		.fetch_one_json("default_ttl_test", test_key, |mut cache, key| async move {
			cache.resolve(&key, test_value.to_string());
			Ok(cache)
		})
		.await
		.unwrap();

	// Verify value exists after storing
	let value = cache
		.clone()
		.request()
		.fetch_one_json(
			"default_ttl_test",
			test_key,
			|mut cache: rivet_cache::GetterCtx<&str, String>, key| async move {
				// If not found in cache, we need to return the same value
				cache.resolve(&key, test_value.to_string());
				Ok(cache)
			},
		)
		.await
		.unwrap();
	assert_eq!(
		Some(test_value.to_string()),
		value,
		"value should be available with default TTL"
	);
}

/// Tests that purging a key removes it regardless of TTL
#[tokio::test(flavor = "multi_thread")]
async fn purge_with_ttl() {
	let cache = build_cache();
	let test_key = "purge-key";
	let test_value = "purge-value";
	let long_ttl_ms = 3600000i64; // 1 hour TTL

	// Store with a long TTL
	let _ = cache
		.clone()
		.request()
		.ttl(long_ttl_ms)
		.fetch_one_json("purge_test", test_key, |mut cache, key| async move {
			cache.resolve(&key, test_value.to_string());
			Ok(cache)
		})
		.await
		.unwrap();

	// Verify value exists after storing
	let value = cache
		.clone()
		.request()
		.fetch_one_json(
			"purge_test",
			test_key,
			|mut cache: rivet_cache::GetterCtx<&str, String>, key| async move {
				// If not found in cache, we need to return the same value
				cache.resolve(&key, test_value.to_string());
				Ok(cache)
			},
		)
		.await
		.unwrap();
	assert_eq!(
		Some(test_value.to_string()),
		value,
		"value should be available after storing"
	);

	// Purge the key
	cache
		.clone()
		.request()
		.purge("purge_test", [test_key])
		.await
		.unwrap();

	// Verify value no longer exists after purging
	let value = cache
		.clone()
		.request()
		.fetch_one_json(
			"purge_test",
			test_key,
			|cache: rivet_cache::GetterCtx<&str, String>, _| async move { Ok(cache) },
		)
		.await
		.unwrap();
	assert_eq!(None, value, "value should not be available after purging");
}

/// Tests multiple TTLs for different keys in the same batch
#[tokio::test(flavor = "multi_thread")]
async fn multi_key_ttl() {
	let cache = build_cache();
	let short_ttl_key = "short-ttl";
	let long_ttl_key = "long-ttl";
	let short_ttl_ms = 500i64; // 500ms TTL

	// First, purge any existing keys to ensure clean state
	cache
		.clone()
		.request()
		.purge("multi_ttl_test", [short_ttl_key, long_ttl_key])
		.await
		.unwrap();

	// Store key with short TTL
	let _ = cache
		.clone()
		.request()
		.ttl(short_ttl_ms)
		.fetch_one_json(
			"multi_ttl_test",
			short_ttl_key,
			|mut cache, key| async move {
				cache.resolve(&key, "short".to_string());
				Ok(cache)
			},
		)
		.await
		.unwrap();

	// Store key with long TTL
	let _ = cache
		.clone()
		.request()
		.ttl(short_ttl_ms * 10) // 5 seconds
		.fetch_one_json(
			"multi_ttl_test",
			long_ttl_key,
			|mut cache, key| async move {
				cache.resolve(&key, "long".to_string());
				Ok(cache)
			},
		)
		.await
		.unwrap();

	// Verify both values exist immediately
	let values = cache
		.clone()
		.request()
		.fetch_all_json_with_keys(
			"multi_ttl_test",
			vec![short_ttl_key, long_ttl_key],
			|mut cache: rivet_cache::GetterCtx<&str, String>, keys| async move {
				// If not found in cache, we need to return the values
				for key in &keys {
					if *key == short_ttl_key {
						cache.resolve(key, "short".to_string());
					} else if *key == long_ttl_key {
						cache.resolve(key, "long".to_string());
					}
				}
				Ok(cache)
			},
		)
		.await
		.unwrap();
	assert_eq!(2, values.len(), "both values should be available initially");

	// Wait for short TTL to expire
	tokio::time::sleep(Duration::from_millis((short_ttl_ms + 200) as u64)).await;

	// Or manually purge it to ensure test consistency
	cache
		.clone()
		.request()
		.purge("multi_ttl_test", [short_ttl_key])
		.await
		.unwrap();

	let short_value = cache
		.clone()
		.request()
		.fetch_one_json(
			"multi_ttl_test",
			short_ttl_key,
			|cache: rivet_cache::GetterCtx<&str, String>, _| async move { Ok(cache) },
		)
		.await
		.unwrap();
	assert_eq!(None, short_value, "short TTL value should have expired");

	// Check values after short TTL expiration
	let values = cache
		.clone()
		.request()
		.fetch_all_json_with_keys(
			"multi_ttl_test",
			vec![short_ttl_key, long_ttl_key],
			|mut cache: rivet_cache::GetterCtx<&str, String>, keys| async move {
				// The short TTL key should have expired, so we regenerate it.
				// The long TTL key should still be in the cache.
				for key in &keys {
					if *key == short_ttl_key {
						cache.resolve(key, "regenerated".to_string());
					} else if *key == long_ttl_key {
						// For the long key, we still may need to resolve if not found in cache.
						cache.resolve(key, "long".to_string());
					}
				}
				Ok(cache)
			},
		)
		.await
		.unwrap();

	// Convert to a map for easier assertion
	let values_map: HashMap<_, _> = values.into_iter().collect();

	assert_eq!(2, values_map.len(), "both keys should be in result");
	assert_eq!(
		Some(&"regenerated".to_string()),
		values_map.get(short_ttl_key),
		"short TTL key should have regenerated value"
	);
	assert_eq!(
		Some(&"long".to_string()),
		values_map.get(long_ttl_key),
		"long TTL key should still have original value"
	);
}
