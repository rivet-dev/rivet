use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("ups")]
pub enum Ups {
	#[error("no_responders", "No responders.")]
	NoResponders,

	#[error("request_timeout", "Request timeout.")]
	RequestTimeout,
}
