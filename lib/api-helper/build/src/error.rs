use chrono::TimeZone;
use global_error::prelude::*;
use headers::HeaderValue;
use http::Response;
use hyper::Body;
use rivet_util::timestamp::DateTimeExt;
use serde::Serialize;
use uuid::Uuid;

// Sent to the client as json when an error happens
#[derive(Debug, Serialize)]
pub struct ErrorReply {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub code: Option<String>,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub documentation: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

pub fn handle_rejection(
	err: GlobalError,
	mut response: http::response::Builder,
	ray_id: Uuid,
) -> Result<Response<Body>, http::Error> {
	// Log error
	let err = match err {
		GlobalError::BadRequest { .. } => {
			tracing::warn!(?err, "bad request response");
			err
		}
		GlobalError::Internal { .. } => {
			tracing::error!(?err, "internal error response");

			// Replace internal errors with global errors
			if std::env::var("RIVET_API_ERROR_VERBOSE")
				.ok()
				.map_or(false, |x| x == "1")
			{
				err_code!(ERROR, error = err.to_string())
			} else {
				err_code!(
					ERROR,
					error = format!("An internal error has occurred (ray_id {}).", ray_id)
				)
			}
		}
	};

	// Modify request based on error
	match &err {
		GlobalError::BadRequest { code, metadata, .. } => {
			if code == &formatted_error::code::API_RATE_LIMIT {
				if let Some(ts) = metadata
					.as_ref()
					.map(|m| serde_json::from_str::<i64>(m).ok())
					.flatten()
				{
					if let chrono::LocalResult::Single(retry_after_ts) =
						chrono::Utc.timestamp_millis_opt(ts)
					{
						// Add retry-after header
						if let (Some(headers), Ok(retry_after)) = (
							response.headers_mut(),
							HeaderValue::from_str(retry_after_ts.to_rfc7231().as_str()),
						) {
							headers.insert("retry-after", retry_after);
						} else {
							tracing::error!("failed to get response headers");
						}
					}
				} else {
					tracing::error!("failed to get/parse API_RATE_LIMIT metadata");
				}
			}
		}
		_ => {}
	};

	// Build response
	let (status, code, message, documentation) = (
		err.http_status(),
		err.code().map(|s| s.to_string()),
		err.message(),
		err.documentation().map(|s| s.to_string()),
	);

	let metadata = match err.metadata() {
		Ok(metadata) => metadata,
		Err(err) => {
			tracing::error!(?err, "rejection metadata error");
			None
		}
	};

	// Create reply
	let error_reply = ErrorReply {
		code,
		message,
		documentation,
		metadata,
	};
	let body = Body::from(serde_json::to_vec(&error_reply).unwrap_or_default());

	response.status(status).body(body)
}
