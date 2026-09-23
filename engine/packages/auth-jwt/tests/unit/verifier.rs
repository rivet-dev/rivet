use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicUsize, Ordering};

use anyhow::Context;
use rivet_auth_policy::{OperationKind, OwnedGrant, ResourceKind, Scope};
use tokio::sync::Notify;

use super::*;
use crate::{Claims, Id, KeyId, SigningKey, TokenId, ValidatedGrantSet, encode, encode_grants};

#[derive(Clone)]
struct MockSource {
	snapshot: Arc<Mutex<Option<VerificationSnapshot>>>,
	reads: Arc<AtomicUsize>,
	blocked: Arc<AtomicBool>,
	local: Arc<AtomicBool>,
	notify: Arc<Notify>,
}

impl MockSource {
	fn new(snapshot: Option<VerificationSnapshot>) -> Self {
		Self {
			snapshot: Arc::new(Mutex::new(snapshot)),
			reads: Arc::new(AtomicUsize::new(0)),
			blocked: Arc::new(AtomicBool::new(false)),
			local: Arc::new(AtomicBool::new(false)),
			notify: Arc::new(Notify::new()),
		}
	}

	fn fetcher(&self) -> SnapshotFetcher {
		let source = self.clone();
		Arc::new(move || {
			let source = source.clone();
			Box::pin(async move {
				source.reads.fetch_add(1, Ordering::AcqRel);
				while source.blocked.load(Ordering::Acquire) {
					source.notify.notified().await;
				}
				source
					.snapshot
					.lock()
					.await
					.clone()
					.context("mock Epoxy replicas are unavailable")
					.map(|verification| super::snapshot::FetchedSnapshot {
						verification,
						active_signer: None,
						path: if source.local.load(Ordering::Acquire) {
							super::snapshot::ReadPath::LocalFallback
						} else {
							super::snapshot::ReadPath::Owner
						},
						fingerprint: None,
					})
			})
		})
	}

	async fn set(&self, snapshot: Option<VerificationSnapshot>) {
		*self.snapshot.lock().await = snapshot;
	}

	fn block(&self) {
		self.blocked.store(true, Ordering::Release);
	}

	fn unblock(&self) {
		self.blocked.store(false, Ordering::Release);
		self.notify.notify_waiters();
	}

	fn reads(&self) -> usize {
		self.reads.load(Ordering::Acquire)
	}
}

fn snapshot(generation: u64, key_bytes: &[u8]) -> VerificationSnapshot {
	VerificationSnapshot {
		claims_version: crate::CLAIMS_VERSION,
		active_issuer: "https://api.rivet.dev".into(),
		active_issuer_activated_ts: 0,
		retiring_issuers: Vec::new(),
		audience: "rivet-api".into(),
		generation,
		keys: key_bytes
			.iter()
			.map(|byte| VerificationKeyEntry {
				kid: KeyId::from_bytes([*byte; 16]).to_string(),
				public_key: vec![*byte; crate::keys::PUBLIC_KEY_BYTES],
				lifecycle: VerificationKeyLifecycle::Active {
					activated_ts: 0,
					sign_until_ts: i64::MAX,
				},
			})
			.collect(),
	}
}

fn test_verifier(
	source: &MockSource,
	cache_ttl: Duration,
	max_stale: Duration,
	unknown_kid_cooldown: Duration,
) -> Arc<KeyRingCache> {
	let (snapshot_tx, _) = watch::channel(None);
	Arc::new(KeyRingCache {
		mode: KeyRingCacheMode::VerifyOnly,
		fetch_snapshot: source.fetcher(),
		cache_ttl,
		max_stale,
		unknown_kid_cooldown,
		snapshot_tx,
		refresh_lock: Mutex::new(()),
		background_refresh_running: AtomicBool::new(false),
		last_unknown_refresh_ts: AtomicI64::new(i64::MIN),
		last_refresh_failure_ts: AtomicI64::new(i64::MIN),
		fallback_state: AtomicU8::new(0),
	})
}

