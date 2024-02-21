use super::*;
use futures_util::StreamExt;
use rivet_cache_result::RateLimitResult;

pub struct RateLimitConfig {
	pub key: String,
	pub buckets: Vec<RateLimitBucketConfig>,
}

pub struct RateLimitBucketConfig {
	pub count: u64,
	pub bucket_duration_ms: i64,
}

impl CacheInner {
	/// Rate limits a given key.
	///
	/// This is infallible in order to make sure that anything that depends on
	/// this never fails.
	pub async fn rate_limit(
		&self,
		key: &impl CacheKey,
		remote_address: impl AsRef<str>,
		rate_limit_config: RateLimitConfig,
	) -> Vec<RateLimitResult> {
		let remote_address = remote_address.as_ref();

		// NOTE: Impossible for bucket list to be empty, validated at compile time
		let results = rate_limit_config
			.buckets
			.into_iter()
			.map(|bucket| RateLimitResult::new(bucket.count, bucket.bucket_duration_ms));
		let original_len = results.len();

		if *rivet_util::debug::DISABLE_RATE_LIMIT {
			return results.collect::<Vec<_>>();
		}

		// Increment the bucket hit count
		let results = futures_util::stream::iter(results)
			.map(|result| {
				let mut conn = self.redis_conn.clone();
				let key = self.build_redis_rate_limit_key(
					key,
					remote_address,
					result.bucket_index(),
					result.ttl_ms(),
				);
				let mut pipe = redis::pipe();
				pipe.atomic();
				pipe.incr(&key, 1);
				pipe.pexpire(&key, result.ttl_ms()).ignore();

				async move {
					match pipe.query_async::<_, (i64,)>(&mut conn).await {
						Ok((incr,)) => (result, Some(incr)),
						Err(err) => {
							tracing::error!(?err, ?key, "failed to increment rate limit key");
							(result, None)
						}
					}
				}
			})
			.buffer_unordered(16)
			.collect::<Vec<_>>()
			.await;

		// Filter out errors
		let mut results = results
			.into_iter()
			.filter_map(|(result, incr)| incr.map(|incr| (result, incr)))
			.collect::<Vec<_>>();

		// Gracefully return when any of the above futures error
		if results.len() != original_len {
			return results
				.into_iter()
				.map(|(result, _)| result)
				.collect::<Vec<_>>();
		}

		// Check all results' validity
		for (result, incr) in &mut results {
			result.is_valid = *incr <= result.max_hits_per_bucket()
		}

		let formatted_results = results
			.iter()
			.map(|(result, incr)| format!("(incr = {}, result = {})", incr, result))
			.collect::<Vec<_>>();
		tracing::info!(
			?key,
			?remote_address,
			results=?formatted_results,
			"registered rate limit hit"
		);

		results
			.into_iter()
			.map(|(result, _)| result)
			.collect::<Vec<_>>()
	}
}
