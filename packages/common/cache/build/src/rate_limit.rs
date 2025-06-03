use futures_util::StreamExt;
use rivet_cache_result::RateLimitResult;

use super::*;

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
	#[tracing::instrument(skip_all)]
	pub async fn rate_limit(
		&self,
		key: &impl CacheKey,
		remote_address: impl AsRef<str>,
		rate_limit_config: RateLimitConfig,
	) -> Vec<RateLimitResult> {
		let remote_address = remote_address.as_ref();

		// NOTE: Impossible for bucket list to be empty, validated at compile time
		// Count the number of buckets
		let buckets_count = rate_limit_config.buckets.len();

		let results = rate_limit_config
			.buckets
			.into_iter()
			.map(|bucket| RateLimitResult::new(bucket.count, bucket.bucket_duration_ms));

		// Increment the bucket hit count
		let results = futures_util::stream::iter(results)
			.map(|result| {
				// Create a clone for the async move
				let result_clone = result.clone();

				// Get rate limit key with driver-specific formatting
				let rate_limit_key = self.driver.process_rate_limit_key(
					key,
					remote_address,
					result.bucket_index(),
					result.ttl_ms(),
				);

				// Execute rate limit increment using the appropriate driver
				Box::pin(async move {
					match self
						.driver
						.rate_limit_increment(&rate_limit_key, result_clone.ttl_ms())
						.await
					{
						Ok(incr) => (result_clone, Some(incr)),
						Err(err) => {
							tracing::error!(
								?err,
								key=?rate_limit_key,
								"failed to increment rate limit key"
							);
							(result_clone, None)
						}
					}
				}) as futures_util::future::BoxFuture<'_, (RateLimitResult, Option<i64>)>
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
		if results.len() != buckets_count {
			return results
				.into_iter()
				.map(|(result, _)| result)
				.collect::<Vec<_>>();
		}

		// Check all results' validity
		for (result, incr) in &mut results {
			result.is_valid = *incr <= result.max_hits_per_bucket();
		}

		let formatted_results = results
			.iter()
			.map(|(result, incr)| format!("(incr = {}, result = {})", incr, result))
			.collect::<Vec<_>>();
		tracing::trace!(
			results=?formatted_results,
			"registered rate limit hit"
		);

		results
			.into_iter()
			.map(|(result, _)| result)
			.collect::<Vec<_>>()
	}
}
