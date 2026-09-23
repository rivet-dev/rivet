use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicUsize, Ordering},
	},
	time::Instant,
};

use tokio::sync::{Mutex, Notify, watch};

use super::*;
use crate::key_ring_cache::{snapshot::FetchedSnapshot, validation::validate_lifecycle};
use crate::{
	Claims, Id, RotationPolicy, SigningKeyRecord, SigningKeyRing, TokenId, ValidatedGrantSet,
};

fn policy() -> RotationPolicy {
	RotationPolicy {
		rotation_interval_ms: 60_000,
		publish_lead_ms: 1_000,
		max_signing_lifetime_ms: 120_000,
		max_token_ttl_ms: 86_400_000,
		clock_skew_ms: 30_000,
	}
}

fn ring(now: i64) -> SigningKeyRing {
	crate::bootstrap(
		"https://api.rivet.dev".into(),
		1,
		"rivet-api".into(),
		1,
		1,
		&SigningKeyRecord::generate(now),
		now,
		policy(),
	)
	.unwrap()
	.successor
}

fn cached(ring: SigningKeyRing, mode: KeyRingCacheMode) -> CachedSnapshot {
	let fetched =
		FetchedSnapshot::from_authoritative(ring, mode, rivet_util::timestamp::now()).unwrap();
	let mut cached = CachedSnapshot::from_wire(fetched.verification).unwrap();
	cached.active_signer = fetched.active_signer;
	cached.fingerprint = fetched.fingerprint;
	cached
}

#[derive(Clone)]
struct Source {
	ring: Arc<Mutex<Option<SigningKeyRing>>>,
	reads: Arc<AtomicUsize>,
	blocked: Arc<AtomicBool>,
	local: Arc<AtomicBool>,
	notify: Arc<Notify>,
}

impl Source {
	fn new(ring: SigningKeyRing) -> Self {
		Self {
			ring: Arc::new(Mutex::new(Some(ring))),
			reads: Arc::new(AtomicUsize::new(0)),
			blocked: Arc::new(AtomicBool::new(false)),
			local: Arc::new(AtomicBool::new(false)),
			notify: Arc::new(Notify::new()),
		}
	}

	fn cache(&self, mode: KeyRingCacheMode) -> Arc<KeyRingCache> {
		let source = self.clone();
		let (snapshot_tx, _) = watch::channel(None);
		Arc::new(KeyRingCache {
			mode,
			fetch_snapshot: Arc::new(move || {
				let source = source.clone();
				Box::pin(async move {
					source.reads.fetch_add(1, Ordering::SeqCst);
					while source.blocked.load(Ordering::SeqCst) {
						source.notify.notified().await;
					}
					let ring = source
						.ring
						.lock()
						.await
						.clone()
						.context("replicas unavailable")?;
					let local = source.local.load(Ordering::Acquire);
					let mut snapshot = FetchedSnapshot::from_authoritative(
						ring,
						if local {
							KeyRingCacheMode::VerifyOnly
						} else {
							mode
						},
						rivet_util::timestamp::now(),
					)?;
					if local {
						snapshot.path = crate::key_ring_cache::snapshot::ReadPath::LocalFallback;
					}
					Ok(snapshot)
				})
			}),
			cache_ttl: Duration::from_secs(60),
			max_stale: Duration::from_secs(300),
			unknown_kid_cooldown: Duration::from_secs(1),
			snapshot_tx,
			refresh_lock: Mutex::new(()),
			background_refresh_running: AtomicBool::new(false),
			last_unknown_refresh_ts: AtomicI64::new(i64::MIN),
			last_refresh_failure_ts: AtomicI64::new(i64::MIN),
			fallback_state: AtomicU8::new(0),
		})
	}

	async fn wait_for_read(&self) {
		tokio::time::timeout(Duration::from_secs(1), async {
			while self.reads.load(Ordering::SeqCst) == 0 {
				tokio::task::yield_now().await;
			}
		})
		.await
		.unwrap();
	}
}

