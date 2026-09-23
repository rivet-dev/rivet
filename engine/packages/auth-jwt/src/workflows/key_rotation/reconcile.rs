//! Reconciles the authoritative JWT key ring one transition at a time.

use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use gas::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
	IssuerReconcile, KeyId, RotationPolicy, SigningKeyRecord, SigningKeyRing, Transition,
	activate_pending, bootstrap, emergency_rotate, metrics, next_wake_ts, ops::key_ring,
	prune_retiring, reconcile_issuer, recover_leader, stage_pending, validate_emergency_request,
};

use super::{EmergencyRotate, StartRotation};

const TRANSIENT_RETRY: Duration = Duration::from_secs(5);
const BLOCKED_RETRY: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub(super) struct Input {
	pub start_rotation: Option<StartRotation>,
	pub emergency: Option<EmergencyRotate>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub(super) struct ConsumedSignals {
	pub start_rotation: bool,
	pub emergency: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum Output {
	Committed {
		next_wake_ts: i64,
		consumed: ConsumedSignals,
	},
	Stable {
		next_wake_ts: i64,
		consumed: ConsumedSignals,
	},
	Retry {
		after_ts: i64,
	},
	Blocked {
		reason: String,
		after_ts: i64,
	},
}

struct Environment {
	desired_issuer: String,
	issuer_config_generation: u64,
	accepted_issuers: Vec<String>,
	audience: String,
	leader_datacenter_id: u16,
	policy: RotationPolicy,
}

impl Environment {
	fn leader_epoch(&self, ring: Option<&SigningKeyRing>) -> Result<u64> {
		match ring {
			None => Ok(1),
			Some(ring) if ring.leader_datacenter_id == self.leader_datacenter_id => {
				Ok(ring.leader_epoch)
			}
			Some(ring) => ring
				.leader_epoch
				.checked_add(1)
				.context("JWT leader epoch overflow"),
		}
	}
}

enum Step<T> {
	Continue(T),
	Complete(Output),
}

#[derive(Default)]
struct CommitIntent {
	consumed: ConsumedSignals,
	issuer_transition: bool,
}

#[activity(ReconcileKeyRing)]
#[max_retries = usize::MAX]
pub(super) async fn reconcile_key_ring(ctx: &ActivityCtx, input: &Input) -> Result<Output> {
	let now = ctx.ts();
	let environment = match load_environment(ctx, now)? {
		Step::Continue(environment) => environment,
		Step::Complete(output) => return Ok(output),
	};
	let snapshot = match read_authoritative_ring(ctx, now).await? {
		Step::Continue(snapshot) => snapshot,
		Step::Complete(output) => return Ok(output),
	};

	if let Some(output) = reconcile_bootstrap(ctx, &environment, snapshot.as_ref(), now).await? {
		return Ok(output);
	}
	let snapshot = snapshot.context("present JWT key ring disappeared during reconciliation")?;
	let ring = &snapshot.ring;

	if let Some(output) = reconcile_issuer_state(ctx, &environment, &snapshot, now).await? {
		return Ok(output);
	}
	if let Some(output) = reconcile_leader_recovery(ctx, &environment, &snapshot, now).await? {
		return Ok(output);
	}

	let mut consumed = ConsumedSignals::default();
	if let Some(output) =
		reconcile_emergency(ctx, input, &environment, &snapshot, &mut consumed, now).await?
	{
		return Ok(output);
	}
	discard_stale_manual_rotation(input, ring, &mut consumed);

	if let Some(output) =
		reconcile_retiring_keys(ctx, &environment, &snapshot, consumed, now).await?
	{
		return Ok(output);
	}
	if let Some(output) =
		reconcile_pending_activation(ctx, &environment, &snapshot, consumed, now).await?
	{
		return Ok(output);
	}
	if let Some(output) =
		reconcile_pending_publication(ctx, input, &environment, &snapshot, consumed, now).await?
	{
		return Ok(output);
	}

	stable(ring, environment.policy, consumed, now)
}

fn load_environment(ctx: &ActivityCtx, now: i64) -> Result<Step<Environment>> {
	let jwt = &ctx.config().auth_required()?.jwt;
	if !jwt.enabled() {
		return Ok(Step::Complete(blocked("JWT is disabled", now)));
	}
	if !ctx.config().is_leader() {
		return Ok(Step::Complete(retry(now)));
	}

	let configured = (|| -> Result<Environment> {
		Ok(Environment {
			desired_issuer: rivet_config::config::auth::derive_issuer(ctx.config())?,
			issuer_config_generation: jwt.issuer_generation(),
			accepted_issuers: jwt.accepted_issuers()?,
			audience: jwt.audience().to_owned(),
			leader_datacenter_id: ctx.config().dc_label(),
			policy: rotation_policy(jwt)?,
		})
	})();

	Ok(match configured {
		Ok(environment) => Step::Continue(environment),
		Err(error) => Step::Complete(blocked(error, now)),
	})
}

async fn read_authoritative_ring(
	ctx: &ActivityCtx,
	now: i64,
) -> Result<Step<Option<key_ring::Snapshot>>> {
	Ok(match ctx.op(key_ring::get_latest::Input).await {
		Ok(key_ring::ReadOutput::Absent) => Step::Continue(None),
		Ok(key_ring::ReadOutput::Present(snapshot)) => Step::Continue(Some(*snapshot)),
		Ok(key_ring::ReadOutput::Corrupt { reason }) => Step::Complete(blocked(reason, now)),
		Err(error) => {
			tracing::warn!(?error, "failed to read authoritative JWT key ring");
			metrics::record_reconcile("retry", now);
			Step::Complete(retry(now))
		}
	})
}

async fn reconcile_bootstrap(
	ctx: &ActivityCtx,
	environment: &Environment,
	snapshot: Option<&key_ring::Snapshot>,
	now: i64,
) -> Result<Option<Output>> {
	if snapshot.is_some() {
		return Ok(None);
	}
	let candidate = SigningKeyRecord::generate(now);
	let transition = match bootstrap(
		environment.desired_issuer.clone(),
		environment.issuer_config_generation,
		environment.audience.clone(),
		environment.leader_datacenter_id,
		environment.leader_epoch(None)?,
		&candidate,
		now,
		environment.policy,
	) {
		Ok(transition) => transition,
		Err(error) => return Ok(Some(blocked(error, now))),
	};
	Ok(Some(
		commit_transition(
			ctx,
			None,
			transition,
			CommitIntent::default(),
			now,
			environment.policy,
		)
		.await?,
	))
}

async fn reconcile_leader_recovery(
	ctx: &ActivityCtx,
	environment: &Environment,
	snapshot: &key_ring::Snapshot,
	now: i64,
) -> Result<Option<Output>> {
	let ring = &snapshot.ring;
	if environment.leader_datacenter_id == ring.leader_datacenter_id {
		return Ok(None);
	}
	if environment.issuer_config_generation != ring.issuer_state.active.config_generation
		|| environment.issuer_config_generation <= ring.leader_config_generation
	{
		return Ok(Some(blocked(
			"JWT signing leadership changed without incrementing auth.jwt.issuer_generation",
			now,
		)));
	}
	let leader_epoch = environment.leader_epoch(Some(ring))?;
	let transition = match recover_leader(
		ring,
		environment.leader_datacenter_id,
		leader_epoch,
		environment.issuer_config_generation,
	) {
		Ok(transition) => transition,
		Err(error) => return Ok(Some(blocked(error, now))),
	};
	Ok(Some(
		commit_transition(
			ctx,
			Some(snapshot.encoded.clone()),
			transition,
			CommitIntent::default(),
			now,
			environment.policy,
		)
		.await?,
	))
}

async fn reconcile_issuer_state(
	ctx: &ActivityCtx,
	environment: &Environment,
	snapshot: &key_ring::Snapshot,
	now: i64,
) -> Result<Option<Output>> {
	let outcome = match reconcile_issuer(
		&snapshot.ring,
		&environment.desired_issuer,
		environment.leader_datacenter_id,
		environment.issuer_config_generation,
		&environment.accepted_issuers,
		now,
		environment.policy,
	) {
		Ok(outcome) => outcome,
		Err(error) => {
			metrics::ISSUER_TRANSITION_TOTAL
				.with_label_values(&["rejected"])
				.inc();
			return Ok(Some(blocked(error, now)));
		}
	};
	let transition = match outcome {
		IssuerReconcile::Stable => {
			return Ok(None);
		}
		IssuerReconcile::StaleConfiguration => {
			tracing::warn!(
				configured_generation = environment.issuer_config_generation,
				durable_generation = snapshot.ring.issuer_state.active.config_generation,
				"ignoring stale JWT issuer configuration"
			);
			return Ok(Some(blocked(
				"this process has stale auth.jwt.issuer_generation configuration",
				now,
			)));
		}
		IssuerReconcile::Transition(transition) => {
			if transition.successor.issuer_state.active.issuer
				== snapshot.ring.issuer_state.active.issuer
				&& transition.successor.issuer_state.retiring.len()
					< snapshot.ring.issuer_state.retiring.len()
			{
				tracing::info!(
					expired_count = snapshot.ring.issuer_state.retiring.len()
						- transition.successor.issuer_state.retiring.len(),
					"expired retiring JWT issuers"
				);
			}
			transition
		}
	};
	Ok(Some(
		commit_transition(
			ctx,
			Some(snapshot.encoded.clone()),
			transition,
			CommitIntent {
				issuer_transition: true,
				..Default::default()
			},
			now,
			environment.policy,
		)
		.await?,
	))
}

async fn reconcile_emergency(
	ctx: &ActivityCtx,
	input: &Input,
	environment: &Environment,
	snapshot: &key_ring::Snapshot,
	consumed: &mut ConsumedSignals,
	now: i64,
) -> Result<Option<Output>> {
	let Some(emergency) = &input.emergency else {
		return Ok(None);
	};
	let ring = &snapshot.ring;
	if let Some(receipt) = &ring.last_emergency_receipt
		&& receipt.request_id == emergency.request_id
	{
		consumed.emergency = true;
		metrics::record_reconcile("committed", now);
		return Ok(Some(Output::Committed {
			next_wake_ts: next_wake_ts(ring, environment.policy)?,
			consumed: *consumed,
		}));
	}
	if emergency.expected_generation != ring.generation {
		consumed.emergency = true;
		return Ok(None);
	}
	let revoke_kids = emergency
		.revoke_kids
		.iter()
		.copied()
		.map(KeyId::from_bytes)
		.collect::<Vec<_>>();
	if let Err(error) =
		validate_emergency_request(ring, emergency.expected_generation, &revoke_kids)
	{
		tracing::warn!(?error, "discarding invalid JWT emergency-rotation signal");
		consumed.emergency = true;
		return Ok(None);
	}

	let candidate = SigningKeyRecord::generate(now);
	let transition = match emergency_rotate(
		ring,
		&candidate,
		emergency.request_id,
		emergency.expected_generation,
		&revoke_kids,
		now,
		environment.policy,
	) {
		Ok(transition) => transition,
		Err(error) => {
			tracing::warn!(?error, "discarding invalid JWT emergency-rotation signal");
			consumed.emergency = true;
			return Ok(None);
		}
	};
	let mut commit_consumed = *consumed;
	commit_consumed.emergency = true;
	Ok(Some(
		commit_transition(
			ctx,
			Some(snapshot.encoded.clone()),
			transition,
			CommitIntent {
				consumed: commit_consumed,
				..Default::default()
			},
			now,
			environment.policy,
		)
		.await?,
	))
}

fn discard_stale_manual_rotation(
	input: &Input,
	ring: &SigningKeyRing,
	consumed: &mut ConsumedSignals,
) {
	if let Some(start) = &input.start_rotation
		&& (start.expected_generation != ring.generation || ring.pending.is_some())
	{
		consumed.start_rotation = true;
	}
}

async fn reconcile_retiring_keys(
	ctx: &ActivityCtx,
	environment: &Environment,
	snapshot: &key_ring::Snapshot,
	consumed: ConsumedSignals,
	now: i64,
) -> Result<Option<Output>> {
	let Some(transition) = (match prune_retiring(&snapshot.ring, now) {
		Ok(transition) => transition,
		Err(error) => return Ok(Some(blocked(error, now))),
	}) else {
		return Ok(None);
	};
	Ok(Some(
		commit_transition(
			ctx,
			Some(snapshot.encoded.clone()),
			transition,
			CommitIntent {
				consumed,
				..Default::default()
			},
			now,
			environment.policy,
		)
		.await?,
	))
}

async fn reconcile_pending_activation(
	ctx: &ActivityCtx,
	environment: &Environment,
	snapshot: &key_ring::Snapshot,
	consumed: ConsumedSignals,
	now: i64,
) -> Result<Option<Output>> {
	if !snapshot
		.ring
		.pending
		.as_ref()
		.is_some_and(|pending| pending.activate_after_ts <= now)
	{
		return Ok(None);
	}
	let transition = match activate_pending(&snapshot.ring, now, environment.policy) {
		Ok(transition) => transition,
		Err(error) => return Ok(Some(blocked(error, now))),
	};
	Ok(Some(
		commit_transition(
			ctx,
			Some(snapshot.encoded.clone()),
			transition,
			CommitIntent {
				consumed,
				..Default::default()
			},
			now,
			environment.policy,
		)
		.await?,
	))
}

async fn reconcile_pending_publication(
	ctx: &ActivityCtx,
	input: &Input,
	environment: &Environment,
	snapshot: &key_ring::Snapshot,
	consumed: ConsumedSignals,
	now: i64,
) -> Result<Option<Output>> {
	let ring = &snapshot.ring;
	let publication_due = ring
		.active
		.activated_ts
		.saturating_add(environment.policy.rotation_interval_ms)
		.saturating_sub(environment.policy.publish_lead_ms);
	let forced = input.start_rotation.is_some() && !consumed.start_rotation;
	if ring.pending.is_some() || (!forced && now < publication_due) {
		return Ok(None);
	}
	if ring.key_count() >= crate::NORMAL_KEY_LIMIT {
		return Ok(Some(retry_at(next_retiring_deadline(ring, now))));
	}

	let candidate = SigningKeyRecord::generate(now);
	let transition = match stage_pending(ring, &candidate, now, environment.policy) {
		Ok(transition) => transition,
		Err(error) => return Ok(Some(blocked(error, now))),
	};
	let mut commit_consumed = consumed;
	commit_consumed.start_rotation |= forced;
	Ok(Some(
		commit_transition(
			ctx,
			Some(snapshot.encoded.clone()),
			transition,
			CommitIntent {
				consumed: commit_consumed,
				..Default::default()
			},
			now,
			environment.policy,
		)
		.await?,
	))
}

async fn commit_transition(
	ctx: &ActivityCtx,
	expected: Option<Vec<u8>>,
	transition: Transition,
	intent: CommitIntent,
	now: i64,
	policy: RotationPolicy,
) -> Result<Output> {
	match ctx
		.op(key_ring::compare_and_set::Input {
			expected,
			successor: transition.successor,
		})
		.await
	{
		Ok(key_ring::compare_and_set::Output::Committed(snapshot)) => {
			if intent.issuer_transition {
				metrics::ISSUER_TRANSITION_TOTAL
					.with_label_values(&["committed"])
					.inc();
			}
			metrics::record_reconcile("committed", now);
			Ok(Output::Committed {
				next_wake_ts: next_wake_ts(&snapshot.ring, policy)?,
				consumed: intent.consumed,
			})
		}
		Ok(key_ring::compare_and_set::Output::Conflict) => {
			// Retain the triggering signal while rereading and replanning from the committed ring.
			metrics::record_reconcile("conflict", now);
			Ok(retry(now))
		}
		Err(error) => {
			tracing::warn!(?error, "failed to commit JWT key-ring transition");
			metrics::record_reconcile("retry", now);
			Ok(retry(now))
		}
	}
}

fn stable(
	ring: &SigningKeyRing,
	policy: RotationPolicy,
	consumed: ConsumedSignals,
	now: i64,
) -> Result<Output> {
	metrics::record_reconcile("stable", now);
	Ok(Output::Stable {
		next_wake_ts: next_wake_ts(ring, policy)?,
		consumed,
	})
}

fn retry(now: i64) -> Output {
	retry_at(now.saturating_add(TRANSIENT_RETRY.as_millis() as i64))
}

fn retry_at(after_ts: i64) -> Output {
	Output::Retry { after_ts }
}

fn blocked(reason: impl std::fmt::Display, now: i64) -> Output {
	metrics::record_reconcile("blocked", now);
	Output::Blocked {
		reason: reason.to_string(),
		after_ts: now.saturating_add(BLOCKED_RETRY.as_millis() as i64),
	}
}

fn next_retiring_deadline(ring: &SigningKeyRing, now: i64) -> i64 {
	ring.retiring
		.iter()
		.map(|retiring| retiring.verify_until_ts)
		.min()
		.unwrap_or_else(|| now.saturating_add(BLOCKED_RETRY.as_millis() as i64))
}

fn rotation_policy(jwt: &rivet_config::config::Jwt) -> Result<RotationPolicy> {
	let millis = |duration: Duration| -> Result<i64> {
		i64::try_from(duration.as_millis()).map_err(|_| anyhow!("JWT duration is too large"))
	};
	Ok(RotationPolicy {
		rotation_interval_ms: millis(jwt.key_rotation_interval())?,
		publish_lead_ms: millis(jwt.key_publish_lead())?,
		max_signing_lifetime_ms: millis(jwt.key_max_signing_lifetime())?,
		max_token_ttl_ms: i64::try_from(crate::PROTOCOL_MAX_TTL)? * 1_000,
		clock_skew_ms: i64::try_from(crate::PROTOCOL_CLOCK_SKEW)? * 1_000,
	})
}
