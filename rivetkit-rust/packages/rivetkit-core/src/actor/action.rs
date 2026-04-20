use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDispatchError {
	pub group: String,
	pub code: String,
	pub message: String,
	pub metadata: Option<JsonValue>,
}

impl ActionDispatchError {
	pub(crate) fn from_anyhow(error: anyhow::Error) -> Self {
		let error = RivetError::extract(&error);
		Self {
			group: error.group().to_owned(),
			code: error.code().to_owned(),
			message: error.message().to_owned(),
			metadata: error.metadata(),
		}
	}
}
