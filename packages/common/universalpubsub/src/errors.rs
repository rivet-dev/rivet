use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("ups")]
pub enum Ups {
	#[error("request_timeout", "Request timeout.")]
	RequestTimeout,
}
