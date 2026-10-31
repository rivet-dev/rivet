//! Main's key lifecycle limits, independent of storage and cache refresh policy.
use crate::{
	DecodedToken, KeyId, PROTOCOL_CLOCK_SKEW, SIGNING_CACHE_LEASE, SigningKeyRing, TokenError,
};

/// A known public key still has a bounded signing interval. Pending seeds cannot authorize
/// early tokens; retired seeds cannot authorize tokens minted after their signing lease.
pub(crate) fn validate_lifecycle(
	token: &DecodedToken,
	ring: &SigningKeyRing,
	kid: KeyId,
	now_seconds: u64,
) -> Result<(), TokenError> {
	let to_seconds = |milliseconds: i64| u64::try_from(milliseconds / 1000).unwrap_or(0);
	if ring.active.key.kid == kid {
		if token.claims.iat.saturating_add(PROTOCOL_CLOCK_SKEW)
			< to_seconds(ring.active.activated_ts)
			|| token.claims.iat
				> to_seconds(ring.active.sign_until_ts).saturating_add(PROTOCOL_CLOCK_SKEW)
		{
			return Err(TokenError::InvalidClaims);
		}
	} else if let Some(pending) = ring
		.pending
		.as_ref()
		.filter(|pending| pending.key.kid == kid)
	{
		if token.claims.iat.saturating_add(PROTOCOL_CLOCK_SKEW)
			< to_seconds(pending.activate_after_ts)
		{
			return Err(TokenError::InvalidClaims);
		}
	} else if let Some(retiring) = ring
		.retiring
		.iter()
		.find(|retiring| retiring.key.kid == kid)
	{
		if now_seconds >= to_seconds(retiring.verify_until_ts)
			|| token.claims.iat
				> to_seconds(retiring.retired_ts)
					.saturating_add(SIGNING_CACHE_LEASE.as_secs())
					.saturating_add(PROTOCOL_CLOCK_SKEW)
			|| token.claims.exp > to_seconds(retiring.max_token_exp_ts)
		{
			return Err(TokenError::InvalidClaims);
		}
	} else {
		return Err(TokenError::InvalidKeyId);
	}
	Ok(())
}


