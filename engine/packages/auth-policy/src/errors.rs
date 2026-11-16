use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

/// How long a caller refused with `auth.operation_unavailable` waits before retrying. The refusal
/// lasts until the engine upgrade finishes; the hint only paces the retries.
pub const OPERATION_UNAVAILABLE_RETRY_AFTER_MS: u64 = 5_000;

/// Public authentication failures for administrator tokens and access JWTs.
#[derive(RivetError, Debug, Deserialize, Serialize)]
#[error("auth")]
pub enum Auth {
	#[error("invalid_token", "Authentication token is invalid.")]
	InvalidToken,

	#[error("token_expired", "Authentication token has expired.")]
	TokenExpired,

	#[error(
		"verification_unavailable",
		"The server could not verify the authentication token because verification keys are temporarily unavailable. Retry the request."
	)]
	VerificationUnavailable,

	#[error(
		"insufficient_permissions",
		"Insufficient permissions to access this resource."
	)]
	InsufficientPermissions,

	#[error(
		"issuance_unavailable",
		"The server could not issue an authentication token because the signing service is temporarily unavailable. Retry the request."
	)]
	IssuanceUnavailable,

	#[error(
		"issuance_disabled",
		"JWT issuance is not enabled for this deployment."
	)]
	IssuanceDisabled,

	/// A grant of an operation that an engine worker still running an older release cannot
	/// decode, refused before anything is written or issued.
	#[error(
		"operation_unavailable",
		"The operation cannot be granted until every engine worker supports it. Retry after the engine upgrade finishes.",
		"Grants of the '{operation}' operation are unavailable until every engine worker supports them; retry after {retry_after_ms} ms."
	)]
	OperationUnavailable {
		operation: String,
		retry_after_ms: u64,
	},
}
