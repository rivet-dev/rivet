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
}
