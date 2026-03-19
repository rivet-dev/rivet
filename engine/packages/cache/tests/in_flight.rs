use std::{
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::Duration,
};

fn build_cache() -> rivet_cache::Cache {
	rivet_cache::CacheInner::new_in_memory(1000, None)
}

/// Two concurrent requests for the same key. The second should wait for the
/// first's getter to complete, then read from cache without calling its own getter.
#[tokio::test(flavor = "multi_thread")]
async fn dedup_two_requests() {
	let cache = build_cache();
	let call_count = Arc::new(AtomicUsize::new(0));
	let getter_started = Arc::new(tokio::sync::Notify::new());

	let cache1 = cache.clone();
	let count1 = call_count.clone();
	let started1 = getter_started.clone();
	let task1 = tokio::spawn(async move {
		cache1
			.request()
			.fetch_one_json(
				"in_flight_dedup",
				"key1",
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
					let count = count1.clone();
					let started = started1.clone();
					async move {
						count.fetch_add(1, Ordering::SeqCst);
						started.notify_one();
						// Hold the lease long enough for task2 to discover the in-flight entry.
						tokio::time::sleep(Duration::from_millis(100)).await;
						ctx.resolve(&key, "from_getter".to_string());
						Ok(ctx)
					}
				},
			)
			.await
	});

	// Wait until task1's getter is running before launching task2.
	getter_started.notified().await;

	let count2 = call_count.clone();
	let task2 = tokio::spawn(async move {
		cache
			.request()
			.fetch_one_json(
				"in_flight_dedup",
				"key1",
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
					let count = count2.clone();
					async move {
						// Should not be reached: key1 is in-flight.
						count.fetch_add(1, Ordering::SeqCst);
						ctx.resolve(&key, "wrong_value".to_string());
						Ok(ctx)
					}
				},
			)
			.await
	});

	let (r1, r2) = tokio::join!(task1, task2);

	assert_eq!(r1.unwrap().unwrap(), Some("from_getter".to_string()));
	assert_eq!(
		r2.unwrap().unwrap(),
		Some("from_getter".to_string()),
		"waiting request should read the value written to cache by the leased request"
	);
	assert_eq!(
		call_count.load(Ordering::SeqCst),
		1,
		"getter called exactly once despite two concurrent requests for the same key"
	);
}

/// Three concurrent requests for the same key. Only the first should call the
/// getter; the other two wait and then read from cache.
#[tokio::test(flavor = "multi_thread")]
async fn dedup_multiple_waiters() {
	let cache = build_cache();
	let call_count = Arc::new(AtomicUsize::new(0));
	let getter_started = Arc::new(tokio::sync::Notify::new());

	let cache1 = cache.clone();
	let count1 = call_count.clone();
	let started1 = getter_started.clone();
	let task1 = tokio::spawn(async move {
		cache1
			.request()
			.fetch_one_json(
				"in_flight_multi",
				"shared_key",
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
					let count = count1.clone();
					let started = started1.clone();
					async move {
						count.fetch_add(1, Ordering::SeqCst);
						started.notify_one();
						tokio::time::sleep(Duration::from_millis(150)).await;
						ctx.resolve(&key, "shared_value".to_string());
						Ok(ctx)
					}
				},
			)
			.await
	});

	getter_started.notified().await;

	let make_waiter = |cache: rivet_cache::Cache, count: Arc<AtomicUsize>| {
		tokio::spawn(async move {
			cache
				.request()
				.fetch_one_json(
					"in_flight_multi",
					"shared_key",
					move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
						let count = count.clone();
						async move {
							// Should not be reached.
							count.fetch_add(1, Ordering::SeqCst);
							ctx.resolve(&key, "wrong_value".to_string());
							Ok(ctx)
						}
					},
				)
				.await
		})
	};

	let task2 = make_waiter(cache.clone(), call_count.clone());
	let task3 = make_waiter(cache.clone(), call_count.clone());

	let (r1, r2, r3) = tokio::join!(task1, task2, task3);

	assert_eq!(r1.unwrap().unwrap(), Some("shared_value".to_string()));
	assert_eq!(r2.unwrap().unwrap(), Some("shared_value".to_string()));
	assert_eq!(r3.unwrap().unwrap(), Some("shared_value".to_string()));
	assert_eq!(
		call_count.load(Ordering::SeqCst),
		1,
		"getter called exactly once for three concurrent requests"
	);
}

/// Concurrent requests for different keys should not share in-flight state.
/// Each key's getter must be called independently.
#[tokio::test(flavor = "multi_thread")]
async fn independent_keys() {
	let cache = build_cache();
	let call_count = Arc::new(AtomicUsize::new(0));

	let make_task = |cache: rivet_cache::Cache, count: Arc<AtomicUsize>, key: &'static str| {
		tokio::spawn(async move {
			cache
				.request()
				.fetch_one_json(
					"in_flight_independent",
					key,
					move |mut ctx: rivet_cache::GetterCtx<&str, String>, k| {
						let count = count.clone();
						async move {
							count.fetch_add(1, Ordering::SeqCst);
							tokio::time::sleep(Duration::from_millis(50)).await;
							ctx.resolve(&k, format!("val_{k}"));
							Ok(ctx)
						}
					},
				)
				.await
		})
	};

	let t1 = make_task(cache.clone(), call_count.clone(), "key_a");
	let t2 = make_task(cache.clone(), call_count.clone(), "key_b");

	let (r1, r2) = tokio::join!(t1, t2);

	assert_eq!(r1.unwrap().unwrap(), Some("val_key_a".to_string()));
	assert_eq!(r2.unwrap().unwrap(), Some("val_key_b".to_string()));
	assert_eq!(
		call_count.load(Ordering::SeqCst),
		2,
		"getter called once per distinct key"
	);
}

