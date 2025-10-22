use crate::INTERNAL_ERROR;
use crate::schema::RivetErrorSchema;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone)]
pub struct RivetError {
	pub schema: &'static RivetErrorSchema,
	pub meta: Option<Box<serde_json::value::RawValue>>,
	pub message: Option<String>,
}

impl RivetError {
	pub fn extract(error: &anyhow::Error) -> Self {
		error
			.chain()
			.find_map(|x| x.downcast_ref::<Self>())
			.cloned()
			.unwrap_or_else(|| INTERNAL_ERROR.build_internal(error))
	}

	pub(crate) fn build_internal(error: &anyhow::Error) -> Self {
		let error_string = format!("{:?}", error);
		let meta_json = serde_json::json!({
			"error": error_string
		});
		let meta = serde_json::value::to_raw_value(&meta_json).ok();

		Self {
			schema: &INTERNAL_ERROR,
			meta,
			message: None,
			// TODO: Expose the message if in dev
			// message: Some(format!("Internal error: {}", error)),
		}
	}

	pub fn group(&self) -> &'static str {
		self.schema.group
	}

	pub fn code(&self) -> &'static str {
		self.schema.code
	}

	pub fn message(&self) -> &str {
		self.message
			.as_deref()
			.unwrap_or(self.schema.default_message)
	}

	pub fn metadata(&self) -> Option<serde_json::Value> {
		self.meta
			.as_ref()
			.and_then(|raw| serde_json::from_str(raw.get()).ok())
	}
}

impl fmt::Display for RivetError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}: {}", self.code(), self.message())
	}
}

impl std::error::Error for RivetError {}

impl Serialize for RivetError {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeStruct;

		let mut state = if self.meta.is_some() {
			serializer.serialize_struct("RivetError", 4)?
		} else {
			serializer.serialize_struct("RivetError", 3)?
		};

		state.serialize_field("group", self.group())?;
		state.serialize_field("code", self.code())?;
		state.serialize_field("message", self.message())?;

		if let Some(meta) = &self.meta {
			state.serialize_field("meta", meta)?;
		}

		state.end()
	}
}
