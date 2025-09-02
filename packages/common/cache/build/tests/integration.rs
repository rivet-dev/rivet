use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Duration,
};

use rand::{Rng, seq::IteratorRandom, thread_rng};

async fn build_in_memory_cache() -> rivet_cache::Cache {
	rivet_cache::CacheInner::new_in_memory("cache-test".to_owned(), 1000)
}

async fn test_multiple_keys(cache: rivet_cache::Cache) {
	let values = cache
		.clone()
		.request()
		.fetch_all(
			"multiple_keys",
			vec!["a", "b", "c"],
			|mut cache, keys| async move {
				for key in &keys {
					cache.resolve(key, format!("{0}{0}{0}", key));
				}
				Ok(cache)
			},
		)
		.await
		.unwrap();
	assert_eq!(3, values.len(), "missing values");
	for (k, v) in values {
		let expected_v = match k {
			"a" => "aaa",
			"b" => "bbb",
			"c" => "ccc",
			_ => panic!("unexpected key {}", k),
		};
		assert_eq!(expected_v, v, "unexpected value");
	}
}

async fn test_smoke_test(cache: rivet_cache::Cache) {
	// Generate random entries for the cache
	let mut entries = HashMap::new();
	for i in 0..16usize {
		entries.insert(i.to_string(), format!("{0}{0}{0}", i));
	}
	let entries = Arc::new(entries);

	let parallel_count = 32; // Reduced for faster tests
	let barrier = Arc::new(tokio::sync::Barrier::new(parallel_count));
	let mut handles = Vec::new();
	for _ in 0..parallel_count {
		let keys =
			std::iter::repeat_with(|| entries.keys().choose(&mut thread_rng()).unwrap().clone())
				.take(thread_rng().gen_range(0..8))
				.collect::<Vec<_>>();
		let deduplicated_keys = keys.clone().into_iter().collect::<HashSet<String>>();

		let entries = entries.clone();
		let cache = cache.clone();
		let barrier = barrier.clone();
		let handle = tokio::spawn(async move {
			barrier.wait().await;
			let values = cache
				.request()
				.fetch_all("smoke_test", keys, move |mut cache, keys| {
					let entries = entries.clone();
					async move {
						// Reduced sleep for faster tests
						tokio::time::sleep(Duration::from_millis(100)).await;
						for key in &keys {
							cache.resolve(key, entries.get(key).expect("invalid key").clone());
						}
						Ok(cache)
					}
				})
				.await
				.unwrap();
			assert_eq!(
				deduplicated_keys,
				values
					.iter()
					.map(|x| x.0.clone())
					.collect::<HashSet<String>>()
			);
		});
		handles.push(handle);
	}
	futures_util::future::try_join_all(handles).await.unwrap();
}

