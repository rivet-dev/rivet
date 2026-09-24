use std::{
	future::Future,
	pin::Pin,
	sync::{Arc, Weak, atomic::Ordering},
	time::{Duration, Instant},
};

use anyhow::Result;
use gas::prelude::*;

use super::{
	KeyRingCache, KeyRingCacheMode, VerificationFailure,
	snapshot::{CachedSnapshot, FetchedSnapshot, ReadPath},
};
use crate::KeyId;

const REFRESH_TIMEOUT: Duration = Duration::from_secs(12);
const REFRESH_FAILURE_BACKOFF: Duration = Duration::from_secs(1);

type SnapshotFuture = Pin<Box<dyn Future<Output = Result<FetchedSnapshot>> + Send>>;
pub(super) type SnapshotFetcher = Arc<dyn Fn() -> SnapshotFuture + Send + Sync>;

fn is_v4_read_error(error: &anyhow::Error) -> bool {
	error
		.chain()
		.any(|cause| cause.to_string() == epoxy_protocol::READ_STATE_REQUIRES_V4_ERROR)
}

impl KeyRingCache {
	pub(super) async fn usable_snapshot(
		self: &Arc<Self>,
	) -> Result<Arc<CachedSnapshot>, VerificationFailure> {
		if let Some(snapshot) = self.current_snapshot() {
			self.observe_age(&snapshot);
			if snapshot.fetched_at.elapsed() < self.max_stale
				&& snapshot.refreshed_at.elapsed() <= self.cache_ttl
			{
				return Ok(snapshot);
			}
			if snapshot.fetched_at.elapsed() < self.max_stale {
				self.refresh_in_background();
				return Ok(snapshot);
			}
		}

		if self.refresh_failure_is_recent(REFRESH_FAILURE_BACKOFF) {
			return Err(VerificationFailure::VerificationUnavailable);
		}
		self.refresh(false).await
	}

	fn refresh_in_background(self: &Arc<Self>) {
		if self
			.background_refresh_running
			.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
			.is_err()
		{
			return;
		}

		let verifier = Arc::clone(self);
		tokio::spawn(async move {
			if let Err(error) = verifier.refresh(false).await {
				tracing::warn!(?error, "background JWT verification-key refresh failed");
			}
			verifier
				.background_refresh_running
				.store(false, Ordering::Release);
		});
	}

	pub(super) async fn refresh(
		&self,
		force: bool,
	) -> Result<Arc<CachedSnapshot>, VerificationFailure> {
		// One waiter performs each fetch so startup and unknown-kid bursts cannot fan out into a
		// thundering herd against Epoxy.
		let _guard = self.refresh_lock.lock().await;
		self.refresh_locked(force).await
	}

	pub(super) async fn refresh_locked(
		&self,
		force: bool,
	) -> Result<Arc<CachedSnapshot>, VerificationFailure> {
		if let Some(snapshot) = self.current_snapshot()
			&& snapshot.fetched_at.elapsed() < self.max_stale
			&& snapshot.refreshed_at.elapsed() <= self.cache_ttl
			&& !force
		{
			return Ok(snapshot);
		}
		// A local success means both authoritative paths just failed. Share it with queued
		// forced refreshes as well, so an issuance burst cannot repeat the outage fanout.
		if let Some(snapshot) = self.current_snapshot()
			&& snapshot.path == ReadPath::LocalFallback
			&& snapshot.refreshed_at.elapsed() < REFRESH_FAILURE_BACKOFF
			&& snapshot.fetched_at.elapsed() < self.max_stale
		{
			return Ok(snapshot);
		}
		if self.refresh_failure_is_recent(REFRESH_FAILURE_BACKOFF) {
			return Err(VerificationFailure::VerificationUnavailable);
		}

		let result = self.fetch_and_cache(force).await;
		if result.is_ok() {
			self.last_refresh_failure_ts
				.store(i64::MIN, Ordering::Release);
		} else {
			self.last_refresh_failure_ts
				.store(rivet_util::timestamp::now(), Ordering::Release);
			if let Some(snapshot) = self.current_snapshot() {
				self.observe_age(&snapshot);
				self.enter_fallback(snapshot.fetched_at.elapsed() >= self.max_stale);
			}
			crate::metrics::READ_PATH_TOTAL
				.with_label_values(&["failure"])
				.inc();
		}
		result
	}

