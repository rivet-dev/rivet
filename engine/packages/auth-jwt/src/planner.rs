use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};

use crate::{
	ActiveIssuer, ActiveKey, EmergencyReceipt, ISSUER_HISTORY_LIMIT, IssuerHistory, IssuerState,
	KeyId, NORMAL_KEY_LIMIT, PendingKey, RetiringIssuer, RetiringKey, SigningKeyRecord,
	SigningKeyRing,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationPolicy {
	pub rotation_interval_ms: i64,
	pub publish_lead_ms: i64,
	pub max_signing_lifetime_ms: i64,
	pub max_token_ttl_ms: i64,
	pub clock_skew_ms: i64,
}

impl RotationPolicy {
	pub fn validate(&self) -> Result<()> {
		ensure!(
			self.rotation_interval_ms > 0,
			"rotation interval must be positive"
		);
		ensure!(self.publish_lead_ms > 0, "publish lead must be positive");
		ensure!(
			self.publish_lead_ms < self.rotation_interval_ms,
			"publish lead must be shorter than rotation interval"
		);
		ensure!(
			self.max_signing_lifetime_ms > self.rotation_interval_ms,
			"maximum signing lifetime must exceed normal rotation interval"
		);
		ensure!(
			self.max_token_ttl_ms > 0,
			"maximum token TTL must be positive"
		);
		ensure!(self.clock_skew_ms >= 0, "clock skew cannot be negative");
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
	pub successor: SigningKeyRing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssuerReconcile {
	Stable,
	/// This process carries an older topology snapshot than the committed issuer state.
	StaleConfiguration,
	Transition(Transition),
}

pub fn bootstrap(
	issuer: String,
	issuer_config_generation: u64,
	audience: String,
	leader_datacenter_id: u16,
	leader_epoch: u64,
	candidate: &SigningKeyRecord,
	now: i64,
	policy: RotationPolicy,
) -> Result<Transition> {
	policy.validate()?;
	validate_candidate(candidate)?;

	let successor = SigningKeyRing {
		claims_version: crate::CLAIMS_VERSION,
		issuer_state: IssuerState {
			active: ActiveIssuer {
				issuer: issuer.clone(),
				activated_ts: now,
				config_generation: issuer_config_generation,
			},
			retiring: Vec::new(),
			history: vec![IssuerHistory {
				issuer,
				activated_ts: now,
				retired_ts: None,
				config_generation: issuer_config_generation,
			}],
		},
		audience,
		leader_datacenter_id,
		leader_epoch,
		leader_config_generation: issuer_config_generation,
		generation: 1,
		active: ActiveKey {
			key: candidate.clone(),
			activated_ts: now,
			sign_until_ts: checked_add(now, policy.max_signing_lifetime_ms)?,
		},
		pending: None,
		retiring: Vec::new(),
		last_emergency_receipt: None,
	};
	successor.validate()?;
	Ok(Transition { successor })
}

pub fn stage_pending(
	ring: &SigningKeyRing,
	candidate: &SigningKeyRecord,
	now: i64,
	policy: RotationPolicy,
) -> Result<Transition> {
	stage_pending_inner(ring, candidate, now, policy, false)
}

/// Stages a manually forced normal rotation while preserving the verifier publication lead.
pub fn stage_pending_forced(
	ring: &SigningKeyRing,
	candidate: &SigningKeyRecord,
	now: i64,
	policy: RotationPolicy,
) -> Result<Transition> {
	stage_pending_inner(ring, candidate, now, policy, true)
}

fn stage_pending_inner(
	ring: &SigningKeyRing,
	candidate: &SigningKeyRecord,
	now: i64,
	policy: RotationPolicy,
	forced: bool,
) -> Result<Transition> {
	policy.validate()?;
	ring.validate()?;
	ensure!(ring.pending.is_none(), "key ring already has a pending key");
	ensure!(
		ring.key_count() < NORMAL_KEY_LIMIT,
		"normal key-ring limit reached"
	);

	let target_generation = next_generation(ring)?;
	validate_candidate(candidate)?;
	let scheduled_activation = checked_add(ring.active.activated_ts, policy.rotation_interval_ms)?;
	// Even a late rotation waits for the full public-key publication lead.
	let activate_after_ts = if forced {
		checked_add(now, policy.publish_lead_ms)?
	} else {
		scheduled_activation.max(checked_add(now, policy.publish_lead_ms)?)
	};

	let mut successor = ring.clone();
	successor.generation = target_generation;
	successor.pending = Some(PendingKey {
		key: candidate.clone(),
		activate_after_ts,
	});
	ring.validate_successor(&successor)?;
	Ok(Transition { successor })
}

pub fn activate_pending(
	ring: &SigningKeyRing,
	now: i64,
	policy: RotationPolicy,
) -> Result<Transition> {
	policy.validate()?;
	ring.validate()?;
	let pending = ring
		.pending
		.as_ref()
		.context("key ring has no pending key")?;
	ensure!(
		now >= pending.activate_after_ts,
		"pending key has not completed its publication lead"
	);

	// Budget cached signing independently of issuer clock skew.
	let max_token_exp_ts = checked_add(
		checked_add(
			checked_add(now, crate::SIGNING_CACHE_LEASE.as_millis() as i64)?,
			policy.max_token_ttl_ms,
		)?,
		policy.clock_skew_ms,
	)?;
	let verify_until_ts = checked_add(max_token_exp_ts, policy.clock_skew_ms)?;
	let mut successor = ring.clone();
	successor.generation = next_generation(ring)?;
	successor.pending = None;
	successor.active = ActiveKey {
		key: pending.key.clone(),
		activated_ts: now,
		// The hard deadline is fixed at activation; retries and forced rotation never extend it.
		sign_until_ts: checked_add(now, policy.max_signing_lifetime_ms)?,
	};
	successor.retiring.push(RetiringKey {
		key: ring.active.key.public.clone(),
		retired_ts: now,
		max_token_exp_ts,
		verify_until_ts,
	});
	ring.validate_successor(&successor)?;
	Ok(Transition { successor })
}

pub fn prune_retiring(ring: &SigningKeyRing, now: i64) -> Result<Option<Transition>> {
	ring.validate()?;
	let mut successor = ring.clone();
	successor
		.retiring
		.retain(|retiring| now < retiring.verify_until_ts);
	if successor.retiring.len() == ring.retiring.len() {
		return Ok(None);
	}
	successor.generation = next_generation(ring)?;
	ring.validate_successor(&successor)?;
	Ok(Some(Transition { successor }))
}

pub fn recover_leader(
	ring: &SigningKeyRing,
	leader_datacenter_id: u16,
	leader_epoch: u64,
	leader_config_generation: u64,
) -> Result<Transition> {
	ring.validate()?;
	ensure!(
		leader_epoch > ring.leader_epoch,
		"leader epoch must increase"
	);
	ensure!(
		leader_config_generation == ring.issuer_state.active.config_generation
			&& leader_config_generation > ring.leader_config_generation,
		"leader recovery requires a newly committed configuration generation"
	);
	let mut successor = ring.clone();
	successor.leader_datacenter_id = leader_datacenter_id;
	successor.leader_epoch = leader_epoch;
	successor.leader_config_generation = leader_config_generation;
	successor.generation = next_generation(ring)?;
	ring.validate_successor(&successor)?;
	Ok(Transition { successor })
}

pub fn emergency_rotate(
	ring: &SigningKeyRing,
	candidate: &SigningKeyRecord,
	request_id: [u8; 16],
	expected_generation: u64,
	selected_revoked_kids: &[KeyId],
	now: i64,
	policy: RotationPolicy,
) -> Result<Transition> {
	policy.validate()?;
	validate_emergency_request(ring, expected_generation, selected_revoked_kids)?;
	let target_generation = next_generation(ring)?;
	validate_candidate(candidate)?;

	let selected = selected_revoked_kids
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();

	let mut revoked_kids = vec![ring.active.key.kid];
	if let Some(pending) = &ring.pending {
		revoked_kids.push(pending.key.kid);
	}
	revoked_kids.extend(selected.iter().copied());
	revoked_kids.sort_unstable();
	revoked_kids.dedup();

	let successor = SigningKeyRing {
		claims_version: ring.claims_version,
		issuer_state: ring.issuer_state.clone(),
		audience: ring.audience.clone(),
		leader_datacenter_id: ring.leader_datacenter_id,
		leader_epoch: ring.leader_epoch,
		leader_config_generation: ring.leader_config_generation,
		generation: target_generation,
		active: ActiveKey {
			key: candidate.clone(),
			activated_ts: now,
			// Emergency activation intentionally skips publish lead, but retains the same hard limit.
			sign_until_ts: checked_add(now, policy.max_signing_lifetime_ms)?,
		},
		pending: None,
		retiring: ring
			.retiring
			.iter()
			.filter(|retiring| !selected.contains(&retiring.key.kid))
			.cloned()
			.collect(),
		last_emergency_receipt: Some(EmergencyReceipt {
			request_id,
			expected_generation,
			committed_generation: target_generation,
			revoked_kids,
		}),
	};
	ring.validate_successor(&successor)?;
	Ok(Transition { successor })
}

/// Reconciles the desired topology-derived issuer against the authoritative key ring.
///
/// A transition is permitted only when configuration explicitly carries every issuer whose
/// outstanding tokens are still valid. Expired retiring issuers are pruned independently of key
/// rotation. The monotonic configuration generation fences stale processes from reversing a
/// completed rolling migration.
pub fn reconcile_issuer(
	ring: &SigningKeyRing,
	desired_issuer: &str,
	desired_leader_datacenter_id: u16,
	issuer_config_generation: u64,
	accepted_issuers: &[String],
	now: i64,
	policy: RotationPolicy,
) -> Result<IssuerReconcile> {
	policy.validate()?;
	ring.validate()?;
	ensure!(
		issuer_config_generation > 0,
		"JWT issuer configuration generation must be positive"
	);

	// A mixed-version leader deployment may execute this activity on a process with an older
	// topology snapshot. Fence it before evaluating overlap declarations; the workflow will also
	// stop that process from mutating ordinary key state.
	if issuer_config_generation < ring.issuer_state.active.config_generation {
		return Ok(IssuerReconcile::StaleConfiguration);
	}

	for retiring in ring
		.issuer_state
		.retiring
		.iter()
		.filter(|issuer| now < issuer.accept_until_ts)
	{
		ensure!(
			accepted_issuers.contains(&retiring.issuer),
			"JWT issuer {} is still required until {}. Restore it to auth.jwt.accepted_issuers or intentionally revoke outstanding tokens.",
			retiring.issuer,
			retiring.accept_until_ts
		);
	}

	let mut successor = ring.clone();
	successor
		.issuer_state
		.retiring
		.retain(|issuer| now < issuer.accept_until_ts);

	if ring.issuer_state.active.issuer == desired_issuer {
		let generation_advanced =
			issuer_config_generation > ring.issuer_state.active.config_generation;
		if generation_advanced {
			successor.issuer_state.active.config_generation = issuer_config_generation;
			if desired_leader_datacenter_id == ring.leader_datacenter_id {
				successor.leader_config_generation = issuer_config_generation;
			}
			let active_history = successor
				.issuer_state
				.history
				.iter_mut()
				.find(|entry| entry.issuer == desired_issuer && entry.retired_ts.is_none())
				.context("active issuer is missing from issuer history")?;
			active_history.config_generation = issuer_config_generation;
		}
		if !generation_advanced && successor.issuer_state.retiring == ring.issuer_state.retiring {
			return Ok(IssuerReconcile::Stable);
		}
		successor.generation = next_generation(ring)?;
		ring.validate_successor(&successor)?;
		return Ok(IssuerReconcile::Transition(Transition { successor }));
	}

	ensure!(
		issuer_config_generation > ring.issuer_state.active.config_generation,
		"The derived JWT issuer has changed, but auth.jwt.issuer_generation was not incremented. Derived issuer: {desired_issuer}. Active issuer: {}. Increase auth.jwt.issuer_generation after preparing the issuer overlap.",
		ring.issuer_state.active.issuer
	);
	ensure!(
		accepted_issuers.contains(&ring.issuer_state.active.issuer),
		"The derived JWT issuer has changed. Derived issuer: {desired_issuer}. Active issuer: {}. Restore the previous leader public_url or temporarily add the active issuer to auth.jwt.accepted_issuers.",
		ring.issuer_state.active.issuer
	);
	let previous = ring.issuer_state.active.clone();
	// Issuers cached before migration can still mint for one signing lease. Retain the
	// old issuer through the latest token's expiration and verification clock leeway.
	let accept_until_ts = checked_add(
		checked_add(
			checked_add(
				checked_add(now, crate::SIGNING_CACHE_LEASE.as_millis() as i64)?,
				policy.max_token_ttl_ms,
			)?,
			policy.clock_skew_ms,
		)?,
		policy.clock_skew_ms,
	)?;
	for entry in &mut successor.issuer_state.history {
		if entry.issuer == previous.issuer && entry.retired_ts.is_none() {
			entry.retired_ts = Some(now);
		}
	}
	successor.issuer_state.retiring.push(RetiringIssuer {
		issuer: previous.issuer,
		retired_ts: now,
		accept_until_ts,
	});
	successor.issuer_state.active = ActiveIssuer {
		issuer: desired_issuer.to_owned(),
		activated_ts: now,
		config_generation: issuer_config_generation,
	};
	if desired_leader_datacenter_id == ring.leader_datacenter_id {
		successor.leader_config_generation = issuer_config_generation;
	}
	successor.issuer_state.history.push(IssuerHistory {
		issuer: desired_issuer.to_owned(),
		activated_ts: now,
		retired_ts: None,
		config_generation: issuer_config_generation,
	});
	if successor.issuer_state.history.len() > ISSUER_HISTORY_LIMIT {
		let remove = successor.issuer_state.history.len() - ISSUER_HISTORY_LIMIT;
		successor.issuer_state.history.drain(..remove);
	}
	successor.generation = next_generation(ring)?;
	ring.validate_successor(&successor)?;
	Ok(IssuerReconcile::Transition(Transition { successor }))
}

pub fn validate_emergency_request(
	ring: &SigningKeyRing,
	expected_generation: u64,
	selected_revoked_kids: &[KeyId],
) -> Result<()> {
	ring.validate()?;
	ensure!(
		expected_generation == ring.generation,
		"emergency request generation does not match key ring"
	);
	let ring_kids = ring
		.all_public_keys()
		.map(|key| key.kid)
		.collect::<BTreeSet<_>>();
	let selected = selected_revoked_kids
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	ensure!(
		selected.len() == selected_revoked_kids.len(),
		"emergency request contains duplicate key ids"
	);
	ensure!(
		selected.is_subset(&ring_kids),
		"emergency request contains a key id outside the authoritative ring"
	);
	Ok(())
}

pub fn next_wake_ts(ring: &SigningKeyRing, policy: RotationPolicy) -> Result<i64> {
	policy.validate()?;
	ring.validate()?;
	let lifecycle_ts = if let Some(pending) = &ring.pending {
		pending.activate_after_ts
	} else {
		checked_add(
			checked_add(ring.active.activated_ts, policy.rotation_interval_ms)?,
			-policy.publish_lead_ms,
		)?
	};
	Ok(ring
		.retiring
		.iter()
		.map(|key| key.verify_until_ts)
		.chain(
			ring.issuer_state
				.retiring
				.iter()
				.map(|issuer| issuer.accept_until_ts),
		)
		.fold(lifecycle_ts, i64::min))
}

fn next_generation(ring: &SigningKeyRing) -> Result<u64> {
	ring.generation
		.checked_add(1)
		.context("generation overflow")
}

fn validate_candidate(candidate: &SigningKeyRecord) -> Result<()> {
	candidate.validate()
}

fn checked_add(left: i64, right: i64) -> Result<i64> {
	left.checked_add(right).context("timestamp overflow")
}
