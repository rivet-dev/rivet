//! One published key-ring generation for runtime JWT verification and optional issuance.
//! Verification allows bounded stale reads; signing uses its own short freshness lease.

mod refresh;
mod signing;
mod snapshot;
mod validation;

use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicI64, AtomicU8},
	},
	time::Duration,
};

use anyhow::Result;
use gas::prelude::*;
use tokio::sync::{Mutex, watch};

pub use snapshot::{
	VerificationIssuerEntry, VerificationKeyEntry, VerificationKeyLifecycle, VerificationSnapshot,
};

use crate::{PROTOCOL_CLOCK_SKEW, VerifiedJwt};
use refresh::{SnapshotFetcher, fetch_snapshot_from_epoxy, periodic_refresh};
use snapshot::CachedSnapshot;
use validation::{decode_with_snapshot, map_token_error, validate_lifecycle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationFailure {
	InvalidToken,
	Expired,
	VerificationUnavailable,
}

impl std::fmt::Display for VerificationFailure {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str(match self {
			Self::InvalidToken => "authentication token is invalid",
			Self::Expired => "authentication token has expired",
			Self::VerificationUnavailable => "authentication key service is unavailable",
		})
	}
}

impl std::error::Error for VerificationFailure {}

impl VerificationFailure {
	pub fn into_error(self) -> anyhow::Error {
		match self {
			Self::InvalidToken => rivet_auth_policy::errors::Auth::InvalidToken.build(),
			Self::Expired => rivet_auth_policy::errors::Auth::TokenExpired.build(),
			Self::VerificationUnavailable => {
				rivet_auth_policy::errors::Auth::VerificationUnavailable.build()
			}
		}
	}
}

/// Private key residency is selected once by the process that owns this cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyRingCacheMode {
	VerifyOnly,
	SignAndVerify,
}

pub struct KeyRingCache {
	mode: KeyRingCacheMode,
	fetch_snapshot: SnapshotFetcher,
	cache_ttl: Duration,
	max_stale: Duration,
	unknown_kid_cooldown: Duration,
	snapshot_tx: watch::Sender<Option<Arc<CachedSnapshot>>>,
	refresh_lock: Mutex<()>,
	background_refresh_running: AtomicBool,
	last_unknown_refresh_ts: AtomicI64,
	last_refresh_failure_ts: AtomicI64,
	fallback_state: AtomicU8,
}

impl KeyRingCache {
	pub fn new(ctx: StandaloneCtx, mode: KeyRingCacheMode) -> Result<Arc<Self>> {
		let jwt = ctx.config().auth_required()?.jwt.clone();
		crate::metrics::ENABLED.set(i64::from(jwt.enabled()));
		crate::metrics::set_rotation_interval_ms(
			i64::try_from(jwt.key_rotation_interval().as_millis()).unwrap_or(i64::MAX),
		);
		let (snapshot_tx, _) = watch::channel(None);
		let fetch_ctx = ctx.clone();
		Ok(Arc::new(Self {
			mode,
			fetch_snapshot: Arc::new(move || {
				let ctx = fetch_ctx.clone();
				Box::pin(async move { fetch_snapshot_from_epoxy(&ctx, mode).await })
			}),
			cache_ttl: jwt.verifier_cache_ttl(),
			max_stale: jwt.verifier_max_stale(),
			unknown_kid_cooldown: jwt.unknown_kid_refresh_cooldown(),
			snapshot_tx,
			refresh_lock: Mutex::new(()),
			background_refresh_running: AtomicBool::new(false),
			last_unknown_refresh_ts: AtomicI64::new(i64::MIN),
			last_refresh_failure_ts: AtomicI64::new(i64::MIN),
			fallback_state: AtomicU8::new(0),
		}))
	}

	pub async fn start(self: &Arc<Self>) -> Result<()> {
		self.refresh(false)
			.await
			.map_err(VerificationFailure::into_error)?;

		let weak = Arc::downgrade(self);
		let interval = self.cache_ttl;
		tokio::spawn(async move { periodic_refresh(weak, interval).await });
		Ok(())
	}

	pub async fn verify(self: &Arc<Self>, token: &str) -> Result<VerifiedJwt, VerificationFailure> {
		let result = self.verify_inner(token).await;
		crate::metrics::VERIFICATION_TOTAL
			.with_label_values(&[match &result {
				Ok(_) => "valid",
				Err(VerificationFailure::InvalidToken) => "invalid",
				Err(VerificationFailure::Expired) => "expired",
				Err(VerificationFailure::VerificationUnavailable) => "verification_unavailable",
			}])
			.inc();
		result
	}

	async fn verify_inner(
		self: &Arc<Self>,
		token: &str,
	) -> Result<VerifiedJwt, VerificationFailure> {
		let header = crate::peek_header(token).map_err(map_token_error)?;
		let mut snapshot = self.usable_snapshot().await?;
		if !snapshot.keys.contains_key(&header.kid) {
			snapshot = self
				.refresh_unknown_kid(header.kid)
				.await?
				.ok_or(VerificationFailure::InvalidToken)?;
		}
		let cached_key = snapshot
			.keys
			.get(&header.kid)
			.ok_or(VerificationFailure::InvalidToken)?;

		let now = u64::try_from(rivet_util::timestamp::now() / 1000)
			.map_err(|_| VerificationFailure::VerificationUnavailable)?;
		let (decoded, retiring_issuer) =
			decode_with_snapshot(token, &cached_key.key, &snapshot, now)?;
		validate_lifecycle(&decoded, &cached_key.lifecycle, now)?;
		if retiring_issuer {
			crate::metrics::RETIRING_ISSUER_VERIFICATION_TOTAL.inc();
		}

		Ok(VerifiedJwt {
			authorization_deadline: decoded.claims.exp.saturating_add(PROTOCOL_CLOCK_SKEW),
			token: decoded,
			kid: header.kid,
		})
	}

	fn current_snapshot(&self) -> Option<Arc<CachedSnapshot>> {
		self.snapshot_tx.borrow().clone()
	}
}

impl std::fmt::Debug for KeyRingCache {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("KeyRingCache")
			.field("mode", &self.mode)
			.finish_non_exhaustive()
	}
}

#[cfg(test)]
#[path = "../../tests/unit/verifier.rs"]
mod tests;
