use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("datacenter")]
pub enum Datacenter {
	#[error("not_found", "The provided datacenter does not exist.")]
	NotFound,
}
