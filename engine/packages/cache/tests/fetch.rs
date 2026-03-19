use std::{collections::HashSet, sync::Arc, time::Duration};

use rand::{Rng, seq::IteratorRandom, thread_rng};

fn build_cache() -> rivet_cache::Cache {
	rivet_cache::CacheInner::new_in_memory(1000, None)
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_keys() {
	let cache = build_cache();
	let values = cache
		.clone()
		.request()
		.fetch_all_json_with_keys(
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
	let cache = build_cache();

	// Generate random entries for the cache
	let mut entries = std::collections::HashMap::new();
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
				.fetch_all_json_with_keys("smoke_test", keys, move |mut cache, keys| {
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
