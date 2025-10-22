use crate::error::RivetError;
use serde::Serialize;
use std::marker::PhantomData;

/// Private marker type that proves an error was created through the macro
#[derive(Debug)]
pub struct MacroMarker {
	pub _private: (),
}

/// Can only be created through the rivet_error macro.
#[derive(Debug)]
pub struct RivetErrorSchema {
	pub group: &'static str,
	pub code: &'static str,
	pub default_message: &'static str,
	#[allow(dead_code)]
	pub meta_type: Option<&'static str>,
	/// Private marker that ensures this was created by the macro
	pub _macro_marker: MacroMarker,
}

/// Can only be created through the rivet_error macro.
#[derive(Debug)]
pub struct RivetErrorSchemaWithMeta<T> {
	pub schema: RivetErrorSchema,
	pub message_fn: fn(&T) -> String,
	pub _phantom: PhantomData<T>,
}

impl RivetErrorSchema {
	/// Internal constructor for built-in errors only (like INTERNAL_ERROR)
	#[doc(hidden)]
	pub(crate) const fn __internal_new(
		group: &'static str,
		code: &'static str,
		message: &'static str,
	) -> Self {
		Self {
			group,
			code,
			default_message: message,
			meta_type: None,
			_macro_marker: MacroMarker { _private: () },
		}
	}

	/// Builds an anyhow::Error from this schema
	pub fn build(&'static self) -> anyhow::Error {
		let error = RivetError {
			schema: self,
			meta: None,
			message: None,
		};
		anyhow::Error::new(error)
	}

	pub(crate) fn build_internal(&'static self, error: &anyhow::Error) -> RivetError {
		RivetError::build_internal(error)
	}
}

impl<T: Serialize> RivetErrorSchemaWithMeta<T> {
	/// Builds an anyhow::Error from this schema with the provided metadata
	pub fn build_with(&'static self, meta: T) -> anyhow::Error {
		let message = (self.message_fn)(&meta);
		let meta_json = serde_json::value::to_raw_value(&meta).ok();

		let error = RivetError {
			schema: &self.schema,
			meta: meta_json,
			message: Some(message),
		};
		anyhow::Error::new(error)
	}
}

impl From<&'static RivetErrorSchema> for RivetError {
	fn from(value: &'static RivetErrorSchema) -> Self {
		RivetError {
			schema: value,
			meta: None,
			message: None,
		}
	}
}
