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
}
