use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("namespace")]
pub enum Namespace {
	#[error(
		"failed_to_create",
		"Failed to create namespace.",
		"Failed to create namespace: {reason}"
	)]
	FailedToCreate { reason: String },

	#[error("name_not_unique", "Namespace name must be unique.")]
	NameNotUnique,

	#[error("not_found", "The namespace does not exist.")]
	NotFound,

	#[error("not_leader", "Attempting to run operation in non-leader datacenter.")]
	NotLeader,

	#[error(
		"invalid_update",
		"Failed to update namespace.",
		"Failed to update namespace: {reason}"
	)]
	InvalidUpdate { reason: String },
}

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("runner_config")]
pub enum RunnerConfig {
	#[error("invalid", "Invalid runner config.", "Invalid runner config: {reason}")]
	Invalid { reason: String },

	#[error("not_found", "No config for this runner exists.")]
	NotFound,
}
