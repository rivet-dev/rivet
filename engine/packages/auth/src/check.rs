use std::{
	future::Future,
	sync::{Arc, RwLock},
};

use anyhow::Result;
use gas::prelude::*;
use rivet_auth_jwt::{KeyId, VerifiedJwt, key_ring_cache::KeyRingCache};
use rivet_auth_policy::{AccessRequest, EffectiveAuthority, OwnedGrant, Scope, is_authorized};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, watch};

use crate::{AccessNamespaceScope, OperationKind, ResourceKind, TargetScope, errors};

pub struct CheckInput<'a> {
	pub token: &'a str,
	pub namespace: AccessNamespaceScope,
	pub resource: ResourceKind,
	pub target: TargetScope,
	pub operation: OperationKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialKind {
	AdminToken,
	Jwt,
}

#[derive(Clone, Debug)]
pub struct JwtCredential {
	verified: VerifiedJwt,
	authority: EffectiveAuthority<Id, Id>,
}

impl JwtCredential {
	pub fn verified(&self) -> &VerifiedJwt {
		&self.verified
	}

	pub fn authority(&self) -> &EffectiveAuthority<Id, Id> {
		&self.authority
	}
}

#[derive(Clone, Debug)]
pub enum AuthenticatedCredential {
	AdminToken,
	Jwt(JwtCredential),
}

impl AuthenticatedCredential {
	pub fn kind(&self) -> CredentialKind {
		match self {
			Self::AdminToken => CredentialKind::AdminToken,
			Self::Jwt(_) => CredentialKind::Jwt,
		}
	}

	pub fn require_admin_token(&self) -> Result<()> {
		match self {
			Self::AdminToken => Ok(()),
			Self::Jwt(_) => Err(errors::Auth::InsufficientPermissions.build()),
		}
	}

	pub fn require_jwt(&self) -> Result<&JwtCredential> {
		match self {
			Self::Jwt(credential) => Ok(credential),
			Self::AdminToken => Err(errors::Auth::InvalidToken.build()),
		}
	}

	fn authorization_deadline(&self) -> Option<u64> {
		match self {
			Self::AdminToken => None,
			Self::Jwt(credential) => Some(credential.verified.authorization_deadline),
		}
	}

	fn kid(&self) -> Option<KeyId> {
		match self {
			Self::AdminToken => None,
			Self::Jwt(credential) => Some(credential.verified.kid),
		}
	}
}

#[derive(Clone)]
pub struct RequestAuthState {
	cached: Arc<Mutex<Option<CachedCredential>>>,
	current: Arc<RwLock<Option<Arc<AuthenticatedCredential>>>>,
	authorization_deadline_tx: watch::Sender<Option<u64>>,
}

impl Default for RequestAuthState {
	fn default() -> Self {
		let (authorization_deadline_tx, _) = watch::channel(None);
		Self {
			cached: Arc::default(),
			current: Arc::default(),
			authorization_deadline_tx,
		}
	}
}

struct CachedCredential {
	fingerprint: [u8; 32],
	credential: Arc<AuthenticatedCredential>,
}

pub async fn authenticate(
	ctx: &StandaloneCtx,
	jwt_key_ring_cache: Option<&Arc<KeyRingCache>>,
	request_state: &RequestAuthState,
	token: &str,
) -> Result<Arc<AuthenticatedCredential>> {
	request_state
		.credential_for_token_with(token, || {
			authenticate_uncached(ctx, jwt_key_ring_cache, token)
		})
		.await
}

