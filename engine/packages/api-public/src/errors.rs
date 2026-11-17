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
#[error("migration")]
pub enum Migration {
	#[error("actor_not_found", "The actor does not exist in this namespace.")]
	ActorNotFound,
	#[error(
		"wrong_datacenter",
		"The actor belongs to another datacenter.",
		"The actor belongs to datacenter {actor_datacenter_label}, but this is datacenter {datacenter_label}. Call the owning datacenter's API."
	)]
	WrongDatacenter {
		actor_datacenter_label: u16,
		datacenter_label: u16,
	},
	#[error(
		"invalid_request",
		"Invalid migration request.",
		"Invalid migration request: {message}"
	)]
	InvalidRequest { message: String },
	#[error(
		"cutover_mismatch",
		"The actor was parked by a different cutover.",
		"The actor was parked by cutover {parked_cutover_ts}, not {cutover_ts}."
	)]
	CutoverMismatch {
		parked_cutover_ts: i64,
		cutover_ts: i64,
	},
	#[error(
		"backfill_incomplete",
		"Migration is unavailable until the actor migration fix backfill finishes.",
		"Migration is unavailable until the actor migration fix backfill finishes. It runs on engine start; retry once it completes."
	)]
	BackfillIncomplete,
}