	async fn fetch_and_cache(
		&self,
		force: bool,
	) -> Result<Arc<CachedSnapshot>, VerificationFailure> {
		let mode = if force { "forced" } else { "soft" };
		let read_started_at = Instant::now();
		let read_started_ts = rivet_util::timestamp::now();
		let fetched = tokio::time::timeout(REFRESH_TIMEOUT, (self.fetch_snapshot)())
			.await
			.map_err(|_| {
				crate::metrics::record_verifier_refresh(mode, "timeout", None);
				VerificationFailure::VerificationUnavailable
			})?
			.map_err(|error| {
				tracing::warn!("JWT key-ring read failed");
				crate::metrics::record_verifier_refresh(mode, "error", None);
				if is_v4_read_error(&error) {
					VerificationFailure::EpoxyV4Pending
				} else {
					VerificationFailure::VerificationUnavailable
				}
			})?;
		let path = fetched.path;
		let authoritative = path.authoritative();
		let previous = self.current_snapshot();
		let wire = fetched.verification;
		wire.validate().map_err(|_| {
			crate::metrics::record_verifier_refresh(mode, "invalid", None);
			VerificationFailure::VerificationUnavailable
		})?;
		if let Some(current) = &previous {
			let divergent = match (fetched.fingerprint, current.fingerprint) {
				(Some(new), Some(old)) => new != old,
				(None, None) => wire != current.verification,
				_ => true,
			};
			if wire.generation < current.generation
				|| (wire.generation == current.generation && divergent)
			{
				tracing::error!(
					cached_generation = current.generation,
					fetched_generation = wire.generation,
					"refusing inconsistent JWT key-ring refresh"
				);
				crate::metrics::record_verifier_refresh(mode, "inconsistent", None);
				return Err(VerificationFailure::VerificationUnavailable);
			}
		}
		if !authoritative
			&& previous
				.as_ref()
				.is_none_or(|current| current.fetched_at.elapsed() >= self.max_stale)
		{
			self.enter_fallback(true);
			return Err(VerificationFailure::VerificationUnavailable);
		}
		if authoritative {
			crate::metrics::record_verifier_snapshot(&wire);
		}
		let mut snapshot = CachedSnapshot::from_wire(wire)
			.map_err(|_| VerificationFailure::VerificationUnavailable)?;
		snapshot.path = path;
		snapshot.fingerprint = fetched.fingerprint;
		if authoritative {
			// Both leases include the entire read duration, not just time since completion.
			snapshot.fetched_at = read_started_at;
			snapshot.read_started_at = read_started_at;
			snapshot.read_started_ts = read_started_ts;
			snapshot.active_signer = fetched.active_signer;
			self.fallback_state.store(0, Ordering::Release);
			crate::metrics::LAST_AUTHORITATIVE_READ_TIMESTAMP_SECONDS.set(read_started_ts / 1000);
		} else {
			let previous = previous
				.as_ref()
				.expect("local fallback requires an authoritative snapshot");
			snapshot.fetched_at = previous.fetched_at;
			snapshot.read_started_at = previous.read_started_at;
			snapshot.read_started_ts = previous.read_started_ts;
			// A local update may revoke verification keys, but cannot authorize a new signer.
			// An unchanged ring can keep the previously authorized signer until its old lease ends.
			if snapshot.generation == previous.generation {
				snapshot.active_signer = previous.active_signer.clone();
			}
			self.enter_fallback(false);
		}
		self.observe_age(&snapshot);
		if snapshot.fetched_at.elapsed() >= self.max_stale {
			self.enter_fallback(true);
			return Err(VerificationFailure::VerificationUnavailable);
		}
		crate::metrics::READ_PATH_TOTAL
			.with_label_values(&[path.label()])
			.inc();
		tracing::debug!(
			read_path = path.label(),
			generation = snapshot.generation,
			authoritative_age_seconds = snapshot.fetched_at.elapsed().as_secs_f64(),
			"JWT key-ring read completed"
		);
		if let Some(previous) = self.current_snapshot()
			&& let (Some(old), Some(new)) = (&previous.active_signer, &snapshot.active_signer)
			&& old.key.public != new.key.public
		{
			crate::metrics::ACTIVE_SIGNER_CACHE_REPLACEMENT_TOTAL.inc();
		}
		let snapshot = Arc::new(snapshot);
		if !snapshot.retiring_issuers.is_empty() {
			tracing::warn!(
				retiring_issuer_count = snapshot.retiring_issuers.len(),
				nearest_retirement_ts = snapshot
					.retiring_issuers
					.iter()
					.map(|issuer| issuer.accept_until_ts)
					.min(),
				"JWT verifier is accepting tokens from retiring issuers"
			);
		}
		self.snapshot_tx.send_replace(Some(snapshot.clone()));
		crate::metrics::record_verifier_refresh(mode, "success", Some(snapshot.generation));
		Ok(snapshot)
	}

