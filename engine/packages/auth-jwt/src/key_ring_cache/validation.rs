use crate::{
	DecodedToken, PROTOCOL_CLOCK_SKEW, SIGNING_CACHE_LEASE, TokenError, VerificationKey, decode,
};

use super::{VerificationFailure, VerificationKeyLifecycle, snapshot::CachedSnapshot};

pub(super) fn decode_with_snapshot(
	token: &str,
	key: &VerificationKey,
	snapshot: &CachedSnapshot,
	now: u64,
) -> Result<(DecodedToken, bool), VerificationFailure> {
	let decode_for = |issuer: &str| {
		decode(
			token,
			key,
			crate::DecodeOptions {
				issuer,
				audience: &snapshot.audience,
				now,
			},
		)
	};

	match decode_for(&snapshot.active_issuer) {
		Ok(token) => return Ok((token, false)),
		Err(TokenError::Expired) => return Err(VerificationFailure::Expired),
		Err(_) => {}
	}

	let now_ms = i64::try_from(now).unwrap_or(i64::MAX).saturating_mul(1_000);
	// Retiring issuers remain accepted only for their bounded migration overlap.
	for issuer in snapshot
		.retiring_issuers
		.iter()
		.filter(|issuer| now_ms < issuer.accept_until_ts)
	{
		match decode_for(&issuer.issuer) {
			Ok(token) => return Ok((token, true)),
			Err(TokenError::Expired) => return Err(VerificationFailure::Expired),
			Err(_) => {}
		}
	}

	Err(VerificationFailure::InvalidToken)
}

pub(super) fn validate_lifecycle(
	decoded: &DecodedToken,
	lifecycle: &VerificationKeyLifecycle,
	now_seconds: u64,
) -> Result<(), VerificationFailure> {
	let to_seconds = |milliseconds: i64| u64::try_from(milliseconds / 1000).unwrap_or(0);
	match lifecycle {
		VerificationKeyLifecycle::Pending { activate_after_ts } => {
			// Pending keys are distributed before activation so verifiers already know the public key
			// when signing switches. Possession of the pending private key must not authorize tokens
			// before the scheduled activation, apart from normal protocol clock skew.
			if decoded.claims.iat.saturating_add(PROTOCOL_CLOCK_SKEW)
				< to_seconds(*activate_after_ts)
			{
				return Err(VerificationFailure::InvalidToken);
			}
		}
		VerificationKeyLifecycle::Active {
			activated_ts,
			sign_until_ts,
		} => {
			if decoded.claims.iat.saturating_add(PROTOCOL_CLOCK_SKEW) < to_seconds(*activated_ts)
				|| decoded.claims.iat
					> to_seconds(*sign_until_ts).saturating_add(PROTOCOL_CLOCK_SKEW)
			{
				return Err(VerificationFailure::InvalidToken);
			}
		}
		VerificationKeyLifecycle::Retiring {
			retired_ts,
			max_token_exp_ts,
			verify_until_ts,
		} => {
			// Normal rotation permits one signing-cache lease after retirement, plus clock skew.
			// The persisted expiration/verification bounds cover that same allowance. Emergency
			// revocation removes the key entirely and does not use this grace period.
			if now_seconds >= to_seconds(*verify_until_ts)
				|| decoded.claims.iat
					> to_seconds(*retired_ts)
						.saturating_add(SIGNING_CACHE_LEASE.as_secs())
						.saturating_add(PROTOCOL_CLOCK_SKEW)
				|| decoded.claims.exp > to_seconds(*max_token_exp_ts)
			{
				return Err(VerificationFailure::InvalidToken);
			}
		}
	}
	Ok(())
}

pub(super) fn map_token_error(error: TokenError) -> VerificationFailure {
	match error {
		TokenError::Expired => VerificationFailure::Expired,
		TokenError::TokenTooLarge
		| TokenError::InvalidEncoding
		| TokenError::InvalidHeader
		| TokenError::InvalidKeyId
		| TokenError::InvalidKey
		| TokenError::InvalidSignature
		| TokenError::InvalidClaims
		| TokenError::IssuedInFuture
		| TokenError::LifetimeTooLong
		| TokenError::InvalidGrants => VerificationFailure::InvalidToken,
	}
}
