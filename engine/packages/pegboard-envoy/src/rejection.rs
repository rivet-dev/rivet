//! Closing an Envoy connection with a close frame that names the reason instead of dropping the
//! socket or reaching a panic: a message that cannot be decoded, since an envoy sends its
//! messages in order and cannot skip one, or a connect whose registration storage could not
//! finish, such as while tablets reopen after an engine restart.

use rivet_env::capabilities::websocket::CloseFrame;
use rivet_error::*;
use serde::{Deserialize, Serialize};
use vbare::DecodeError;

/// WebSocket close code for a message whose payload is invalid.
const INVALID_PAYLOAD: u16 = 1007;

#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"envoy",
	"malformed_message",
	"The message could not be decoded.",
	"Malformed message: {reason}"
)]
pub(crate) struct MalformedMessage {
	pub reason: String,
}

#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"envoy",
	"connect_overloaded",
	"Too many runners are connecting at once. Retry after the hint.",
	"Envoy connect shed at admission; retry after {retry_after_ms} ms"
)]
pub(crate) struct ConnectOverloaded {
	pub retry_after_ms: u64,
}

#[derive(RivetError, Debug, Serialize, Deserialize)]
#[error(
	"envoy",
	"connect_unavailable",
	"Storage could not register the runner yet. Retry after the hint.",
	"Envoy connect refused before init; retry after {retry_after_ms} ms"
)]
pub(crate) struct ConnectUnavailable {
	pub retry_after_ms: u64,
}

/// Bounds a storage retry hint passed on to an envoy, matching the connect admission hint, so a
/// refused envoy retries well inside its lost threshold.
const CONNECT_RETRY_AFTER_MS: std::ops::RangeInclusive<u64> = 100..=5_000;

/// A message the connection rejected: the close frame that tells the envoy why, and the error
/// the connection ends with.
pub(crate) struct Rejection {
	pub frame: CloseFrame,
	pub error: anyhow::Error,
}

impl Rejection {
	/// Classifies an envoy message that failed to decode as malformed.
	pub(crate) fn decode(error: DecodeError) -> Self {
		let reason = match error.into_conversion() {
			Ok(conversion) => conversion.to_string(),
			Err(error) => error.to_string(),
		};
		let error = MalformedMessage { reason }.build();

		let reason = crate::routes::close_reason(&error);
		Rejection {
			frame: CloseFrame {
				code: INVALID_PAYLOAD,
				reason: reason.into(),
			},
			error,
		}
	}

	/// Classifies a connect that failed before the envoy was sent its init. Storage that refused
	/// the work or ran out of time closes with `1013` and `envoy.connect_unavailable` carrying
	/// the storage hint, or `default_retry_after_ms` when it gave none. Any other failure closes
	/// with `1011` and the error's own `<group>.<code>`.
	pub(crate) fn connect(error: anyhow::Error, default_retry_after_ms: u64) -> Self {
		let frame = match connect_retry_after_ms(&error, default_retry_after_ms) {
			Some(retry_after_ms) => {
				let refusal = ConnectUnavailable { retry_after_ms }.build();
				CloseFrame {
					code: crate::routes::TRY_AGAIN_LATER,
					reason: crate::routes::retry_close_reason(&refusal, retry_after_ms).into(),
				}
			}
			None => CloseFrame {
				code: crate::routes::INTERNAL_ERROR,
				reason: crate::routes::close_reason(&error).into(),
			},
		};
		Rejection { frame, error }
	}
}

/// The retry hint for a connect that storage refused or could not finish in time, or `None` for
/// a failure a retry would not fix.
fn connect_retry_after_ms(error: &anyhow::Error, default_retry_after_ms: u64) -> Option<u64> {
	if let Some(retry_after_ms) = depot_client::retry_after_ms(error) {
		let retry_after_ms = retry_after_ms.clamp(
			*CONNECT_RETRY_AFTER_MS.start(),
			*CONNECT_RETRY_AFTER_MS.end(),
		);
		return Some(retry_after_ms);
	}
	let structured = error
		.chain()
		.find_map(|cause| cause.downcast_ref::<rivet_error::RivetError>())?;
	let transient = matches!(
		(structured.group(), structured.code()),
		("depot", "request_deadline_exceeded" | "unavailable")
			| ("ups", "timeout" | "overloaded" | "endpoint_unresolved")
	);
	if transient {
		Some(default_retry_after_ms)
	} else {
		None
	}
}


