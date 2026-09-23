use std::time::Duration;

use anyhow::{Context, Result, ensure};

use super::{KeyRingCache, KeyRingCacheMode, VerificationFailure, snapshot::CachedSnapshot};
use crate::SigningKey;

/// Independent of verification's TTL/max-stale. Includes the duration of the ring read.
const SIGNING_LEASE: Duration = crate::SIGNING_CACHE_LEASE;

pub(crate) struct ActiveSigner<'a> {
	pub(crate) key: &'a SigningKey,
	pub(crate) issuer: &'a str,
	pub(crate) audience: &'a str,
	pub(crate) issued_ts: i64,
	pub(crate) now: i64,
}

impl KeyRingCache {
	/// Borrow the current signer only for synchronous token creation. Holding the watch read
	/// guard prevents publishing a different generation halfway through issuance.
	pub(crate) async fn with_active_signer<T>(
		&self,
		sign: impl FnOnce(ActiveSigner<'_>) -> Result<T>,
	) -> Result<T> {
		ensure!(
			self.mode == KeyRingCacheMode::SignAndVerify,
			"JWT signing is disabled on this cache"
		);
		let fresh = self
			.current_snapshot()
			.is_some_and(|snapshot| snapshot.signing_lease_valid(rivet_util::timestamp::now()));
		crate::metrics::ACTIVE_SIGNER_CACHE_ACCESS_TOTAL
			.with_label_values(&[if fresh { "hit" } else { "miss" }])
			.inc();
		if !fresh {
			let _guard = self.refresh_lock.lock().await;
			// A verifier refresh or another issuer may have renewed the same cache while waiting.
			if !self
				.current_snapshot()
				.is_some_and(|snapshot| snapshot.signing_lease_valid(rivet_util::timestamp::now()))
			{
				self.refresh_locked(true)
					.await
					.map_err(VerificationFailure::into_error)?;
			}
		}

		let published = self.snapshot_tx.borrow();
		let snapshot = published.as_ref().context("JWT key ring is unavailable")?;
		let result = sign(snapshot.active_signer(rivet_util::timestamp::now())?)?;
		// Recheck after encoding: a scheduling pause must not return a token outside the lease
		// or past the hard signing deadline. Token timestamps use the actual signing time.
		snapshot.active_signer(rivet_util::timestamp::now())?;
		Ok(result)
	}
}

impl CachedSnapshot {
	fn signing_lease_valid(&self, now: i64) -> bool {
		self.active_signer.is_some()
			&& self.read_started_at.elapsed() < SIGNING_LEASE
			&& now >= self.read_started_ts
			&& now.saturating_sub(self.read_started_ts) < SIGNING_LEASE.as_millis() as i64
	}

	fn active_signer(&self, now: i64) -> Result<ActiveSigner<'_>> {
		ensure!(
			self.signing_lease_valid(now),
			"JWT signing freshness lease expired"
		);
		let active = self
			.active_signer
			.as_ref()
			.context("JWT signing is disabled on this cache")?;
		ensure!(
			now < active.sign_until_ts,
			"active JWT signing key reached its hard signing deadline"
		);

		Ok(ActiveSigner {
			key: &active.key.signing_key,
			issuer: &self.active_issuer,
			audience: &self.audience,
			issued_ts: now.div_euclid(1_000) * 1_000,
			now,
		})
	}
}

#[cfg(test)]
#[path = "../../tests/unit/signing.rs"]
mod tests;