fn age_snapshot(verifier: &KeyRingCache, wire: VerificationSnapshot, age: Duration) {
	let mut cached = CachedSnapshot::from_wire(wire).unwrap();
	cached.fetched_at = std::time::Instant::now() - age;
	cached.refreshed_at = cached.fetched_at;
	verifier.snapshot_tx.send_replace(Some(Arc::new(cached)));
}

async fn wait_for_reads(source: &MockSource, expected: usize) {
	let deadline = std::time::Instant::now() + Duration::from_secs(1);
	while source.reads() < expected {
		assert!(
			std::time::Instant::now() < deadline,
			"timed out waiting for mock read"
		);
		tokio::task::yield_now().await;
	}
}

async fn wait_for_generation(verifier: &KeyRingCache, expected: u64) {
	let deadline = std::time::Instant::now() + Duration::from_secs(1);
	while verifier
		.current_snapshot()
		.is_none_or(|snapshot| snapshot.generation != expected)
	{
		assert!(
			std::time::Instant::now() < deadline,
			"timed out waiting for snapshot generation"
		);
		tokio::task::yield_now().await;
	}
}

fn decoded(iat: u64, exp: u64) -> crate::DecodedToken {
	crate::DecodedToken {
		claims: Claims {
			rivet_ver: crate::CLAIMS_VERSION,
			iss: "https://api.rivet.dev".into(),
			aud: "rivet-api".into(),
			sub: None,
			iat,
			exp,
			jti: TokenId::from_bytes([1; 16]).to_string(),
			rivet_ns: Id::nil().to_string(),
			rivet_grants: String::new(),
		},
		namespace_id: Id::nil(),
		grants: Vec::new(),
	}
}

fn signed_token(signing_key: &SigningKey, issuer: &str, now: u64) -> String {
	let namespace_id = Id::nil();
	let grants = ValidatedGrantSet::new(
		namespace_id,
		[OwnedGrant {
			namespace: Scope::Id(namespace_id),
			resource: ResourceKind::Actor,
			target: Scope::Any,
			operations: vec![OperationKind::Read],
		}],
	)
	.unwrap();
	encode(
		signing_key,
		&Claims {
			rivet_ver: crate::CLAIMS_VERSION,
			iss: issuer.into(),
			aud: "rivet-api".into(),
			sub: None,
			iat: now,
			exp: now + 60,
			jti: TokenId::from_bytes([2; 16]).to_string(),
			rivet_ns: namespace_id.to_string(),
			rivet_grants: encode_grants(&grants).unwrap(),
		},
	)
	.unwrap()
}

#[test]
fn durable_snapshot_accepts_retiring_issuer_only_until_deadline() {
	let now = 1_788_138_000;
	let signing_key = SigningKey::from_seed(KeyId::from_bytes([7; 16]), [42; 32]);
	let cached = CachedSnapshot::from_wire(VerificationSnapshot {
		claims_version: crate::CLAIMS_VERSION,
		active_issuer: "https://new.example".into(),
		active_issuer_activated_ts: 0,
		retiring_issuers: vec![VerificationIssuerEntry {
			issuer: "https://old.example".into(),
			accept_until_ts: i64::try_from(now + 30).unwrap() * 1_000,
		}],
		audience: "rivet-api".into(),
		generation: 2,
		keys: vec![VerificationKeyEntry {
			kid: signing_key.kid().to_string(),
			public_key: signing_key.public_key().to_vec(),
			lifecycle: VerificationKeyLifecycle::Active {
				activated_ts: 0,
				sign_until_ts: i64::MAX,
			},
		}],
	})
	.unwrap();

	let active = signed_token(&signing_key, "https://new.example", now);
	assert!(
		!decode_with_snapshot(&active, &signing_key.verification_key(), &cached, now)
			.unwrap()
			.1
	);

	let retiring = signed_token(&signing_key, "https://old.example", now);
	assert!(
		decode_with_snapshot(&retiring, &signing_key.verification_key(), &cached, now)
			.unwrap()
			.1
	);
	assert_eq!(
		decode_with_snapshot(
			&retiring,
			&signing_key.verification_key(),
			&cached,
			now + 30,
		),
		Err(VerificationFailure::InvalidToken)
	);
}

