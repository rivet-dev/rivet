use std::sync::atomic::{AtomicI64, Ordering};

use rivet_metrics::{REGISTRY, prometheus::*};

use crate::key_ring_cache::VerificationSnapshot;

static ROTATION_INTERVAL_MS: AtomicI64 = AtomicI64::new(0);

lazy_static::lazy_static! {
	pub static ref READ_PATH_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"rivet_auth_jwt_key_ring_read_total", "JWT key-ring read outcomes by consistency path.", &["path"], *REGISTRY
	).unwrap();
	pub static ref AUTHORITATIVE_READ_AGE_SECONDS: Gauge = register_gauge_with_registry!(
		"rivet_auth_jwt_authoritative_read_age_seconds", "Age of the last authoritative JWT ring read, sampled on refresh and verification.", *REGISTRY
	).unwrap();
	pub static ref LAST_AUTHORITATIVE_READ_TIMESTAMP_SECONDS: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_last_authoritative_read_timestamp_seconds", "Start of the last authoritative JWT ring read; local fallback never advances this timestamp.", *REGISTRY
	).unwrap();
	pub static ref ENABLED: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_enabled",
		"Whether JWT verification is enabled on this process.",
		*REGISTRY
	).unwrap();
	pub static ref KEY_RING_GENERATION: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_key_ring_generation",
		"Latest authoritative JWT key-ring generation observed by a durable reader.",
		*REGISTRY
	).unwrap();
	pub static ref ACTIVE_KEY_ROTATION_DUE_TIMESTAMP_SECONDS: IntGauge =
		register_int_gauge_with_registry!(
			"rivet_auth_jwt_active_key_rotation_due_timestamp_seconds",
			"Unix timestamp when the active JWT signing key should normally rotate.",
			*REGISTRY
		).unwrap();
	pub static ref ACTIVE_KEY_HARD_SIGNING_DEADLINE_TIMESTAMP_SECONDS: IntGauge =
		register_int_gauge_with_registry!(
			"rivet_auth_jwt_active_key_hard_signing_deadline_timestamp_seconds",
			"Unix timestamp after which the active JWT key must not sign.",
			*REGISTRY
		).unwrap();
	pub static ref PENDING_KEY_PRESENT: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_pending_key_present",
		"Whether the JWT key ring has a pending public key.",
		*REGISTRY
	).unwrap();
	pub static ref SIGNER_AVAILABLE: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_signer_available",
		"Whether the authoritative JWT signer is currently available.",
		*REGISTRY
	).unwrap();
	pub static ref KEY_RING_BLOCKED: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_key_ring_blocked",
		"Whether this process cannot currently read or validate the authoritative JWT ring.",
		*REGISTRY
	).unwrap();
	pub static ref ISSUER_CONFIGURATION_VALID: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_issuer_configuration_valid",
		"Whether the latest authoritative JWT verification snapshot is issuer-consistent.",
		*REGISTRY
	).unwrap();
	pub static ref ACTIVE_ISSUER_AGE_SECONDS: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_active_issuer_age_seconds",
		"Age of the active durable JWT issuer in seconds.",
		*REGISTRY
	).unwrap();
	pub static ref RETIRING_ISSUER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_retiring_issuer_count",
		"Number of JWT issuers currently accepted during migration overlap.",
		*REGISTRY
	).unwrap();
	pub static ref NEAREST_ISSUER_RETIREMENT_SECONDS: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_nearest_issuer_retirement_seconds",
		"Seconds until the nearest JWT issuer retirement deadline, or -1 when none are retiring.",
		*REGISTRY
	).unwrap();
	pub static ref ISSUER_TRANSITION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"rivet_auth_jwt_issuer_transition_total",
		"JWT issuer transition outcomes.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref LAST_SUCCESSFUL_RECONCILE_TIMESTAMP_SECONDS: IntGauge =
		register_int_gauge_with_registry!(
			"rivet_auth_jwt_last_successful_reconcile_timestamp_seconds",
			"Unix timestamp of this process's latest successful authoritative JWT ring read.",
			*REGISTRY
		).unwrap();
	pub static ref RECONCILE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"rivet_auth_jwt_reconcile_total",
		"JWT key-ring reconcile outcomes.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref ACTIVE_SIGNER_CACHE_ACCESS_TOTAL: IntCounterVec =
		register_int_counter_vec_with_registry!(
			"rivet_auth_jwt_active_signer_cache_access_total",
			"JWT active signer cache accesses.",
			&["result"],
			*REGISTRY
		).unwrap();
	pub static ref ACTIVE_SIGNER_CACHE_REPLACEMENT_TOTAL: IntCounter =
		register_int_counter_with_registry!(
			"rivet_auth_jwt_active_signer_cache_replacement_total",
			"JWT active signer cache replacements after the active key changed.",
			*REGISTRY
		).unwrap();
	pub static ref VERIFICATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"rivet_auth_jwt_verification_total",
		"JWT verification outcomes.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref RETIRING_ISSUER_VERIFICATION_TOTAL: IntCounter =
		register_int_counter_with_registry!(
			"rivet_auth_jwt_retiring_issuer_verification_total",
			"JWT verifications accepted through a retiring issuer.",
			*REGISTRY
		).unwrap();
	pub static ref ISSUANCE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"rivet_auth_jwt_issuance_total",
		"JWT issuance outcomes.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref VERIFIER_REFRESH_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"rivet_auth_jwt_verifier_refresh_total",
		"JWT verification-key refresh outcomes.",
		&["mode", "result"],
		*REGISTRY
	).unwrap();
	pub static ref VERIFIER_SNAPSHOT_GENERATION: IntGauge = register_int_gauge_with_registry!(
		"rivet_auth_jwt_verifier_snapshot_generation",
		"Latest JWT key-ring generation cached by this verifier process.",
		*REGISTRY
	).unwrap();
	pub static ref VERIFIER_LAST_SUCCESSFUL_REFRESH_TIMESTAMP_SECONDS: IntGauge =
		register_int_gauge_with_registry!(
			"rivet_auth_jwt_verifier_last_successful_refresh_timestamp_seconds",
			"Unix timestamp of this process's latest successful JWT verification-key refresh.",
			*REGISTRY
		).unwrap();
}

