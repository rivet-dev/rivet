//! Shared authentication and namespace-scoped grant enforcement for edge transports. Every
//! edge classifies a presented token the same way: a reserved JWT first, then the configured
//! admin token.

use anyhow::Result;
use http::{HeaderMap, header};
pub use rivet_auth_policy::{OperationKind, ResourceKind};
use rivet_error::RivetError;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// A request carried no valid credential. Same identity as the shared auth crate's error, so
/// clients see one `auth.invalid_token` whichever check refused them.
#[derive(Debug, RivetError)]
#[error("auth", "invalid_token", "Authentication token is invalid.")]
pub struct InvalidToken;

/// WebSocket subprotocol prefix that carries a token, as `rivet_token.<token>`.
pub const WS_PROTOCOL_TOKEN: &str = "rivet_token.";

/// HTTP header that carries a token.
pub const X_RIVET_TOKEN: &str = "x-rivet-token";

/// WebSocket close code for a rejected credential.
pub use crate::errors::POLICY_VIOLATION_CLOSE_CODE;

/// The token a request carries: a `rivet_token.` WebSocket subprotocol, an `x-rivet-token`
/// header or an `Authorization: Bearer` header, in that order. The caller passes it to
/// [`admit_admin_or_insecure`] unchanged.
pub fn transport_token(headers: &HeaderMap) -> Option<&str> {
	let from_protocol = headers
		.get_all(header::SEC_WEBSOCKET_PROTOCOL)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|protocols| protocols.split(','))
		.find_map(|protocol| protocol.trim().strip_prefix(WS_PROTOCOL_TOKEN));
	if from_protocol.is_some() {
		return from_protocol;
	}

	if let Some(token) = headers
		.get(X_RIVET_TOKEN)
		.and_then(|value| value.to_str().ok())
	{
		return Some(token);
	}

	headers
		.get(header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(bearer_token)
}

/// The token of an `Authorization: Bearer` value. The scheme matches case-insensitively.
fn bearer_token(value: &str) -> Option<&str> {
	let (scheme, token) = value.split_once(' ')?;
	if scheme.eq_ignore_ascii_case("bearer") {
		Some(token.trim())
	} else {
		None
	}
}

/// Whether a WebSocket subprotocol carries a token. Such a protocol is never selected in the
/// upgrade reply or forwarded to an actor, so the token is not echoed back or leaked.
pub fn is_token_protocol(protocol: &str) -> bool {
	protocol.trim().starts_with(WS_PROTOCOL_TOKEN)
}

/// Whether an `Authorization` header value is a bearer for the configured admin token. The
/// gateway drops such a header instead of forwarding the engine's credential to actor code,
/// while an actor-level bearer, such as an inspector token, still reaches the actor.
pub fn is_admin_bearer(config: &rivet_config::Config, value: &str) -> bool {
	bearer_token(value).is_some_and(|token| is_admin_token(config, token))
}

/// Compares two tokens in constant time. Comparing digests keeps the comparison constant-time
/// regardless of the tokens' lengths.
fn tokens_match(expected: &str, token: &str) -> bool {
	let expected_digest = Sha256::digest(expected.as_bytes());
	let token_digest = Sha256::digest(token.as_bytes());
	bool::from(expected_digest.ct_eq(&token_digest))
}

/// Admits a request that presented the admin token. A deployment with
/// `insecure_allow_unauthenticated` admits every request instead.
pub fn admit_admin_or_insecure(config: &rivet_config::Config, token: Option<&str>) -> Result<()> {
	if config.insecure_allow_unauthenticated() {
		return Ok(());
	}
	match token {
		Some(token) if is_admin_token(config, token) => Ok(()),
		Some(_) | None => Err(InvalidToken.build()),
	}
}

/// The HTTP reply for a request [`admit_admin_or_insecure`] refused: the canonical error body with its
/// `x-rivet-error` header, so RivetKit clients surface it as a structured error.
pub fn http_rejection(error: &anyhow::Error) -> http::Response<Vec<u8>> {
	crate::errors::http_response(error)
}

/// The WebSocket close reason for a request [`admit_admin_or_insecure`] refused: the error's `<group>.<code>`.
pub fn close_reason(error: &anyhow::Error) -> String {
	let error = RivetError::extract(error);
	format!("{}.{}", error.group(), error.code())
}

/// Valid credential with its grants retained for per-route authorization.
#[derive(Clone, Debug)]
pub enum Credential {
	/// The configured admin token was presented.
	Admin,
	/// No valid token was presented and `insecure_allow_unauthenticated` admitted the request.
	/// It reaches actor and routing endpoints like an admin would, but never management
	/// routes, so a deployment that turns the flag off is not left with credentials minted
	/// while it was on.
	Unauthenticated,
	Jwt(rivet_auth_jwt::VerifiedJwt),
}

pub use rivet_auth_policy::errors::Auth::InsufficientPermissions;