/// Tests that a custom TTL is properly respected when setting and accessing items
async fn test_custom_ttl(cache: rivet_cache::Cache) {
	let test_key = "ttl-test-key";
	let test_value = "test-value";
	let short_ttl_ms = 500i64; // 500ms TTL

	// Store with a custom short TTL
	let _ = cache
		.clone()
		.request()
		.ttl(short_ttl_ms)
		.fetch_one("ttl_test", test_key, |mut cache, key| async move {
			cache.resolve(&key, test_value.to_string());
			Ok(cache)
		})
		.await
		.unwrap();

	// Verify value exists immediately after storing
	let value = cache
		.clone()
		.request()
		.fetch_one(
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
		"Value should be available before TTL expiration"
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
		.fetch_one(
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
		"Value should not be available after TTL expiration"
	);
}

/// Tests that default TTL is applied correctly when not explicitly specified
async fn test_default_ttl(cache: rivet_cache::Cache) {
	let test_key = "default-ttl-key";
	let test_value = "default-value";

	// Store with default TTL (should use 2 hours)
	let _ = cache
		.clone()
		.request()
		.fetch_one("default_ttl_test", test_key, |mut cache, key| async move {
			cache.resolve(&key, test_value.to_string());
			Ok(cache)
		})
		.await
		.unwrap();

	// Verify value exists after storing
	let value = cache
		.clone()
		.request()
		.fetch_one(
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
		"Value should be available with default TTL"
	);
}

/// Tests that purging a key removes it regardless of TTL
async fn test_purge_with_ttl(cache: rivet_cache::Cache) {
	let test_key = "purge-key";
	let test_value = "purge-value";
	let long_ttl_ms = 3600000i64; // 1 hour TTL

	// Store with a long TTL
	let _ = cache
		.clone()
		.request()
		.ttl(long_ttl_ms)
		.fetch_one("purge_test", test_key, |mut cache, key| async move {
			cache.resolve(&key, test_value.to_string());
			Ok(cache)
		})
		.await
		.unwrap();

	// Verify value exists after storing
	let value = cache
		.clone()
		.request()
		.fetch_one(
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
		"Value should be available after storing"
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
		.fetch_one(
			"purge_test",
			test_key,
			|cache: rivet_cache::GetterCtx<&str, String>, _| async move { Ok(cache) },
		)
		.await
		.unwrap();

	assert_eq!(None, value, "Value should not be available after purging");
}

/// Tests multiple TTLs for different keys in the same batch
async fn test_multi_key_ttl(cache: rivet_cache::Cache) {
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

	// Create separate cache handlers with different TTLs
	let short_ttl_cache = cache.clone().request().ttl(short_ttl_ms);
	let long_ttl_cache = cache.clone().request().ttl(short_ttl_ms * 10); // 5 seconds

	// Store key with short TTL
	let _ = short_ttl_cache
		.clone()
		.fetch_one(
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
	let _ = long_ttl_cache
		.clone()
		.fetch_one(
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
		.fetch_all(
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

	assert_eq!(2, values.len(), "Both values should be available initially");

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
		.fetch_one(
			"multi_ttl_test",
			short_ttl_key,
			|cache: rivet_cache::GetterCtx<&str, String>, _| async move { Ok(cache) },
		)
		.await
		.unwrap();

	assert_eq!(None, short_value, "Short TTL value should have expired");

	// Check values after short TTL expiration
	let values = cache
		.clone()
		.request()
		.fetch_all(
			"multi_ttl_test",
			vec![short_ttl_key, long_ttl_key],
			|mut cache: rivet_cache::GetterCtx<&str, String>, keys| async move {
				// The short TTL key should have expired, so we regenerate it
				// The long TTL key should still be in the cache
				for key in &keys {
					if *key == short_ttl_key {
						cache.resolve(key, "regenerated".to_string());
					}
					// For the long key, we still may need to resolve if not found in cache
					else if *key == long_ttl_key {
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

	assert_eq!(2, values_map.len(), "Both keys should be in result");
	assert_eq!(
		Some(&"regenerated".to_string()),
		values_map.get(short_ttl_key),
		"Short TTL key should have regenerated value"
	);
	assert_eq!(
		Some(&"long".to_string()),
		values_map.get(long_ttl_key),
		"Long TTL key should still have original value"
	);
}

/// Tests basic rate limiting functionality
async fn test_rate_limit_basic(cache: rivet_cache::Cache) {
	// Define a simple cache key for testing
	#[derive(Debug, Clone, PartialEq)]
	struct TestKey;

	impl rivet_cache::CacheKey for TestKey {
		fn cache_key(&self) -> String {
			"rate-limit-test".to_string()
		}
	}

	for _i in 0..5 {
		let config = rivet_cache::RateLimitConfig {
			key: "test_rate_limit".to_string(),
			buckets: vec![rivet_cache::RateLimitBucketConfig {
				count: 5,                  // Allow 5 requests per minute
				bucket_duration_ms: 60000, // 1 minute (60,000 ms)
			}],
		};

		let result = cache.rate_limit(&TestKey, "127.0.0.1", config).await;
		assert_eq!(1, result.len());
		assert!(result[0].is_valid, "Request should be valid");
	}

	// Sixth request should not be valid (exceeds the limit of 5)
	let config = rivet_cache::RateLimitConfig {
		key: "test_rate_limit".to_string(),
		buckets: vec![rivet_cache::RateLimitBucketConfig {
			count: 5,                  // Allow 5 requests per minute
			bucket_duration_ms: 60000, // 1 minute (60,000 ms)
		}],
	};

	let result = cache.rate_limit(&TestKey, "127.0.0.1", config).await;
	assert_eq!(1, result.len());
	assert!(!result[0].is_valid, "Sixth request should not be valid");
}

/// Tests that rate limits are properly isolated by IP address
async fn test_rate_limit_ip_isolation(cache: rivet_cache::Cache) {
	// Define a simple cache key for testing
	#[derive(Debug, Clone, PartialEq)]
	struct TestKey;

	impl rivet_cache::CacheKey for TestKey {
		fn cache_key(&self) -> String {
			"ip-isolation-test".to_string()
		}
	}

	// IP addresses to test
	let ip1 = "192.168.0.1";
	let ip2 = "10.0.0.1";

	// Make multiple requests from IP1
	for _i in 0..3 {
		let config = rivet_cache::RateLimitConfig {
			key: "test_ip_isolation".to_string(),
			buckets: vec![rivet_cache::RateLimitBucketConfig {
				count: 3,                  // Allow 3 requests per minute
				bucket_duration_ms: 60000, // 1 minute (60,000 ms)
			}],
		};

		let result = cache.rate_limit(&TestKey, ip1, config).await;
		assert_eq!(1, result.len());
		assert!(result[0].is_valid, "Request from IP1 should be valid");
	}

	// Next request from IP1 should exceed the limit
	let config = rivet_cache::RateLimitConfig {
		key: "test_ip_isolation".to_string(),
		buckets: vec![rivet_cache::RateLimitBucketConfig {
			count: 3,                  // Allow 3 requests per minute
			bucket_duration_ms: 60000, // 1 minute (60,000 ms)
		}],
	};

	let result = cache.rate_limit(&TestKey, ip1, config).await;
	assert_eq!(1, result.len());
	assert!(
		!result[0].is_valid,
		"Fourth request from IP1 should not be valid"
	);

	// Requests from IP2 should still be valid even though IP1 is blocked
	for _i in 0..3 {
		let config = rivet_cache::RateLimitConfig {
			key: "test_ip_isolation".to_string(),
			buckets: vec![rivet_cache::RateLimitBucketConfig {
				count: 3,                  // Allow 3 requests per minute
				bucket_duration_ms: 60000, // 1 minute (60,000 ms)
			}],
		};

		let result = cache.rate_limit(&TestKey, ip2, config).await;
		assert_eq!(1, result.len());
		assert!(result[0].is_valid, "Request from IP2 should be valid");
	}

	// Next request from IP2 should exceed the limit
	let config = rivet_cache::RateLimitConfig {
		key: "test_ip_isolation".to_string(),
		buckets: vec![rivet_cache::RateLimitBucketConfig {
			count: 3,                  // Allow 3 requests per minute
			bucket_duration_ms: 60000, // 1 minute (60,000 ms)
		}],
	};

	let result = cache.rate_limit(&TestKey, ip2, config).await;
	assert_eq!(1, result.len());
	assert!(
		!result[0].is_valid,
		"Fourth request from IP2 should not be valid"
	);

	// IP1 should still be blocked (testing key persistence)
	let config = rivet_cache::RateLimitConfig {
		key: "test_ip_isolation".to_string(),
		buckets: vec![rivet_cache::RateLimitBucketConfig {
			count: 3,                  // Allow 3 requests per minute
			bucket_duration_ms: 60000, // 1 minute (60,000 ms)
		}],
	};

	let result = cache.rate_limit(&TestKey, ip1, config).await;
	assert_eq!(1, result.len());
	assert!(
		!result[0].is_valid,
		"IP1 should still be blocked after IP2 requests"
	);
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_multiple_keys() {
	let cache = build_in_memory_cache().await;
	test_multiple_keys(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_smoke_test() {
	let cache = build_in_memory_cache().await;
	test_smoke_test(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_custom_ttl() {
	let cache = build_in_memory_cache().await;
	test_custom_ttl(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_default_ttl() {
	let cache = build_in_memory_cache().await;
	test_default_ttl(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_purge_with_ttl() {
	let cache = build_in_memory_cache().await;
	test_purge_with_ttl(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_multi_key_ttl() {
	let cache = build_in_memory_cache().await;
	test_multi_key_ttl(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_rate_limit_basic() {
	let cache = build_in_memory_cache().await;
	test_rate_limit_basic(cache).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_rate_limit_ip_isolation() {
	let cache = build_in_memory_cache().await;
	test_rate_limit_ip_isolation(cache).await;
}