pub fn set_rotation_interval_ms(rotation_interval_ms: i64) {
	ROTATION_INTERVAL_MS.store(rotation_interval_ms.max(0), Ordering::Relaxed);
}

pub fn record_reconcile(result: &'static str, _now: i64) {
	RECONCILE_TOTAL.with_label_values(&[result]).inc();
}

pub fn record_verifier_snapshot(snapshot: &VerificationSnapshot) {
	let now = rivet_util::timestamp::now();
	KEY_RING_GENERATION.set(saturating_i64(snapshot.generation));
	LAST_SUCCESSFUL_RECONCILE_TIMESTAMP_SECONDS.set(now / 1_000);
	KEY_RING_BLOCKED.set(0);
	ISSUER_CONFIGURATION_VALID.set(1);

	let mut active = None;
	let mut pending = false;
	let mut retiring = Vec::new();
	for key in &snapshot.keys {
		match &key.lifecycle {
			crate::key_ring_cache::VerificationKeyLifecycle::Active {
				activated_ts,
				sign_until_ts,
			} => active = Some((*activated_ts, *sign_until_ts)),
			crate::key_ring_cache::VerificationKeyLifecycle::Pending { .. } => pending = true,
			crate::key_ring_cache::VerificationKeyLifecycle::Retiring {
				verify_until_ts, ..
			} => retiring.push(*verify_until_ts),
		}
	}
	if let Some((activated_ts, sign_until_ts)) = active {
		ACTIVE_KEY_ROTATION_DUE_TIMESTAMP_SECONDS
			.set(activated_ts.saturating_add(ROTATION_INTERVAL_MS.load(Ordering::Relaxed)) / 1_000);
		ACTIVE_KEY_HARD_SIGNING_DEADLINE_TIMESTAMP_SECONDS.set(sign_until_ts / 1_000);
		SIGNER_AVAILABLE.set(i64::from(now < sign_until_ts));
		ACTIVE_ISSUER_AGE_SECONDS
			.set(now.saturating_sub(snapshot.active_issuer_activated_ts) / 1_000);
	}
	PENDING_KEY_PRESENT.set(i64::from(pending));
	RETIRING_ISSUER_COUNT.set(snapshot.retiring_issuers.len() as i64);
	NEAREST_ISSUER_RETIREMENT_SECONDS.set(
		snapshot
			.retiring_issuers
			.iter()
			.map(|issuer| issuer.accept_until_ts.saturating_sub(now) / 1_000)
			.min()
			.unwrap_or(-1),
	);
}

pub fn record_issuance(result: &'static str) {
	ISSUANCE_TOTAL.with_label_values(&[result]).inc();
}

pub fn record_verifier_refresh(mode: &'static str, result: &'static str, generation: Option<u64>) {
	VERIFIER_REFRESH_TOTAL
		.with_label_values(&[mode, result])
		.inc();
	if let Some(generation) = generation {
		VERIFIER_SNAPSHOT_GENERATION.set(saturating_i64(generation));
		VERIFIER_LAST_SUCCESSFUL_REFRESH_TIMESTAMP_SECONDS
			.set(rivet_util::timestamp::now() / 1_000);
		KEY_RING_BLOCKED.set(0);
	} else if result != "rollback" {
		KEY_RING_BLOCKED.set(1);
	}
}

fn saturating_i64(value: u64) -> i64 {
	i64::try_from(value).unwrap_or(i64::MAX)
}
