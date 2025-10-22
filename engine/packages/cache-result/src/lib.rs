#[derive(Debug, Clone)]
pub struct RateLimitResult {
	pub is_valid: bool,

	/// How long each bucket lives
	ttl_ms: i64,

	/// The bucket to put hits in based on the current timestamp. When this
	/// index changes, the rate limit is reset.
	bucket_index: i64,

	/// How long until the next bucket.
	retry_after_ts: i64,

	/// Maximum amount of hits allowed in this bucket.
	max_hits_per_bucket: i64,
}

impl RateLimitResult {
	/// Count is in hits.
	pub fn new(count: u64, bucket_duration_ms: i64) -> RateLimitResult {
		let bucket_index = rivet_util::timestamp::now() / bucket_duration_ms;
		let retry_after_ts = (bucket_index + 1) * bucket_duration_ms;
		let max_hits_per_bucket =
			count as i64 * bucket_duration_ms / rivet_util::duration::minutes(1);

		RateLimitResult {
			is_valid: true,
			ttl_ms: bucket_duration_ms,
			bucket_index,
			retry_after_ts,
			max_hits_per_bucket,
		}
	}

	pub fn ttl_ms(&self) -> i64 {
		self.ttl_ms
	}

	pub fn bucket_index(&self) -> i64 {
		self.bucket_index
	}

	pub fn retry_after_ts(&self) -> i64 {
		self.retry_after_ts
	}

	pub fn max_hits_per_bucket(&self) -> i64 {
		self.max_hits_per_bucket
	}
}

impl std::fmt::Display for RateLimitResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_map()
			.entry(&"is_valid", &self.is_valid)
			.entry(&"retry_after_ts", &self.retry_after_ts)
			.entry(&"max_hits_per_bucket", &self.max_hits_per_bucket)
			.finish()
	}
}
