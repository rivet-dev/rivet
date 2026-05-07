use rivet_error::{ActorSpecifier, RivetError, RivetErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::{client_error_message, client_error_metadata, is_internal_error};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDispatchError {
	pub group: String,
	pub code: String,
	pub message: String,
	pub metadata: Option<JsonValue>,
	pub actor: Option<ActorSpecifier>,
}

impl ActionDispatchError {
	pub(crate) fn from_anyhow(error: anyhow::Error) -> Self {
		let original_message = error.to_string();
		let error = RivetError::extract(&error);
		let message = if is_internal_error(error.group(), error.code()) {
			original_message
		} else {
			error.message().to_owned()
		};
		Self {
			group: error.group().to_owned(),
			code: error.code().to_owned(),
			message,
			metadata: error.metadata(),
			actor: error.actor().cloned(),
		}
	}

	pub(crate) fn client_message(&self) -> &str {
		client_error_message(&self.group, &self.code, &self.message)
	}

	pub(crate) fn client_metadata(&self) -> Option<&JsonValue> {
		client_error_metadata(&self.group, &self.code, self.metadata.as_ref())
	}

	pub(crate) fn into_anyhow(self) -> anyhow::Error {
		let ActionDispatchError {
			group,
			code,
			message,
			metadata,
			actor,
		} = self;
		let message = client_error_message(&group, &code, &message).to_owned();
		let meta = client_error_metadata(&group, &code, metadata.as_ref())
			.and_then(|value| serde_json::value::to_raw_value(value).ok());
		anyhow::Error::new(RivetError {
			kind: RivetErrorKind::Dynamic {
				group,
				code,
				default_message: message.clone(),
			},
			meta,
			message: Some(message),
			actor,
		})
	}
}

// Test shim keeps moved tests under tests while retaining private module access.
#[cfg(test)]
#[path = "../../tests/modules/action_dispatch_error.rs"]
mod tests;
