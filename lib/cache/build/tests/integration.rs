use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Duration,
};

use rand::{seq::IteratorRandom, thread_rng, Rng};
use uuid::Uuid;

async fn build_cache() -> rivet_cache::Cache {
	let redis_conn = redis::Client::open(todo!())
		.unwrap()
		.get_tokio_connection_manager()
		.await
		.unwrap();

	// Unique hash will validate that each test is isolated
	let build_hash = Uuid::new_v4().to_string();
	rivet_cache::CacheInner::new("cache-test".to_owned(), build_hash, redis_conn)
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_keys() {
	let cache = build_cache().await;

	let values = cache
		.clone()
		.request()
		.immutable()
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

#[tokio::test(flavor = "multi_thread")]
async fn smoke_test() {
	let cache = build_cache().await;

	// Generate random entries for the cache
	let mut entries = HashMap::new();
	for i in 0..16usize {
		entries.insert(i.to_string(), format!("{0}{0}{0}", i));
	}
	let entries = Arc::new(entries);

	let parallel_count = 128;
	let barrier = Arc::new(tokio::sync::Barrier::new(parallel_count));
	let mut handles = Vec::new();
	for _ in 0..parallel_count {
		let keys =
			std::iter::repeat_with(|| entries.keys().choose(&mut thread_rng()).unwrap().clone())
				.take(thread_rng().gen_range(0..8))
				.collect::<Vec<_>>();
		let deduplicated_keys = keys.clone().into_iter().collect::<HashSet<String>>();
		dbg!(&keys);
		dbg!(&deduplicated_keys);

		let entries = entries.clone();
		let cache = cache.clone();
		let barrier = barrier.clone();
		let handle = tokio::spawn(async move {
			barrier.wait().await;
			let values = cache
				.request()
				.immutable()
				.fetch_all("smoke_test", keys, move |mut cache, keys| {
					let entries = entries.clone();
					async move {
						tokio::time::sleep(Duration::from_secs(1)).await;
						for key in &keys {
							cache.resolve(key, entries.get(key).expect("invalid key").clone());
						}
						Ok(cache)
					}
				})
				.await
				.unwrap();
			dbg!(&values);
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
