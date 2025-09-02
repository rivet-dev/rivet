use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("datacenter")]
pub enum Datacenter {
	#[error("not_found", "The provided datacenter does not exist.")]
	NotFound,
}

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("validation")]
pub enum Validation {
	#[error(
		"too_many_actor_ids",
		"Too many actor IDs provided",
		"Too many actor IDs provided. Maximum is {max}, got {count}"
	)]
	TooManyActorIds { max: usize, count: usize },
	#[error("no_keys", "No keys provided. At least one key is required.")]
	NoKeys,
	#[error("invalid_input", "Invalid input provided", "{message}")]
	InvalidInput { message: String },
	#[error(
		"race_condition",
		"Race condition detected",
		"Race condition detected between get_or_create and delete operations"
	)]
	RaceCondition,
}
