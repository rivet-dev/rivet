use std::{borrow::Cow, fmt};

use axum::{
	http::StatusCode,
	response::{IntoResponse, Json, Response},
};
use rivet_error::*;
use serde::{Deserialize, Serialize};

/// API Error wrapper that implements IntoResponse for building error responses
#[derive(Debug)]
pub struct ApiError(anyhow::Error);

impl From<anyhow::Error> for ApiError {
	fn from(err: anyhow::Error) -> Self {
		ApiError(err)
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		let (status, error_response) =
			if let Some(rivet_err) = self.0.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
				let status = match (rivet_err.group(), rivet_err.code()) {
					("api", "not_found") => StatusCode::NOT_FOUND,
					("api", "invalid_token") | ("api", "unauthorized") => StatusCode::UNAUTHORIZED,
					("api", "forbidden") => StatusCode::FORBIDDEN,
					_ => StatusCode::BAD_REQUEST,
				};

				(status, ErrorResponse::from(rivet_err))
			} else if let Some(raw_err) = self
				.0
				.chain()
				.find_map(|x| x.downcast_ref::<RawErrorResponse>())
			{
				(raw_err.0, raw_err.1.clone())
			} else {
				(
					StatusCode::INTERNAL_SERVER_ERROR,
					ErrorResponse::from(&RivetError {
						schema: &rivet_error::INTERNAL_ERROR,
						meta: None,
						message: None,
					}),
				)
			};

		let error_ext = ErrorExt {
			group: error_response.group.clone(),
			code: error_response.code.clone(),
			metadata: error_response.metadata.clone(),
			// If internal error, print information about the root error
			internal: if error_response.group == rivet_error::INTERNAL_ERROR.group
				&& error_response.code == rivet_error::INTERNAL_ERROR.code
			{
				Some(format!("{}", self.0).into())
			} else {
				None
			},
		};

		// Build response
		let mut res = (status, Json(error_response.clone())).into_response();

		// Add extension so we can reference it when logging response
		res.extensions_mut().insert(error_ext);

		res
	}
}

/// JSON data structured used to serialize JSON error responses.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
	pub group: Cow<'static, str>,
	pub code: Cow<'static, str>,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

impl From<&RivetError> for ErrorResponse {
	fn from(value: &RivetError) -> Self {
		ErrorResponse {
			group: value.group().into(),
			code: value.code().into(),
			message: value.message().into(),
			metadata: value.metadata(),
		}
	}
}

/// Error response received from an upstream service. This will re-serialize in to the same error
/// type.
#[derive(Debug, Clone)]
pub struct RawErrorResponse(pub StatusCode, pub ErrorResponse);

impl fmt::Display for RawErrorResponse {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}: {}", self.1.code, self.1.message)
	}
}

impl std::error::Error for RawErrorResponse {}

/// Response extension that includes information about the root error for logging.
#[derive(Clone)]
pub struct ErrorExt {
	pub group: Cow<'static, str>,
	pub code: Cow<'static, str>,
	pub metadata: Option<serde_json::Value>,
	/// If this is an internal error, this provides a formatted string of the root error
	pub internal: Option<Cow<'static, str>>,
}