fn claims(signer: &ActiveSigner<'_>) -> Claims {
	let namespace = Id::nil();
	let grants = ValidatedGrantSet::new(
		namespace,
		[rivet_auth_policy::OwnedGrant {
			namespace: rivet_auth_policy::Scope::Id(namespace),
			resource: rivet_auth_policy::ResourceKind::Actor,
			target: rivet_auth_policy::Scope::Any,
			operations: vec![rivet_auth_policy::OperationKind::Read],
		}],
	)
	.unwrap();
	Claims {
		rivet_ver: crate::CLAIMS_VERSION,
		iss: signer.issuer.to_owned(),
		aud: signer.audience.to_owned(),
		sub: None,
		iat: (signer.issued_ts / 1000) as u64,
		exp: (signer.issued_ts / 1000) as u64 + crate::PROTOCOL_MAX_TTL,
		jti: TokenId::from_bytes([1; 16]).to_string(),
		rivet_ns: namespace.to_string(),
		rivet_grants: crate::encode_grants(&grants).unwrap(),
	}
}

async fn mint(cache: &KeyRingCache) -> Result<String> {
	cache
		.with_active_signer(|signer| Ok(crate::encode(signer.key, &claims(&signer))?))
		.await
}

#[test]
fn signing_lease_is_strict_and_checks_both_clocks() {
	let mut snapshot = cached(ring(100_000), KeyRingCacheMode::SignAndVerify);
	snapshot.read_started_ts = 100_000;
	assert!(snapshot.active_signer(100_000).is_ok());
	assert!(snapshot.active_signer(104_999).is_ok());
	assert!(snapshot.active_signer(105_000).is_err());
	assert!(snapshot.active_signer(99_999).is_err());
	snapshot.read_started_at = Instant::now() - SIGNING_LEASE;
	assert!(snapshot.active_signer(100_000).is_err());
}

#[test]
fn hard_signing_deadline_uses_actual_time_without_skew() {
	let mut ring = ring(100_000);
	ring.active.sign_until_ts = 104_000;
	let mut snapshot = cached(ring, KeyRingCacheMode::SignAndVerify);
	snapshot.read_started_ts = 100_000;
	assert!(snapshot.active_signer(103_999).is_ok());
	assert!(snapshot.active_signer(104_000).is_err());
}