/// Shared credential classifier. Edge transports never interpret JWT claims themselves.
pub async fn authenticate(
	ctx: &rivet_engine_context::EngineContext,
	token: Option<&str>,
) -> Result<Credential> {
	let Some(token) = token else {
		return unauthenticated(ctx.config());
	};
	if is_jwt(token) {
		return ctx
			.jwt()
			.verify(token)
			.await
			.map(Credential::Jwt)
			.map_err(|error| {
				if let Some(error) = error.downcast_ref::<rivet_auth_jwt::TokenError>() {
					match error {
						rivet_auth_jwt::TokenError::Expired => {
							rivet_auth_policy::errors::Auth::TokenExpired.build()
						}
						rivet_auth_jwt::TokenError::TokenTooLarge
						| rivet_auth_jwt::TokenError::InvalidEncoding
						| rivet_auth_jwt::TokenError::InvalidHeader
						| rivet_auth_jwt::TokenError::InvalidKeyId
						| rivet_auth_jwt::TokenError::InvalidKey
						| rivet_auth_jwt::TokenError::InvalidSignature
						| rivet_auth_jwt::TokenError::InvalidClaims
						| rivet_auth_jwt::TokenError::IssuedInFuture
						| rivet_auth_jwt::TokenError::LifetimeTooLong
						| rivet_auth_jwt::TokenError::InvalidGrants => InvalidToken.build(),
					}
				} else {
					error
				}
			});
	}
	if is_admin_token(ctx.config(), token) {
		return Ok(Credential::Admin);
	}
	// An unknown token is admitted only where the edge admits everything.
	unauthenticated(ctx.config())
}

fn unauthenticated(config: &rivet_config::Config) -> Result<Credential> {
	if config.insecure_allow_unauthenticated() {
		Ok(Credential::Unauthenticated)
	} else {
		Err(InvalidToken.build())
	}
}

/// Whether `token` is the configured admin token, compared in constant time.
fn is_admin_token(config: &rivet_config::Config, token: &str) -> bool {
	let Some(auth) = config.auth.as_ref() else {
		return false;
	};
	let expected = auth.admin_token.read();
	!expected.is_empty() && tokens_match(expected, token)
}

impl Credential {
	/// Credential expiration as a Unix timestamp in seconds, if it expires.
	pub fn expires_ts(&self) -> Option<u64> {
		match self {
			Self::Admin | Self::Unauthenticated => None,
			Self::Jwt(jwt) => Some(jwt.authorization_deadline),
		}
	}

	/// Whether the credential's grants are bound to a namespace, so a route must know the
	/// namespace it touches before it can be authorized.
	pub fn is_namespace_scoped(&self) -> bool {
		!matches!(self, Self::Admin | Self::Unauthenticated)
	}

	/// Credential management and JWT issuance are reserved for presented persistent
	/// credentials. A JWT is itself delegated authority and may not widen or mint more, and
	/// the insecure admit presented nothing.
	pub fn require_management(&self) -> Result<()> {
		match self {
			Self::Admin => Ok(()),
			Self::Jwt(_) => Err(InsufficientPermissions.build()),
			Self::Unauthenticated => Err(InvalidToken.build()),
		}
	}

	/// Authorizes an operation within the namespace `namespace`.
	pub fn authorize(
		&self,
		now_unix_ms: i64,
		namespace: &str,
		resource: rivet_auth_policy::ResourceKind,
		target: Option<rivet_auth_jwt::Id>,
		operation: rivet_auth_policy::OperationKind,
	) -> Result<()> {
		self.authorize_scope(
			now_unix_ms,
			rivet_auth_policy::Scope::Id(namespace),
			resource,
			target,
			operation,
		)
	}

	/// Authorizes an operation whose namespace scope may be every namespace, as credential
	/// management is. Only a grant for every namespace satisfies such a request.
	pub fn authorize_scope(
		&self,
		now_unix_ms: i64,
		namespace: rivet_auth_policy::Scope<&str>,
		resource: rivet_auth_policy::ResourceKind,
		target: Option<rivet_auth_jwt::Id>,
		operation: rivet_auth_policy::OperationKind,
	) -> Result<()> {
		use rivet_auth_policy::{AccessRequest, Scope};
		let namespace = match namespace {
			Scope::Any => Scope::Any,
			Scope::Id(name) => Scope::Id(name.to_owned()),
		};
		let target = target.map_or(Scope::Any, Scope::Id);
		let request = AccessRequest {
			namespace: namespace.as_ref(),
			resource,
			target: target.as_ref(),
			operation,
		};
		let grants = match self {
			Self::Admin | Self::Unauthenticated => return Ok(()),
			Self::Jwt(jwt) => {
				if now_unix_ms.div_euclid(1000)
					>= i64::try_from(jwt.authorization_deadline).unwrap_or(i64::MAX)
				{
					return Err(rivet_auth_policy::errors::Auth::TokenExpired.build());
				}
				jwt.token.grants.as_slice()
			}
		};
		if rivet_auth_policy::is_authorized(
			&request,
			grants.iter().map(rivet_auth_policy::OwnedGrant::as_grant),
		) {
			Ok(())
		} else {
			Err(InsufficientPermissions.build())
		}
	}
}

pub async fn check_request(
	ctx: &rivet_engine_context::EngineContext,
	token: Option<&str>,
	namespace: &str,
	resource: rivet_auth_policy::ResourceKind,
	target: Option<rivet_auth_jwt::Id>,
	operation: rivet_auth_policy::OperationKind,
) -> Result<()> {
	authenticate(ctx, token).await?.authorize(
		ctx.env().clock().observe()?.unix_ms,
		namespace,
		resource,
		target,
		operation,
	)
}

/// A reserved Rivet JWT bearer is never forwarded to actor application code.
pub fn is_jwt(token: &str) -> bool {
	rivet_auth_jwt::is_reserved_token(token)
}

/// Credentials belonging to Engine must be removed before forwarding to actor code.
pub fn is_engine_bearer(config: &rivet_config::Config, value: &str) -> bool {
	is_admin_bearer(config, value) || bearer_token(value).is_some_and(is_jwt)
}