#[test]
fn active_lifecycle_bounds_issuance_time_with_clock_skew() {
	let lifecycle = VerificationKeyLifecycle::Active {
		activated_ts: 100_000,
		sign_until_ts: 200_000,
	};

	assert_eq!(
		validate_lifecycle(&decoded(69, 100), &lifecycle, 100),
		Err(VerificationFailure::InvalidToken)
	);
	assert!(validate_lifecycle(&decoded(70, 100), &lifecycle, 100).is_ok());
	assert!(validate_lifecycle(&decoded(230, 240), &lifecycle, 100).is_ok());
	assert_eq!(
		validate_lifecycle(&decoded(231, 240), &lifecycle, 100),
		Err(VerificationFailure::InvalidToken)
	);
}

#[test]
fn pending_lifecycle_rejects_early_issuance() {
	let lifecycle = VerificationKeyLifecycle::Pending {
		activate_after_ts: 200_000,
	};

	assert_eq!(
		validate_lifecycle(&decoded(169, 220), &lifecycle, 170),
		Err(VerificationFailure::InvalidToken)
	);
	assert!(validate_lifecycle(&decoded(170, 220), &lifecycle, 170).is_ok());
}

#[tokio::test]
async fn start_requires_and_installs_an_initial_snapshot() {
	let unavailable = MockSource::new(None);
	let verifier = test_verifier(
		&unavailable,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	assert!(verifier.start().await.is_err());
	assert!(verifier.current_snapshot().is_none());

	let available = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&available,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.start().await.unwrap();
	assert_eq!(verifier.current_snapshot().unwrap().generation, 1);
}

#[tokio::test]
async fn request_before_preload_uses_a_single_flight_refresh() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	source.block();
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	let left = tokio::spawn({
		let verifier = verifier.clone();
		async move { verifier.usable_snapshot().await }
	});
	let right = tokio::spawn({
		let verifier = verifier.clone();
		async move { verifier.usable_snapshot().await }
	});
	wait_for_reads(&source, 1).await;
	assert_eq!(source.reads(), 1);
	source.unblock();
	assert_eq!(left.await.unwrap().unwrap().generation, 1);
	assert_eq!(right.await.unwrap().unwrap().generation, 1);
	assert_eq!(source.reads(), 1);
}

#[tokio::test]
async fn cache_hit_does_not_read_epoxy() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	assert_eq!(verifier.usable_snapshot().await.unwrap().generation, 1);
	assert_eq!(source.reads(), 1);
}

#[tokio::test]
async fn periodic_refresh_atomically_replaces_the_snapshot() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_millis(1),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1]), Duration::from_secs(1));
	source.set(Some(snapshot(2, &[1, 2]))).await;
	source.block();
	let refresh = tokio::spawn({
		let verifier = verifier.clone();
		async move { verifier.refresh(false).await }
	});
	wait_for_reads(&source, 2).await;
	assert_eq!(verifier.current_snapshot().unwrap().generation, 1);
	source.unblock();
	assert_eq!(refresh.await.unwrap().unwrap().generation, 2);
	assert_eq!(verifier.current_snapshot().unwrap().generation, 2);
}

#[tokio::test]
async fn refresh_rejects_snapshot_generation_rollback() {
	let source = MockSource::new(Some(snapshot(2, &[2])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();

	source.set(Some(snapshot(1, &[1]))).await;
	assert!(matches!(
		verifier.refresh(true).await,
		Err(VerificationFailure::VerificationUnavailable)
	));
	let cached = verifier.current_snapshot().unwrap();
	assert_eq!(cached.generation, 2);
	assert!(cached.keys.contains_key(&KeyId::from_bytes([2; 16])));
	assert!(!cached.keys.contains_key(&KeyId::from_bytes([1; 16])));
}

#[tokio::test]
async fn stale_requests_do_not_wait_for_periodic_refresh() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(1),
		Duration::from_secs(10),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1]), Duration::from_secs(2));
	source.set(Some(snapshot(2, &[1, 2]))).await;
	source.block();

	let (left, right) = tokio::join!(verifier.usable_snapshot(), verifier.usable_snapshot());
	assert_eq!(left.unwrap().generation, 1);
	assert_eq!(right.unwrap().generation, 1);
	wait_for_reads(&source, 2).await;
	assert_eq!(source.reads(), 2);
	source.unblock();
	wait_for_generation(&verifier, 2).await;
}