/// A batch request that mixes an already-cached key with an in-flight key.
/// The cached key resolves immediately; the in-flight key waits without the
/// getter being invoked again.
#[tokio::test(flavor = "multi_thread")]
async fn mixed_cached_and_in_flight() {
	let cache = build_cache();

	// Pre-populate "cached_key".
	cache
		.clone()
		.request()
		.fetch_one_json(
			"in_flight_mixed",
			"cached_key",
			|mut ctx: rivet_cache::GetterCtx<&str, String>, key| async move {
				ctx.resolve(&key, "cached_value".to_string());
				Ok(ctx)
			},
		)
		.await
		.unwrap();

	let call_count = Arc::new(AtomicUsize::new(0));
	let getter_started = Arc::new(tokio::sync::Notify::new());

	// Task 1: fetches only the slow key, holds the in-flight lease.
	let cache1 = cache.clone();
	let count1 = call_count.clone();
	let started1 = getter_started.clone();
	let t1 = tokio::spawn(async move {
		cache1
			.request()
			.fetch_one_json(
				"in_flight_mixed",
				"slow_key",
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
					let count = count1.clone();
					let started = started1.clone();
					async move {
						count.fetch_add(1, Ordering::SeqCst);
						started.notify_one();
						tokio::time::sleep(Duration::from_millis(100)).await;
						ctx.resolve(&key, "slow_value".to_string());
						Ok(ctx)
					}
				},
			)
			.await
	});

	getter_started.notified().await;

	// Task 2: fetches both keys at once. "cached_key" is a cache hit and
	// "slow_key" is in-flight, so neither should trigger the getter.
	let count2 = call_count.clone();
	let t2 = tokio::spawn(async move {
		cache
			.request()
			.fetch_all_json(
				"in_flight_mixed",
				vec!["cached_key", "slow_key"],
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, keys| {
					let count = count2.clone();
					async move {
						if !keys.is_empty() {
							// Should not be reached for either key.
							count.fetch_add(1, Ordering::SeqCst);
							for k in &keys {
								ctx.resolve(k, "wrong_value".to_string());
							}
						}
						Ok(ctx)
					}
				},
			)
			.await
	});

	let (r1, r2) = tokio::join!(t1, t2);

	assert_eq!(r1.unwrap().unwrap(), Some("slow_value".to_string()));

	let mut r2_vals = r2.unwrap().unwrap();
	r2_vals.sort();
	assert_eq!(
		r2_vals,
		vec!["cached_value".to_string(), "slow_value".to_string()]
	);
	assert_eq!(
		call_count.load(Ordering::SeqCst),
		1,
		"only the leasing task's getter should be called"
	);
}

/// When the leasing task's getter takes longer than IN_FLIGHT_TIMEOUT (5s),
/// the waiting task should stop waiting and fall back to calling its own getter.
#[tokio::test(flavor = "multi_thread")]
async fn timeout_falls_back_to_getter() {
	let cache = build_cache();
	let call_count = Arc::new(AtomicUsize::new(0));
	let getter_started = Arc::new(tokio::sync::Notify::new());
	let getter_release = Arc::new(tokio::sync::Notify::new());

	// Task 1: holds the in-flight lease for longer than IN_FLIGHT_TIMEOUT.
	let cache1 = cache.clone();
	let count1 = call_count.clone();
	let started1 = getter_started.clone();
	let release1 = getter_release.clone();
	let task1 = tokio::spawn(async move {
		cache1
			.request()
			.fetch_one_json(
				"timeout_ns",
				"key1",
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
					let count = count1.clone();
					let started = started1.clone();
					let release = release1.clone();
					async move {
						count.fetch_add(1, Ordering::SeqCst);
						started.notify_one();
						// Block until told to proceed, simulating a very slow getter.
						release.notified().await;
						ctx.resolve(&key, "task1_value".to_string());
						Ok(ctx)
					}
				},
			)
			.await
	});

	getter_started.notified().await;

	// Task 2: subscribes as a waiter and will time out after IN_FLIGHT_TIMEOUT.
	let count2 = call_count.clone();
	let task2 = tokio::spawn(async move {
		cache
			.request()
			.fetch_one_json(
				"timeout_ns",
				"key1",
				move |mut ctx: rivet_cache::GetterCtx<&str, String>, key| {
					let count = count2.clone();
					async move {
						count.fetch_add(1, Ordering::SeqCst);
						ctx.resolve(&key, "task2_value".to_string());
						Ok(ctx)
					}
				},
			)
			.await
	});

	// Wait for task2 to time out (IN_FLIGHT_TIMEOUT = 5s), then release task1.
	// task2 should have already fallen back to its own getter by the time task1 finishes.
	task2.await.unwrap().unwrap();
	getter_release.notify_one();
	task1.await.unwrap().unwrap();

	assert_eq!(
		call_count.load(Ordering::SeqCst),
		2,
		"both getters should be called: task1 held the lease, task2 timed out and fetched itself"
	);
}
