//! Admin-only JWT operator commands use the current singleton's existing worker-addressed UPS.
use anyhow::{Result, bail, ensure};
use clap::Subcommand;
use rivet_auth_jwt::{KeyId, rotation};
use rivet_data::generated::auth_jwt_rotation_v1::{
	EmergencyRotateRequest, EmergencyRotateResponse,
};
use rivet_engine_context::EngineContext;
use rivet_singleton_coordinator::{
	singleton_coordinator_machine::{executor::LeaseStore, types::LeaseRead},
	store::ObjectLeaseStore,
};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Show public key IDs, ring version, coordinator epoch and the last emergency receipt.
	Show,
	/// Revoke the active and pending signers immediately, plus any named retiring keys.
	/// Cached verifiers learn the revocation within five minutes. A lost reply is ambiguous;
	/// inspect `auth jwt show`, then retry with the same request ID and ring version.
	EmergencyRotate {
		/// Stable logical command ID, retained across retries after a lost reply.
		#[arg(long)]
		request_id: Uuid,
		/// Existing singleton tenure epoch printed by `auth jwt show`.
		#[arg(long)]
		expected_coordinator_epoch: u64,
		/// Exact key-ring version printed by `auth jwt show`.
		#[arg(long)]
		expected_ring_version: u64,
		/// Additional retiring key IDs to revoke, in the displayed base64url form.
		#[arg(long = "revoke-kid")]
		revoke_kids: Vec<KeyId>,
	},
}

pub(super) async fn execute(ctx: &EngineContext, command: Command) -> Result<Value> {
	ensure!(
		ctx.config().is_leader(),
		"JWT rotation is owned by the leader datacenter; run this command there"
	);
	let timeout = Duration::from_millis(ctx.config().depot().volume_request_timeout_ms());
	let clock = ctx.env().clock();
	let lease = match rivet_env::timeout(
		clock.as_ref(),
		timeout,
		ObjectLeaseStore::new(ctx.object_stores().lease.clone()).read(),
	)
	.await?
	{
		LeaseRead::Found(lease) if !lease.released => lease,
		LeaseRead::Found(_) | LeaseRead::Missing | LeaseRead::Unknown => {
			bail!("no live singleton coordinator route is available")
		}
	};
	match command {
		Command::Show => {
			let mut status = rivet_env::timeout(
				clock.as_ref(),
				timeout,
				rotation::status(ctx.object_stores().lease.as_ref()),
			)
			.await??;
			status["coordinatorEpoch"] = json!(lease.tenure.coordinator_generation);
			status["holder"] = json!(Uuid::from_bytes(*lease.tenure.owner.worker_id.as_bytes()));
			Ok(status)
		}
		Command::EmergencyRotate {
			request_id,
			expected_coordinator_epoch,
			expected_ring_version,
			revoke_kids,
		} => {
			ensure!(
				revoke_kids.len() <= rivet_auth_jwt::ABSOLUTE_KEY_LIMIT,
				"too many key IDs to revoke"
			);
			let request = EmergencyRotateRequest {
				request_id: *request_id.as_bytes(),
				expected_coordinator_epoch,
				expected_ring_version,
				revoke_kids: revoke_kids.into_iter().map(|kid| *kid.as_bytes()).collect(),
			};
			let response = rotation::request(
				&ctx.ups()?,
				ctx.topology().as_ref(),
				&lease.tenure,
				request,
				ctx.config().auth_required()?.admin_token.read(),
				timeout,
			)
			.await?;
			match response {
				EmergencyRotateResponse::Committed(committed) => Ok(
					json!({"requestId": request_id, "committedRingVersion": committed.ring_version, "revokedKids": committed.revoked_kids.into_iter().map(|kid| KeyId::from_bytes(kid).to_string()).collect::<Vec<_>>()}),
				),
				EmergencyRotateResponse::Rejected(reason) => bail!(
					"JWT emergency rotation rejected: {reason:?}; inspect `auth jwt show` before retrying; reuse the request ID and body after an unknown outcome"
				),
			}
		}
	}
}
