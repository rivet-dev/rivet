use std::{collections::HashMap, fmt::Display};

use http::StatusCode;
use serde::Serialize;

#[cfg(feature = "chirp")]
use types::rivet::chirp;

pub type GlobalResult<T> = Result<T, GlobalError>;

#[derive(Debug)]
pub enum GlobalError {
	Internal {
		ty: String,
		message: String,
		debug: String,

		/// If true, will retry the request immediately with a backoff.
		///
		/// This is disabled by default to mitigate amplification of resource
		/// exhaustion-related errors.
		///
		/// This is useful for situations where trying again might be helpful,
		/// such as in race conditions.
		///
		/// This is intentionally not sent in response to other services, since
		/// we should only retry the message on this specific worker.
		retry_immediately: bool,
	},
	BadRequest {
		code: String,
		context: HashMap<String, String>,
		metadata: Option<String>, // JSON string
	},
}

impl Display for GlobalError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			GlobalError::Internal { ty, message, .. } => {
				write!(f, "{} ({})", ty, message)
			}
			GlobalError::BadRequest { code, .. } => {
				write!(f, "{}", code)
			}
		}
	}
}

impl GlobalError {
	pub fn new<E>(err: E) -> GlobalError
	where
		E: std::error::Error,
	{
		let mut ty = std::any::type_name::<E>().to_owned();
		let debug = format!("{:?}", err);

		// Extract more information for type from the debug information. This is
		// helpful to extrapolate enum types like `ManagerError` in to
		// `ManagerError::RpcError`
		let ty_suffix = if let Some((left, _)) = debug.split_once(|c: char| !c.is_alphanumeric()) {
			left
		} else {
			debug.as_str()
		};
		if !ty_suffix.is_empty() && ty_suffix.chars().next().map_or(false, char::is_alphabetic) {
			ty = format!("{}::{}", ty, ty_suffix);
		}

		GlobalError::Internal {
			ty,
			message: format!("{}", err),
			debug,
			retry_immediately: false,
		}
	}

	pub fn bad_request(code: &'static str) -> GlobalError {
		GlobalError::BadRequest {
			code: code.to_string(),
			context: HashMap::new(),
			metadata: None,
		}
	}

	pub fn bad_request_builder(code: &'static str) -> BadRequestBuilder {
		BadRequestBuilder::new().code(code)
	}

	/// Matches this error against a `formatted_error::code` variant.
	pub fn is(&self, err_code: &'static str) -> bool {
		match self {
			GlobalError::BadRequest { ref code, .. } => code == err_code,
			_ => false,
		}
	}

	pub fn http_status(&self) -> StatusCode {
		match self {
			GlobalError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
			GlobalError::BadRequest { code, .. } => formatted_error::parse(code).http_status(),
		}
	}

	pub fn code(&self) -> Option<&str> {
		match self {
			GlobalError::Internal { .. } => None,
			GlobalError::BadRequest { code, .. } => Some(code),
		}
	}

	pub fn message(&self) -> String {
		match self {
			GlobalError::Internal { .. } => format!("{}", self),
			GlobalError::BadRequest { code, context, .. } => {
				if context.is_empty() {
					formatted_error::parse(code).description()
				} else {
					formatted_error::parse(code).format_description(context)
				}
			}
		}
	}

	pub fn documentation(&self) -> Option<&str> {
		match self {
			GlobalError::Internal { .. } => None,
			GlobalError::BadRequest { code, .. } => Some(formatted_error::parse(code).documentation()),
		}
	}

	// Deserializes metadata into `serde_json::Value`
	pub fn metadata(&self) -> GlobalResult<Option<serde_json::Value>> {
		match self {
			GlobalError::Internal { .. } => Ok(None),
			GlobalError::BadRequest { metadata, .. } => metadata
				.as_ref()
				.map(|metadata| serde_json::from_str::<serde_json::Value>(&metadata))
				.transpose()
				.map_err(Into::into),
		}
	}
}

impl<T> From<T> for GlobalError
where
	T: std::error::Error,
{
	fn from(err: T) -> Self {
		GlobalError::new(err)
	}
}

#[cfg(feature = "chirp")]
impl From<GlobalError> for chirp::response::Err {
	fn from(val: GlobalError) -> Self {
		match val {
			GlobalError::Internal {
				ty,
				message,
				debug,
				retry_immediately: _,
			} => chirp::response::Err {
				kind: Some(chirp::response::err::Kind::Internal(
					chirp::response::err::Internal {
						ty,
						message,
						debug,
					},
				)),
			},
			GlobalError::BadRequest {
				code,
				context,
				metadata,
			} => chirp::response::Err {
				kind: Some(chirp::response::err::Kind::BadRequest(
					chirp::response::err::BadRequest {
						code: code.to_owned(),
						context,
						metadata,
					},
				)),
			},
		}
	}
}

#[derive(Default)]
pub struct BadRequestBuilder {
	code: Option<&'static str>,
	context: Option<HashMap<String, String>>,
	metadata: Option<serde_json::Value>,
}

impl BadRequestBuilder {
	pub fn new() -> BadRequestBuilder {
		BadRequestBuilder {
			..Default::default()
		}
	}

	pub fn code(mut self, code: &'static str) -> BadRequestBuilder {
		self.code = Some(code);

		self
	}

	pub fn context(mut self, context: HashMap<String, String>) -> BadRequestBuilder {
		self.context = Some(context);

		self
	}

	pub fn metadata<T: Serialize>(mut self, metadata: T) -> GlobalResult<BadRequestBuilder> {
		self.metadata = Some(serde_json::to_value(metadata)?);

		Ok(self)
	}

	pub fn build(self) -> GlobalError {
		match self.build_inner() {
			Ok(err) => err,
			Err(err) => err,
		}
	}

	fn build_inner(self) -> GlobalResult<GlobalError> {
		Ok(GlobalError::BadRequest {
			code: self.code.ok_or(BadRequestBuilderError)?.to_string(),
			context: self.context.unwrap_or_else(HashMap::new),
			metadata: self.metadata.map(|m| m.to_string()),
		})
	}
}

#[derive(Debug, thiserror::Error)]
#[error("`BadRequest` builder error")]
pub struct BadRequestBuilderError;
