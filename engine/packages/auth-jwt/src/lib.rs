mod claims;
mod codec;
mod error;
mod grants;
mod key_ring;
mod keys;
mod persistence;
mod planner;
mod token_id;
mod verified_jwt;

#[cfg(feature = "service")]
pub mod issuer;
#[cfg(feature = "service")]
pub mod key_ring_cache;
#[cfg(feature = "service")]
pub mod metrics;
#[cfg(feature = "service")]
pub mod ops;
#[cfg(feature = "service")]
pub mod storage_keys;
#[cfg(feature = "service")]
pub mod workflows;

pub use claims::{CLAIMS_VERSION, Claims, PROTOCOL_CLOCK_SKEW, PROTOCOL_MAX_TTL};
pub use codec::{
	DecodeOptions, DecodedToken, MAX_SUBJECT_BYTES, MAX_TOKEN_BYTES, TokenHeader, decode, encode,
	is_reserved_token, peek_header,
};
pub use error::TokenError;
pub use grants::{
	MAX_GRANTS, MAX_OPERATIONS_PER_GRANT, ValidatedGrantSet, decode_grants, encode_grants,
};
pub use key_ring::{
	ABSOLUTE_KEY_LIMIT, ActiveIssuer, ActiveKey, Algorithm, EmergencyReceipt, ISSUER_HISTORY_LIMIT,
	IssuerHistory, IssuerState, NORMAL_KEY_LIMIT, PendingKey, PublicKeyRecord, RetiringIssuer,
	RetiringKey, SigningKeyRecord, SigningKeyRing,
};
pub use keys::{KeyId, SigningKey, VerificationKey};
pub use persistence::{decode_signing_key_ring, encode_signing_key_ring};
pub use planner::{
	IssuerReconcile, RotationPolicy, Transition, activate_pending, bootstrap, emergency_rotate,
	next_wake_ts, prune_retiring, reconcile_issuer, recover_leader, stage_pending,
	stage_pending_forced, validate_emergency_request,
};
pub use rivet_util_id::Id;
pub use token_id::TokenId;
pub use verified_jwt::VerifiedJwt;

pub const TOKEN_TYPE: &str = "rivet-auth+jwt";
pub const ALGORITHM: &str = "EdDSA";

/// Maximum age of a ring read used for signing. Normal retirement budgets this separately
/// from protocol clock skew; changing it requires upgrading verifiers before issuers.
pub const SIGNING_CACHE_LEASE: std::time::Duration = std::time::Duration::from_secs(5);

#[cfg(feature = "service")]
pub fn registry() -> gas::prelude::WorkflowResult<gas::prelude::Registry> {
	workflows::registry()
}
