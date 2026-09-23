use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

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
}