	pub(super) async fn refresh_unknown_kid(
		&self,
		kid: KeyId,
	) -> Result<Option<Arc<CachedSnapshot>>, VerificationFailure> {
		let _guard = self.refresh_lock.lock().await;
		if let Some(snapshot) = self.current_snapshot().filter(|snapshot| {
			snapshot.fetched_at.elapsed() < self.max_stale && snapshot.keys.contains_key(&kid)
		}) {
			return Ok(Some(snapshot));
		}
		if self.refresh_failure_is_recent(self.unknown_kid_cooldown) {
			return Err(VerificationFailure::VerificationUnavailable);
		}

		let now = rivet_util::timestamp::now();
		let cooldown_ms = i64::try_from(self.unknown_kid_cooldown.as_millis()).unwrap_or(i64::MAX);
		let last_success = self.last_unknown_refresh_ts.load(Ordering::Acquire);
		if now.saturating_sub(last_success) < cooldown_ms {
			return Ok(None);
		}

		// Record the attempt before awaiting the fetch. Cancellation must not let a burst of
		// unknown-kid requests bypass the cooldown by dropping this future mid-refresh.
		self.last_unknown_refresh_ts.store(now, Ordering::Release);
		let snapshot = self.refresh_locked(true).await?;
		Ok(snapshot.keys.contains_key(&kid).then_some(snapshot))
	}

	fn observe_age(&self, snapshot: &CachedSnapshot) {
		crate::metrics::AUTHORITATIVE_READ_AGE_SECONDS
			.set(snapshot.fetched_at.elapsed().as_secs_f64());
		if snapshot.fetched_at.elapsed() >= self.max_stale {
			self.enter_fallback(true);
		}
	}

	fn enter_fallback(&self, expired: bool) {
		let state = if expired { 2 } else { 1 };
		if self.fallback_state.swap(state, Ordering::AcqRel) != state {
			if expired {
				tracing::warn!("JWT verification fallback deadline reached");
			} else {
				tracing::warn!("JWT verification is using bounded local fallback");
			}
		}
	}

	fn refresh_failure_is_recent(&self, duration: Duration) -> bool {
		let now = rivet_util::timestamp::now();
		let duration_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
		now.saturating_sub(self.last_refresh_failure_ts.load(Ordering::Acquire)) < duration_ms
	}
}

pub(super) async fn periodic_refresh(verifier: Weak<KeyRingCache>, interval: Duration) {
	let mut ticker = tokio::time::interval(interval);
	ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	ticker.tick().await;
	loop {
		ticker.tick().await;
		let Some(verifier) = verifier.upgrade() else {
			break;
		};
		if let Err(error) = verifier.refresh(false).await {
			tracing::warn!(?error, "periodic JWT verification-key refresh failed");
		}
	}
}

pub(super) async fn fetch_snapshot_from_epoxy(
	ctx: &StandaloneCtx,
	mode: KeyRingCacheMode,
) -> Result<FetchedSnapshot> {
	use epoxy::ops::kv::get::{Input, ReadMode};
	let logical_key = crate::storage_keys::SigningKeyRingKey;
	let key = crate::storage_keys::subspace().pack(&logical_key);
	let owner = ctx.config().leader_dc()?.datacenter_label;
	// The fixed owner records writes locally before quorum acceptance; unresolved writes
	// disqualify its fast path. Consensus reads resolve accepted state. A local committed
	// read proves neither and therefore never renews verification or signing freshness.
	for (path, read_mode, timeout) in [
		(
			ReadPath::Owner,
			ReadMode::LocalCommitted {
				replica_id: u64::from(owner),
			},
			Duration::from_secs(2),
		),
		(
			ReadPath::Linearizable,
			ReadMode::Linearizable {
				target_replicas: None,
			},
			Duration::from_secs(7),
		),
		(
			ReadPath::LocalFallback,
			ReadMode::LocalCommitted {
				replica_id: ctx.config().epoxy_replica_id(),
			},
			Duration::from_secs(1),
		),
	] {
		let output = match tokio::time::timeout(
			timeout,
			ctx.op(Input {
				key: key.clone(),
				mode: read_mode,
			}),
		)
		.await
		{
			Ok(Ok(output)) => output,
			Ok(Err(error)) if is_v4_read_error(&error) => {
				return Err(error.into());
			}
			_ => continue,
		};
		if path == ReadPath::Owner && output.pending_write {
			continue;
		}
		let Some(committed) = output.value else {
			continue;
		};
		let snapshot = crate::ops::key_ring::decode_committed(&logical_key, committed)?;
		// Leader changes are unsupported. An epoch bump is not evidence of a safe handover.
		if snapshot.ring.leader_datacenter_id != owner || snapshot.ring.leader_epoch != 1 {
			anyhow::bail!("JWT key-ring owner does not match the fixed deployment owner");
		}
		let mut fetched = FetchedSnapshot::from_authoritative(
			snapshot.ring,
			if path.authoritative() {
				mode
			} else {
				KeyRingCacheMode::VerifyOnly
			},
			rivet_util::timestamp::now(),
		)?;
		fetched.path = path;
		return Ok(fetched);
	}
	anyhow::bail!("JWT key ring is unavailable")
}
