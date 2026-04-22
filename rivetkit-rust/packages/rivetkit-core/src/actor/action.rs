use rivet_error::{MacroMarker, RivetError, RivetErrorSchema};
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

	pub(crate) fn into_anyhow(self) -> anyhow::Error {
		let meta = self
			.metadata
			.and_then(|value| serde_json::value::to_raw_value(&value).ok());
		let schema = Box::leak(Box::new(RivetErrorSchema {
			group: Box::leak(self.group.into_boxed_str()),
			code: Box::leak(self.code.into_boxed_str()),
			default_message: Box::leak(self.message.clone().into_boxed_str()),
			meta_type: None,
			_macro_marker: MacroMarker { _private: () },
		}));
		anyhow::Error::new(RivetError {
			schema,
			meta,
			message: Some(self.message),
		})
	}
}
