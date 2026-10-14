use rivet_error::*;
use serde::{Deserialize, Serialize};

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

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("signing")]
pub enum Signing {
	#[error("invalid_input", "Invalid signing request.")]
	InvalidInput,
	#[error(
		"payload_too_large",
		"The signing payload or token exceeds its size limit."
	)]
	PayloadTooLarge,
	#[error("unavailable", "The signing service is temporarily unavailable.")]
	Unavailable,
	#[error(
		"conflict",
		"The signing key generation or request ID conflicts with current state."
	)]
	Conflict,
	#[error("not_found", "The namespace signing keys have not been provisioned.")]
	NotFound,
}