#[tokio::test]
async fn periodic_refresh_reloads_an_expired_snapshot() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_millis(1),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1]), Duration::from_secs(1));
	source.set(Some(snapshot(2, &[1, 2]))).await;
	let task = tokio::spawn(periodic_refresh(
		Arc::downgrade(&verifier),
		Duration::from_millis(1),
	));
	wait_for_reads(&source, 2).await;
	wait_for_generation(&verifier, 2).await;
	assert_eq!(verifier.current_snapshot().unwrap().generation, 2);
	task.abort();
}

#[tokio::test]
async fn concurrent_unknown_kid_requests_share_one_refresh() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(60),
	);
	verifier.refresh(false).await.unwrap();
	source.set(Some(snapshot(2, &[1, 2]))).await;
	source.block();
	let kid = KeyId::from_bytes([2; 16]);
	let left = tokio::spawn({
		let verifier = verifier.clone();
		async move { verifier.refresh_unknown_kid(kid).await }
	});
	let right = tokio::spawn({
		let verifier = verifier.clone();
		async move { verifier.refresh_unknown_kid(kid).await }
	});
	wait_for_reads(&source, 2).await;
	assert_eq!(source.reads(), 2);
	source.unblock();
	assert!(left.await.unwrap().unwrap().is_some());
	assert!(right.await.unwrap().unwrap().is_some());
	assert_eq!(source.reads(), 2);
}

#[tokio::test]
async fn unknown_kid_refreshes_once_then_obeys_cooldown() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(60),
	);
	verifier.refresh(false).await.unwrap();
	source.set(Some(snapshot(2, &[1, 2]))).await;
	let second = KeyId::from_bytes([2; 16]);
	assert!(
		verifier
			.refresh_unknown_kid(second)
			.await
			.unwrap()
			.is_some()
	);
	assert_eq!(source.reads(), 2);

	let third = KeyId::from_bytes([3; 16]);
	assert!(verifier.refresh_unknown_kid(third).await.unwrap().is_none());
	assert_eq!(source.reads(), 2);
}

#[tokio::test]
async fn cancelled_unknown_kid_refresh_still_obeys_cooldown() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(60),
	);
	verifier.refresh(false).await.unwrap();
	source.set(Some(snapshot(2, &[1, 2]))).await;
	source.block();

	let task = tokio::spawn({
		let verifier = verifier.clone();
		async move {
			verifier
				.refresh_unknown_kid(KeyId::from_bytes([2; 16]))
				.await
		}
	});
	wait_for_reads(&source, 2).await;
	task.abort();
	assert!(matches!(task.await, Err(error) if error.is_cancelled()));
	source.unblock();

	assert!(
		verifier
			.refresh_unknown_kid(KeyId::from_bytes([3; 16]))
			.await
			.unwrap()
			.is_none()
	);
	assert_eq!(source.reads(), 2);
}

#[tokio::test]
async fn refresh_preloads_pending_public_keys() {
	let source = MockSource::new(Some(VerificationSnapshot {
		claims_version: crate::CLAIMS_VERSION,
		active_issuer: "https://api.rivet.dev".into(),
		active_issuer_activated_ts: 0,
		retiring_issuers: Vec::new(),
		audience: "rivet-api".into(),
		generation: 1,
		keys: vec![
			VerificationKeyEntry {
				kid: KeyId::from_bytes([1; 16]).to_string(),
				public_key: vec![1; crate::keys::PUBLIC_KEY_BYTES],
				lifecycle: VerificationKeyLifecycle::Active {
					activated_ts: 0,
					sign_until_ts: i64::MAX,
				},
			},
			VerificationKeyEntry {
				kid: KeyId::from_bytes([2; 16]).to_string(),
				public_key: vec![2; crate::keys::PUBLIC_KEY_BYTES],
				lifecycle: VerificationKeyLifecycle::Pending {
					activate_after_ts: i64::MAX,
				},
			},
		],
	}));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	let cached = verifier.refresh(false).await.unwrap();
	assert!(cached.keys.contains_key(&KeyId::from_bytes([2; 16])));
	assert_eq!(source.reads(), 1);
}

