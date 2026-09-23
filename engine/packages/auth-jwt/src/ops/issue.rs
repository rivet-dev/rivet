use anyhow::{Context, Result, ensure};
use gas::prelude::*;
use rand::rngs::OsRng;

use crate::{
	CLAIMS_VERSION, Claims, MAX_SUBJECT_BYTES, TokenId, ValidatedGrantSet, encode, encode_grants,
	issuer::{IssueRequest, IssueResponse},
	metrics,
};

/// The caller supplies the process-owned cache used by its authentication path.
#[derive(Debug)]
pub struct Input {
	pub request: IssueRequest,
	pub key_ring_cache: std::sync::Arc<crate::key_ring_cache::KeyRingCache>,
}

#[operation]
pub async fn auth_jwt_issue(ctx: &OperationCtx, input: &Input) -> Result<IssueResponse> {
	let cache = &input.key_ring_cache;
	let input = &input.request;
	let jwt = &ctx.config().auth_required()?.jwt;
	ensure!(jwt.issuance_enabled(), "JWT issuance is disabled");
	let response = issue_token(
		cache,
		input,
		i64::try_from(jwt.max_duration().as_millis())
			.context("JWT maximum duration does not fit in milliseconds")?,
	)
	.await?;

	metrics::record_issuance("success");
	tracing::info!(
		issuer_token_id = %input.issuer_token_id,
		namespace_id = %input.namespace_id,
		jti = %response.jti,
		kid = %response.kid,
		expires_ts = %response.expires_ts,
		"issued namespace-scoped JWT"
	);
	Ok(response)
}

/// The issuance implementation shared by the Gas operation and cache regression tests.
pub(crate) async fn issue_token(
	cache: &crate::key_ring_cache::KeyRingCache,
	input: &IssueRequest,
	max_duration_ms: i64,
) -> Result<IssueResponse> {
	ensure!(
		input
			.subject
			.as_ref()
			.is_none_or(|subject| subject.len() <= MAX_SUBJECT_BYTES),
		"JWT subject exceeds the protocol limit"
	);
	let grants = ValidatedGrantSet::new(input.namespace_id, input.grants.clone())
		.context("invalid JWT grant set supplied to signer")?;

	let encoded_grants =
		encode_grants(&grants).context("invalid JWT grant set supplied to signer")?;
	cache
		.with_active_signer(|signer| {
			let issued_ts = signer.issued_ts;
			let expires_no_later_than_ts =
				bounded_expiration(issued_ts, input.expires_no_later_than_ts, max_duration_ms)?;

			let iat =
				u64::try_from(issued_ts.div_euclid(1_000)).context("invalid issuance timestamp")?;
			let exp = u64::try_from(expires_no_later_than_ts.div_euclid(1_000))
				.context("invalid expiration timestamp")?;
			ensure!(exp > iat, "JWT expiration must follow issuance");
			ensure!(
				exp > u64::try_from(signer.now.div_euclid(1_000))?,
				"JWT expiration must be in the future"
			);
			let jti = TokenId::generate(&mut OsRng).to_string();
			let claims = Claims {
				rivet_ver: CLAIMS_VERSION,
				iss: signer.issuer.to_owned(),
				aud: signer.audience.to_owned(),
				sub: input.subject.clone(),
				iat,
				exp,
				jti: jti.clone(),
				rivet_ns: input.namespace_id.to_string(),
				rivet_grants: encoded_grants,
			};
			let kid = signer.key.kid().to_string();
			let token = encode(signer.key, &claims).context("failed to encode JWT")?;
			let expires_ts = i64::try_from(exp)
				.context("JWT expiration does not fit in milliseconds")?
				.checked_mul(1_000)
				.context("JWT expiration overflow")?;

			Ok(IssueResponse {
				token,
				issued_ts,
				expires_ts,
				kid,
				jti,
			})
		})
		.await
		.context("failed to use active JWT signer")
}

fn bounded_expiration(
	issued_ts: i64,
	requested_expires_ts: i64,
	max_duration_ms: i64,
) -> Result<i64> {
	ensure!(
		requested_expires_ts > issued_ts,
		"JWT expiration must follow issuance"
	);
	let maximum_expires_ts = issued_ts
		.checked_add(max_duration_ms)
		.context("JWT expiration overflow")?;
	Ok(requested_expires_ts.min(maximum_expires_ts))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn signer_clamps_expiration_to_its_own_maximum() {
		assert_eq!(bounded_expiration(1_000, 12_000, 10_000).unwrap(), 11_000);
		assert_eq!(bounded_expiration(1_000, 9_000, 10_000).unwrap(), 9_000);
		assert!(bounded_expiration(1_000, 1_000, 10_000).is_err());
	}
}
