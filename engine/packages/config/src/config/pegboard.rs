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
}