pub async fn check(
	ctx: &StandaloneCtx,
	jwt_key_ring_cache: Option<&Arc<KeyRingCache>>,
	request_state: &RequestAuthState,
	input: CheckInput<'_>,
) -> Result<()> {
	let credential = authenticate(ctx, jwt_key_ring_cache, request_state, input.token).await?;
	if matches!(&*credential, AuthenticatedCredential::AdminToken) {
		return Ok(());
	}
	let jwt = credential.require_jwt()?;
	let namespace = match input.namespace {
		AccessNamespaceScope::Any => Scope::Any,
		AccessNamespaceScope::Id(namespace_id) => Scope::Id(namespace_id),
		AccessNamespaceScope::Name(name) => {
			let namespace = ctx
				.op(namespace::ops::resolve_for_name_global::Input { name })
				.await?
				.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;
			Scope::Id(namespace.namespace_id)
		}
	};
	let target = match input.target {
		TargetScope::Any => Scope::Any,
		TargetScope::Id(target_id) => Scope::Id(target_id),
	};
	let request = AccessRequest {
		namespace: namespace.as_ref(),
		resource: input.resource,
		target: target.as_ref(),
		operation: input.operation,
	};
	if !is_authorized(
		&request,
		jwt.authority().grants().iter().map(OwnedGrant::as_grant),
	) {
		return Err(errors::Auth::InsufficientPermissions.build());
	}
	Ok(())
}

async fn authenticate_uncached(
	ctx: &StandaloneCtx,
	jwt_key_ring_cache: Option<&Arc<KeyRingCache>>,
	token: &str,
) -> Result<AuthenticatedCredential> {
	let auth = ctx.config().auth_required()?;
	if bool::from(token.as_bytes().ct_eq(auth.admin_token.read().as_bytes())) {
		return Ok(AuthenticatedCredential::AdminToken);
	}
	if !rivet_auth_jwt::is_reserved_token(token) {
		return Err(errors::Auth::InvalidToken.build());
	}
	let jwt_key_ring_cache =
		jwt_key_ring_cache.ok_or_else(|| errors::Auth::InvalidToken.build())?;
	let verified = jwt_key_ring_cache
		.verify(token)
		.await
		.map_err(rivet_auth_jwt::key_ring_cache::VerificationFailure::into_error)?;
	let authority = EffectiveAuthority::new(verified.token.grants.clone());
	Ok(AuthenticatedCredential::Jwt(JwtCredential {
		verified,
		authority,
	}))
}

impl RequestAuthState {
	pub fn authorization_deadline(&self) -> Option<u64> {
		self.current
			.read()
			.expect("request auth state poisoned")
			.as_ref()
			.and_then(|credential| credential.authorization_deadline())
	}

	pub fn credential_kid(&self) -> Option<KeyId> {
		self.current
			.read()
			.expect("request auth state poisoned")
			.as_ref()
			.and_then(|credential| credential.kid())
	}

	pub fn subscribe_authorization_deadline(&self) -> watch::Receiver<Option<u64>> {
		self.authorization_deadline_tx.subscribe()
	}

	fn set_current(&self, credential: Arc<AuthenticatedCredential>) {
		let authorization_deadline = credential.authorization_deadline();
		*self.current.write().expect("request auth state poisoned") = Some(credential);
		self.authorization_deadline_tx
			.send_replace(authorization_deadline);
	}

	fn clear_current(&self) {
		*self.current.write().expect("request auth state poisoned") = None;
		self.authorization_deadline_tx.send_replace(None);
	}

	async fn credential_for_token_with<F, Fut>(
		&self,
		token: &str,
		authenticate: F,
	) -> Result<Arc<AuthenticatedCredential>>
	where
		F: FnOnce() -> Fut,
		Fut: Future<Output = Result<AuthenticatedCredential>>,
	{
		let fingerprint: [u8; 32] = Sha256::digest(token).into();
		let mut cached = self.cached.lock().await;
		if let Some(cached) = cached.as_ref()
			&& cached.fingerprint == fingerprint
		{
			self.set_current(cached.credential.clone());
			return Ok(cached.credential.clone());
		}

		self.clear_current();
		let credential = Arc::new(authenticate().await?);
		*cached = Some(CachedCredential {
			fingerprint,
			credential: credential.clone(),
		});
		self.set_current(credential.clone());
		Ok(credential)
	}
}

#[cfg(test)]
#[path = "../tests/unit/check.rs"]
mod tests;
