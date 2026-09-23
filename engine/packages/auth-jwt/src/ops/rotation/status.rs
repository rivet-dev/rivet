use anyhow::Result;
use gas::prelude::*;

use crate::{KeyId, ops::key_ring};

#[derive(Debug)]
pub struct Input;

#[derive(Debug, Clone)]
pub struct KeyStatus {
	pub kid: KeyId,
}

#[derive(Debug, Clone)]
pub struct RetiringIssuerStatus {
	pub issuer: String,
	pub accept_until_ts: i64,
}

#[derive(Debug, Clone)]
pub struct Output {
	pub generation: u64,
	pub leader_datacenter_id: u16,
	pub leader_epoch: u64,
	pub leader_config_generation: u64,
	pub desired_leader_datacenter_id: u16,
	pub desired_issuer: String,
	pub configured_issuer_generation: u64,
	pub active_issuer: String,
	pub active_issuer_generation: u64,
	pub retiring_issuers: Vec<RetiringIssuerStatus>,
	pub issuer_configuration_valid: bool,
	pub jwt_enabled: bool,
	pub issuance_enabled: bool,
	pub active: KeyStatus,
	pub active_rotation_due_ts: i64,
	pub active_hard_signing_deadline_ts: i64,
	pub pending: Option<(KeyStatus, i64)>,
	pub retiring: Vec<(KeyStatus, i64)>,
	pub last_emergency_receipt: Option<([u8; 16], u64)>,
}

#[operation]
pub async fn auth_jwt_rotation_status(
	ctx: &OperationCtx,
	_input: &Input,
) -> Result<Option<Output>> {
	let Some(snapshot) = ctx.op(key_ring::get_latest::Input).await?.optional()? else {
		return Ok(None);
	};
	let auth = ctx.config().auth_required()?;
	let rotation_ms = i64::try_from(auth.jwt.key_rotation_interval().as_millis())?;
	let desired_issuer = rivet_config::config::auth::derive_issuer(ctx.config())?;
	let desired_leader_datacenter_id = ctx.config().leader_dc()?.datacenter_label;
	let accepted_issuers = auth.jwt.accepted_issuers()?;
	let configured_issuer_generation = auth.jwt.issuer_generation();
	let now = ctx.ts();
	let key_status = |key: &crate::PublicKeyRecord| KeyStatus { kid: key.kid };
	Ok(Some(Output {
		generation: snapshot.ring.generation,
		leader_datacenter_id: snapshot.ring.leader_datacenter_id,
		leader_epoch: snapshot.ring.leader_epoch,
		leader_config_generation: snapshot.ring.leader_config_generation,
		desired_leader_datacenter_id,
		desired_issuer: desired_issuer.clone(),
		configured_issuer_generation,
		active_issuer: snapshot.ring.issuer_state.active.issuer.clone(),
		active_issuer_generation: snapshot.ring.issuer_state.active.config_generation,
		retiring_issuers: snapshot
			.ring
			.issuer_state
			.retiring
			.iter()
			.filter(|issuer| now < issuer.accept_until_ts)
			.map(|issuer| RetiringIssuerStatus {
				issuer: issuer.issuer.clone(),
				accept_until_ts: issuer.accept_until_ts,
			})
			.collect(),
		issuer_configuration_valid: (((desired_issuer
			== snapshot.ring.issuer_state.active.issuer
			&& configured_issuer_generation
				== snapshot.ring.issuer_state.active.config_generation)
			|| (desired_issuer != snapshot.ring.issuer_state.active.issuer
				&& configured_issuer_generation
					> snapshot.ring.issuer_state.active.config_generation
				&& accepted_issuers.contains(&snapshot.ring.issuer_state.active.issuer)))
			&& snapshot
				.ring
				.issuer_state
				.retiring
				.iter()
				.filter(|issuer| now < issuer.accept_until_ts)
				.all(|issuer| accepted_issuers.contains(&issuer.issuer)))
			&& if desired_leader_datacenter_id == snapshot.ring.leader_datacenter_id {
				configured_issuer_generation == snapshot.ring.leader_config_generation
			} else {
				configured_issuer_generation > snapshot.ring.leader_config_generation
			},
		jwt_enabled: auth.jwt.enabled(),
		issuance_enabled: auth.jwt.issuance_enabled(),
		active: key_status(&snapshot.ring.active.key),
		active_rotation_due_ts: snapshot
			.ring
			.active
			.activated_ts
			.saturating_add(rotation_ms),
		active_hard_signing_deadline_ts: snapshot.ring.active.sign_until_ts,
		pending: snapshot
			.ring
			.pending
			.as_ref()
			.map(|pending| (key_status(&pending.key), pending.activate_after_ts)),
		retiring: snapshot
			.ring
			.retiring
			.iter()
			.map(|retiring| (key_status(&retiring.key), retiring.verify_until_ts))
			.collect(),
		last_emergency_receipt: snapshot
			.ring
			.last_emergency_receipt
			.as_ref()
			.map(|receipt| (receipt.request_id, receipt.committed_generation)),
	}))
}