#[tokio::test]
async fn verification_only_never_retains_a_private_signer() {
	let now = rivet_util::timestamp::now();
	let ring = crate::stage_pending_forced(
		&ring(now - 10_000),
		&SigningKeyRecord::generate(now),
		now,
		policy(),
	)
	.unwrap()
	.successor;
	let source = Source::new(ring);
	let cache = source.cache(KeyRingCacheMode::VerifyOnly);
	cache.start().await.unwrap();
	let snapshot = cache.current_snapshot().unwrap();
	assert_eq!(snapshot.keys.len(), 2);
	assert!(snapshot.active_signer.is_none());
	assert!(mint(&cache).await.is_err());
	assert_eq!(source.reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rotation_advances_signer_and_verifier_together_and_releases_old_snapshot() {
	let now = rivet_util::timestamp::now();
	let initial = ring(now - 10_000);
	let source = Source::new(initial.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	cache.start().await.unwrap();
	let first = mint(&cache).await.unwrap();
	assert_eq!(
		cache.verify(&first).await.unwrap().kid,
		initial.active.key.kid
	);
	let old_snapshot = Arc::downgrade(&cache.current_snapshot().unwrap());

	let staged = crate::stage_pending_forced(
		&initial,
		&SigningKeyRecord::generate(now - 2_000),
		now - 2_000,
		policy(),
	)
	.unwrap()
	.successor;
	let rotated = crate::activate_pending(&staged, now, policy())
		.unwrap()
		.successor;
	*source.ring.lock().await = Some(rotated.clone());
	cache.refresh(true).await.unwrap();
	// The cache owns the only surviving private snapshot. Its drop zeroizes SigningKey's seed.
	assert!(old_snapshot.upgrade().is_none());
	let second = mint(&cache).await.unwrap();
	let verified = cache.verify(&second).await.unwrap();
	assert_eq!(verified.kid, rotated.active.key.kid);
	assert_eq!(
		cache.current_snapshot().unwrap().generation,
		rotated.generation
	);
	assert!(cache.verify(&first).await.is_ok());
}

#[tokio::test]
async fn expired_signing_lease_fails_while_verification_remains_available() {
	let now = rivet_util::timestamp::now();
	let ring = ring(now - 10_000);
	let source = Source::new(ring.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let token = mint(&cache).await.unwrap();
	let mut aged = cached(ring, KeyRingCacheMode::SignAndVerify);
	aged.read_started_at = Instant::now() - SIGNING_LEASE;
	cache.snapshot_tx.send_replace(Some(Arc::new(aged)));
	*source.ring.lock().await = None;
	assert!(mint(&cache).await.is_err());
	assert!(cache.verify(&token).await.is_ok());
	// Failed refreshes back off even though the verification cache is still fresh.
	assert!(mint(&cache).await.is_err());
	assert_eq!(source.reads.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn unexpired_signing_lease_can_be_used_after_a_failed_refresh() {
	let source = Source::new(ring(rivet_util::timestamp::now() - 10_000));
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	cache.start().await.unwrap();
	*source.ring.lock().await = None;
	assert!(cache.refresh(true).await.is_err());
	assert!(mint(&cache).await.is_ok());
}

#[tokio::test]
async fn concurrent_issuance_and_verification_share_startup_refresh() {
	let source = Source::new(ring(rivet_util::timestamp::now() - 10_000));
	source.blocked.store(true, Ordering::SeqCst);
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let left = tokio::spawn({
		let cache = cache.clone();
		async move { mint(&cache).await }
	});
	let right = tokio::spawn({
		let cache = cache.clone();
		async move { mint(&cache).await }
	});
	let verifier = tokio::spawn({
		let cache = cache.clone();
		async move { cache.usable_snapshot().await }
	});
	source.wait_for_read().await;
	source.blocked.store(false, Ordering::SeqCst);
	source.notify.notify_waiters();
	assert!(left.await.unwrap().is_ok());
	assert!(right.await.unwrap().is_ok());
	assert!(verifier.await.unwrap().is_ok());
	assert_eq!(source.reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn generation_rollback_cannot_renew_a_signing_lease() {
	let now = rivet_util::timestamp::now();
	let old = ring(now - 10_000);
	let mut current = old.clone();
	current.generation = 2;
	let source = Source::new(old);
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let mut aged = cached(current, KeyRingCacheMode::SignAndVerify);
	aged.read_started_at = Instant::now() - SIGNING_LEASE;
	cache.snapshot_tx.send_replace(Some(Arc::new(aged)));
	assert!(mint(&cache).await.is_err());
	let retained = cache.current_snapshot().unwrap();
	assert_eq!(retained.generation, 2);
	assert!(retained.read_started_at.elapsed() >= SIGNING_LEASE);
}

#[test]
fn retirement_budgets_cached_signing_and_full_clock_skew_separately() {
	// Exercise whole-second rounding with the issuer already the full 30 seconds ahead.
	for retired_ts in [100_000, 100_001, 100_999] {
		let initial = ring(60_000);
		let staged = crate::stage_pending_forced(
			&initial,
			&SigningKeyRecord::generate(98_000),
			98_000,
			policy(),
		)
		.unwrap()
		.successor;
		let rotated = crate::activate_pending(&staged, retired_ts, policy())
			.unwrap()
			.successor;
		let retirement = &rotated.retiring[0];
		let lifecycle = crate::key_ring_cache::VerificationKeyLifecycle::Retiring {
			retired_ts,
			max_token_exp_ts: retirement.max_token_exp_ts,
			verify_until_ts: retirement.verify_until_ts,
		};
		let mut snapshot = cached(initial, KeyRingCacheMode::SignAndVerify);
		snapshot.read_started_ts = retired_ts + 30_000;
		let now = snapshot.read_started_ts + 4_999;
		let signer = snapshot.active_signer(now).unwrap();
		assert_eq!(signer.issued_ts, now / 1_000 * 1_000);
		let token = crate::encode(signer.key, &claims(&signer)).unwrap();
		let decoded = crate::decode(
			&token,
			&signer.key.verification_key(),
			crate::DecodeOptions {
				issuer: signer.issuer,
				audience: signer.audience,
				now: ((now - 30_000) / 1_000) as u64,
			},
		)
		.unwrap();
		assert!(validate_lifecycle(&decoded, &lifecycle, ((now - 30_000) / 1_000) as u64).is_ok());
		let mut too_late = decoded.clone();
		too_late.claims.iat = (retired_ts / 1_000) as u64 + 36;
		assert!(validate_lifecycle(&too_late, &lifecycle, 105).is_err());
		let mut too_long = decoded.clone();
		too_long.claims.exp = (retirement.max_token_exp_ts / 1_000) as u64 + 1;
		assert!(validate_lifecycle(&too_long, &lifecycle, 105).is_err());
		assert!(
			validate_lifecycle(
				&decoded,
				&lifecycle,
				(retirement.verify_until_ts / 1_000) as u64
			)
			.is_err()
		);
	}
}

#[tokio::test]
async fn a_slow_successful_read_does_not_start_a_new_signing_lease_on_completion() {
	let source = Source::new(ring(rivet_util::timestamp::now() - 10_000));
	source.blocked.store(true, Ordering::SeqCst);
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let signing = tokio::spawn({
		let cache = cache.clone();
		async move { mint(&cache).await }
	});
	source.wait_for_read().await;
	tokio::time::sleep(SIGNING_LEASE).await;
	source.blocked.store(false, Ordering::SeqCst);
	source.notify.notify_waiters();
	assert!(signing.await.unwrap().is_err());
	assert!(cache.usable_snapshot().await.is_ok());
}

#[test]
fn rotation_during_read_with_clock_behind_stays_within_activation_allowance() {
	let mut snapshot = cached(ring(100_000), KeyRingCacheMode::SignAndVerify);
	snapshot.read_started_ts = 69_000;
	let signer = snapshot.active_signer(71_000).unwrap();
	assert_eq!(signer.issued_ts, 71_000);
	let decoded = crate::DecodedToken {
		claims: claims(&signer),
		namespace_id: Id::nil(),
		grants: vec![],
	};
	let lifecycle = &snapshot.keys.get(&signer.key.kid()).unwrap().lifecycle;
	assert!(validate_lifecycle(&decoded, lifecycle, 71).is_ok());
}

#[tokio::test]
async fn deadline_crossed_during_signing_discards_the_result() {
	let now = rivet_util::timestamp::now();
	let mut initial = ring(now - 10_000);
	initial.active.sign_until_ts = now + 100;
	let deadline = initial.active.sign_until_ts;
	let source = Source::new(initial);
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let mut entered = false;
	let result = cache
		.with_active_signer(|_| {
			entered = true;
			let remaining = deadline.saturating_sub(rivet_util::timestamp::now());
			std::thread::sleep(Duration::from_millis(remaining as u64 + 1));
			Ok(())
		})
		.await;
	assert!(entered);
	assert!(result.is_err());
}

#[tokio::test]
async fn invalid_ring_cannot_partially_replace_the_published_generation() {
	let initial = ring(rivet_util::timestamp::now() - 10_000);
	let source = Source::new(initial.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	cache.start().await.unwrap();
	let previous = cache.current_snapshot().unwrap();
	let mut invalid = initial;
	invalid.generation += 1;
	invalid.active.key.signing_key = crate::SigningKey::generate();
	*source.ring.lock().await = Some(invalid);
	assert!(cache.refresh(true).await.is_err());
	assert!(Arc::ptr_eq(&previous, &cache.current_snapshot().unwrap()));
	let token = mint(&cache).await.unwrap();
	assert!(cache.verify(&token).await.is_ok());
}

#[tokio::test]
async fn one_second_tokens_use_current_time_even_with_an_older_cached_ring() {
	let now = rivet_util::timestamp::now();
	let initial = ring(now - 10_000);
	let source = Source::new(initial.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let mut snapshot = cached(initial, KeyRingCacheMode::SignAndVerify);
	snapshot.read_started_ts = now - 2_000;
	snapshot.read_started_at = Instant::now() - Duration::from_secs(2);
	cache.snapshot_tx.send_replace(Some(Arc::new(snapshot)));
	let request = crate::issuer::IssueRequest {
		namespace_id: Id::nil(),
		grants: vec![rivet_auth_policy::OwnedGrant {
			namespace: rivet_auth_policy::Scope::Id(Id::nil()),
			resource: rivet_auth_policy::ResourceKind::Actor,
			target: rivet_auth_policy::Scope::Any,
			operations: vec![rivet_auth_policy::OperationKind::Read],
		}],
		expires_no_later_than_ts: now + 60_000,
		subject: None,
		issuer_token_id: Id::nil(),
	};
	// Run the same validation, expiration calculation and encoding as the Gas operation.
	let issued = crate::ops::issue::issue_token(&cache, &request, 1_000)
		.await
		.unwrap();
	assert!(issued.issued_ts >= now / 1_000 * 1_000);
	assert_eq!(issued.expires_ts - issued.issued_ts, 1_000);
	assert_eq!(source.reads.load(Ordering::SeqCst), 0);
	let verified = cache.verify(&issued.token).await.unwrap();
	assert_eq!(verified.token.claims.iat * 1_000, issued.issued_ts as u64);
	assert_eq!(verified.token.claims.exp * 1_000, issued.expires_ts as u64);
}

#[test]
fn issuance_timestamp_tracks_signing_time_across_second_boundaries() {
	let mut snapshot = cached(ring(60_000), KeyRingCacheMode::SignAndVerify);
	snapshot.read_started_ts = 100_123;
	for now in [100_123, 100_999, 101_000, 104_999, 105_122] {
		assert_eq!(
			snapshot.active_signer(now).unwrap().issued_ts,
			now / 1_000 * 1_000
		);
	}
}

#[tokio::test]
async fn emergency_revocation_removes_the_key_without_retirement_grace() {
	let now = rivet_util::timestamp::now();
	let initial = ring(now - 10_000);
	let source = Source::new(initial.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	let token = mint(&cache).await.unwrap();
	let revoked = crate::emergency_rotate(
		&initial,
		&SigningKeyRecord::generate(now),
		[1; 16],
		initial.generation,
		&[],
		now,
		policy(),
	)
	.unwrap()
	.successor;
	assert!(revoked.retiring.is_empty());
	*source.ring.lock().await = Some(revoked.clone());
	cache.refresh(true).await.unwrap();
	assert!(matches!(
		cache.verify(&token).await,
		Err(VerificationFailure::InvalidToken)
	));
	let replacement = mint(&cache).await.unwrap();
	assert_eq!(
		cache.verify(&replacement).await.unwrap().kid,
		revoked.active.key.kid
	);
}

#[tokio::test]
async fn local_fallback_never_renews_the_signing_lease() {
	let now = rivet_util::timestamp::now();
	let ring = ring(now);
	let source = Source::new(ring.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	cache.refresh(false).await.unwrap();
	let mut snapshot = cached(ring, KeyRingCacheMode::SignAndVerify);
	snapshot.read_started_at = Instant::now() - SIGNING_LEASE;
	snapshot.read_started_ts = now - SIGNING_LEASE.as_millis() as i64;
	let original_start = snapshot.read_started_at;
	cache.snapshot_tx.send_replace(Some(Arc::new(snapshot)));
	source.local.store(true, Ordering::Release);
	for _ in 0..3 {
		let refreshed = cache.refresh(true).await.unwrap();
		assert_eq!(refreshed.read_started_at, original_start);
		assert!(cache.with_active_signer(|_| Ok(())).await.is_err());
		assert!(cache.usable_snapshot().await.is_ok());
	}
}

#[tokio::test]
async fn local_fallback_cannot_initialize_a_signer_or_replace_its_key() {
	let now = rivet_util::timestamp::now();
	let source = Source::new(ring(now));
	source.local.store(true, Ordering::Release);
	let cold = source.cache(KeyRingCacheMode::SignAndVerify);
	assert!(cold.with_active_signer(|_| Ok(())).await.is_err());
	assert!(cold.current_snapshot().is_none());

	source.local.store(false, Ordering::Release);
	let warm = source.cache(KeyRingCacheMode::SignAndVerify);
	warm.refresh(false).await.unwrap();
	let mut replacement = ring(now);
	replacement.generation = 2;
	*source.ring.lock().await = Some(replacement);
	source.local.store(true, Ordering::Release);
	warm.refresh(true).await.unwrap();
	assert!(warm.current_snapshot().unwrap().active_signer.is_none());
	assert!(warm.with_active_signer(|_| Ok(())).await.is_err());
}

#[tokio::test]
async fn full_ring_divergence_is_rejected_even_when_public_snapshot_matches() {
	let now = rivet_util::timestamp::now();
	let original = ring(now);
	let source = Source::new(original.clone());
	let cache = source.cache(KeyRingCacheMode::SignAndVerify);
	cache.refresh(false).await.unwrap();
	let previous = cache.current_snapshot().unwrap();
	let mut changed = original;
	// Key creation time is persisted but is not part of the public verification view.
	changed.active.key.public.created_ts -= 1;
	changed.validate().unwrap();
	assert_eq!(
		crate::key_ring_cache::VerificationSnapshot::from_authoritative(&changed, now),
		previous.verification
	);
	*source.ring.lock().await = Some(changed);
	assert!(cache.refresh(true).await.is_err());
	assert!(Arc::ptr_eq(&previous, &cache.current_snapshot().unwrap()));
}
