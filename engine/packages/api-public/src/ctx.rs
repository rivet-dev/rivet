use anyhow::Result;
use rivet_auth::{AccessNamespaceScope, OperationKind, ResourceKind, TargetScope};
use std::{
	ops::Deref,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

#[derive(Clone)]
pub struct ApiCtx {
	inner: rivet_api_builder::ApiCtx,
	token: Option<String>,
	jwt_key_ring_cache: Option<Arc<rivet_auth_jwt::key_ring_cache::KeyRingCache>>,
	auth_state: rivet_auth::RequestAuthState,
	authentication_handled: Arc<AtomicBool>,
}

impl ApiCtx {
	pub fn new(
		inner: rivet_api_builder::ApiCtx,
		token: Option<String>,
		jwt_key_ring_cache: Option<Arc<rivet_auth_jwt::key_ring_cache::KeyRingCache>>,
	) -> Self {
		Self {
			inner,
			token,
			jwt_key_ring_cache,
			auth_state: rivet_auth::RequestAuthState::default(),
			authentication_handled: Arc::new(AtomicBool::new(false)),
		}
	}

	pub async fn auth(
		&self,
		namespace: AccessNamespaceScope,
		resource: ResourceKind,
		target: TargetScope,
		operation: OperationKind,
	) -> Result<()> {
		self.authentication_handled.store(true, Ordering::Relaxed);
		if self.config().insecure_allow_unauthenticated()
			&& !matches!(resource, ResourceKind::Token | ResourceKind::Jwt)
		{
			return Ok(());
		}
		let token = self
			.token
			.as_deref()
			.ok_or_else(|| rivet_auth::errors::Auth::InvalidToken.build())?;
		rivet_auth::check(
			self,
			self.jwt_key_ring_cache.as_ref(),
			&self.auth_state,
			rivet_auth::CheckInput {
				token,
				namespace,
				resource,
				target,
				operation,
			},
		)
		.await
	}

	pub async fn authenticate(&self) -> Result<Arc<rivet_auth::AuthenticatedCredential>> {
		self.authentication_handled.store(true, Ordering::Relaxed);
		let token = self
			.token
			.as_deref()
			.ok_or_else(|| rivet_auth::errors::Auth::InvalidToken.build())?;
		rivet_auth::authenticate(
			self,
			self.jwt_key_ring_cache.as_ref(),
			&self.auth_state,
			token,
		)
		.await
	}

	pub fn jwt_key_ring_cache(&self) -> Result<Arc<rivet_auth_jwt::key_ring_cache::KeyRingCache>> {
		self.jwt_key_ring_cache
			.clone()
			.ok_or_else(|| rivet_auth::errors::Auth::IssuanceUnavailable.build())
	}

	pub fn skip_auth(&self) {
		self.authentication_handled.store(true, Ordering::Relaxed);
	}

	pub fn is_auth_handled(&self) -> bool {
		self.authentication_handled.load(Ordering::Relaxed)
	}

	pub fn token(&self) -> Option<&str> {
		self.token.as_deref()
	}
}

impl Deref for ApiCtx {
	type Target = rivet_api_builder::ApiCtx;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl From<ApiCtx> for rivet_api_builder::ApiCtx {
	fn from(value: ApiCtx) -> rivet_api_builder::ApiCtx {
		value.inner
	}
}