#[tokio::test]
async fn unavailable_replicas_use_cache_within_max_stale() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(1),
		Duration::from_secs(10),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1]), Duration::from_secs(2));
	source.set(None).await;

	assert_eq!(verifier.usable_snapshot().await.unwrap().generation, 1);
	wait_for_reads(&source, 2).await;
}

#[tokio::test]
async fn unavailable_replicas_fail_closed_after_max_stale() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(1),
		Duration::from_secs(10),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1]), Duration::from_secs(20));
	source.set(None).await;

	assert!(matches!(
		verifier.usable_snapshot().await,
		Err(VerificationFailure::VerificationUnavailable)
	));
	assert_eq!(source.reads(), 2);
}

#[tokio::test]
async fn local_reads_preserve_the_original_deadline_and_apply_revocations() {
	let source = MockSource::new(Some(snapshot(1, &[1, 2])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(1),
		Duration::from_secs(10),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1, 2]), Duration::from_secs(8));
	let deadline_start = verifier.current_snapshot().unwrap().fetched_at;
	source.local.store(true, Ordering::Release);
	source.set(Some(snapshot(2, &[2]))).await;
	for _ in 0..3 {
		// Advance the retry schedule independently of the fixed authoritative deadline.
		if let Some(current) = verifier.current_snapshot() {
			let mut next = CachedSnapshot::from_wire(current.verification.clone()).unwrap();
			next.fetched_at = current.fetched_at;
			next.refreshed_at = std::time::Instant::now() - Duration::from_secs(2);
			next.path = current.path;
			verifier.snapshot_tx.send_replace(Some(Arc::new(next)));
		}
		let current = verifier.refresh(true).await.unwrap();
		assert_eq!(current.fetched_at, deadline_start);
		assert!(!current.keys.contains_key(&KeyId::from_bytes([1; 16])));
	}
	age_snapshot(&verifier, snapshot(2, &[2]), Duration::from_secs(11));
	assert!(matches!(
		verifier.refresh(true).await,
		Err(VerificationFailure::VerificationUnavailable)
	));
	assert!(matches!(
		verifier.usable_snapshot().await,
		Err(VerificationFailure::VerificationUnavailable)
	));
}

#[tokio::test]
async fn local_reads_cannot_initialize_a_cold_verifier() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	source.local.store(true, Ordering::Release);
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	assert!(matches!(
		verifier.refresh(false).await,
		Err(VerificationFailure::VerificationUnavailable)
	));
	assert!(verifier.current_snapshot().is_none());
}

#[tokio::test]
async fn divergent_same_generation_does_not_replace_or_renew_the_snapshot() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	let previous = verifier.current_snapshot().unwrap();
	source.set(Some(snapshot(1, &[2]))).await;
	assert!(matches!(
		verifier.refresh(true).await,
		Err(VerificationFailure::VerificationUnavailable)
	));
	assert!(Arc::ptr_eq(
		&previous,
		&verifier.current_snapshot().unwrap()
	));
	assert!(verifier.usable_snapshot().await.is_ok());
}

#[tokio::test]
async fn expired_authority_cannot_be_hidden_by_a_long_cache_ttl() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(600),
		Duration::from_secs(10),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	age_snapshot(&verifier, snapshot(1, &[1]), Duration::from_secs(11));
	source.local.store(true, Ordering::Release);
	assert!(matches!(
		verifier.usable_snapshot().await,
		Err(VerificationFailure::VerificationUnavailable)
	));
}

#[tokio::test]
async fn forced_refresh_bursts_share_a_recent_local_fallback() {
	let source = MockSource::new(Some(snapshot(1, &[1])));
	let verifier = test_verifier(
		&source,
		Duration::from_secs(60),
		Duration::from_secs(300),
		Duration::from_secs(1),
	);
	verifier.refresh(false).await.unwrap();
	source.local.store(true, Ordering::Release);
	let (a, b, c) = tokio::join!(
		verifier.refresh(true),
		verifier.refresh(true),
		verifier.refresh(true)
	);
	assert!(a.is_ok() && b.is_ok() && c.is_ok());
	assert_eq!(source.reads(), 2);
}
