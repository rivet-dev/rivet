use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pegboard {
	/// Time to delay an actor from rescheduling after a rescheduling failure.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub base_retry_timeout: Option<usize>,
	/// How long to wait after creating and not receiving a starting state before setting actor as lost.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub actor_start_threshold: Option<i64>,
	/// How long to wait after stopping and not receiving a stop state before setting actor as lost.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub actor_stop_threshold: Option<i64>,
	/// How long an actor goes without retries before it's retry count is reset to 0, effectively resetting its
	/// backoff to 0.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub retry_reset_duration: Option<i64>,
	/// Maximum exponent for the reschedule backoff calculation.
	///
	/// This controls the maximum backoff duration when rescheduling actors.
	///
	/// **Experimental**
	pub reschedule_backoff_max_exponent: Option<usize>,
	/// How long after last ping before considering a runner ineligible for allocation.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub runner_eligible_threshold: Option<i64>,
	/// How long to wait after last ping before forcibly removing a runner from the database
	/// and deleting its workflow, evicting all actors.
	///
	/// Note that the runner may still be running and can reconnect.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub runner_lost_threshold: Option<i64>,
	/// How long after last ping before considering a hibernating request disconnected.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub hibernating_request_eligible_threshold: Option<i64>,
	/// Time to delay a serverless runner from attempting a new outbound connection after a connection failure.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub serverless_base_retry_timeout: Option<usize>,
	/// How long a serverless runner goes without connection failures before it's retry count is reset to 0,
	/// effectively resetting its backoff to 0.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub serverless_retry_reset_duration: Option<i64>,
	/// Maximum exponent for the serverless backoff calculation.
	///
	/// This controls the maximum backoff duration when serverlessly connecting to runners.
	///
	/// **Experimental**
	pub serverless_backoff_max_exponent: Option<usize>,

	/// Global pool desired max.
	pub pool_desired_max_override: Option<u32>,

	/// Default metadata poll interval for serverless runners when not specified in runner config.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub default_metadata_poll_interval: Option<u64>,

	/// Minimum metadata poll interval for serverless runners.
	///
	/// The actual poll interval will be the maximum of this value and the runner config's
	/// `metadata_poll_interval` setting. This prevents excessive polling even if the
	/// runner config specifies a very short interval.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub min_metadata_poll_interval: Option<u64>,

	/// Number of consecutive successes required to clear an active runner pool error.
	///
	/// This prevents a single success from clearing an error during flapping conditions.
	/// Higher values provide more stability but slower recovery from transient errors.
	///
	/// **Experimental**
	pub runner_pool_consecutive_successes_to_clear_error: Option<u32>,
	/// Amount of runners to query from the allocation queue and choose at random when allocating an actor.
	///
	/// **Experimental**
	pub actor_allocation_candidate_sample_size: Option<usize>,
}

impl Pegboard {
	pub fn base_retry_timeout(&self) -> usize {
		self.base_retry_timeout.unwrap_or(2000)
	}

	pub fn actor_start_threshold(&self) -> i64 {
		self.actor_start_threshold.unwrap_or(30_000)
	}

	pub fn actor_stop_threshold(&self) -> i64 {
		self.actor_stop_threshold.unwrap_or(30_000)
	}

	pub fn retry_reset_duration(&self) -> i64 {
		self.retry_reset_duration.unwrap_or(10 * 60 * 1000)
	}

	pub fn reschedule_backoff_max_exponent(&self) -> usize {
		self.reschedule_backoff_max_exponent.unwrap_or(8)
	}

	pub fn runner_eligible_threshold(&self) -> i64 {
		self.runner_eligible_threshold.unwrap_or(10_000)
	}

	pub fn runner_lost_threshold(&self) -> i64 {
		self.runner_lost_threshold.unwrap_or(15_000)
	}

	pub fn hibernating_request_eligible_threshold(&self) -> i64 {
		self.hibernating_request_eligible_threshold
			.unwrap_or(90_000)
	}

	pub fn serverless_base_retry_timeout(&self) -> usize {
		self.serverless_base_retry_timeout.unwrap_or(2000)
	}

	pub fn serverless_retry_reset_duration(&self) -> i64 {
		self.serverless_retry_reset_duration
			.unwrap_or(10 * 60 * 1000)
	}

	pub fn serverless_backoff_max_exponent(&self) -> usize {
		self.serverless_backoff_max_exponent.unwrap_or(8)
	}

	pub fn runner_pool_error_consecutive_successes_to_clear(&self) -> u32 {
		self.runner_pool_consecutive_successes_to_clear_error
			.unwrap_or(3)
	}

	pub fn default_metadata_poll_interval(&self) -> u64 {
		self.default_metadata_poll_interval.unwrap_or(10_000)
	}

	pub fn min_metadata_poll_interval(&self) -> u64 {
		self.min_metadata_poll_interval.unwrap_or(5_000)
	}

	pub fn actor_allocation_candidate_sample_size(&self) -> usize {
		self.actor_allocation_candidate_sample_size.unwrap_or(100)
	}
}
