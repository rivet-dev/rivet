//! Operator request/reply over existing worker-addressed UPS. Only the singleton core mutates
//! the ring. A lost reply is an unknown outcome; replay the same request ID and immutable body.
use crate::{
	key_ring_machine::{
		event::Event,
		types::{EmergencyRotateRequest, EmergencyRotateResponse, Rejected},
	},
	keys::ups::JwtRotationSubject,
};
use anyhow::{Context, Result, ensure};
use parking_lot::Mutex;
use rivet_config::Config;
use rivet_data::generated::auth_jwt_rotation_v1::OperatorRequest;
use rivet_data::versioned::{
	AuthJwtEmergencyRotateRequestData, AuthJwtEmergencyRotateResponseData,
};
use rivet_singleton_coordinator::{
	authority::Authority, singleton_coordinator_machine::types::CoordinatorTenure,
};
use rivet_topology::Topology;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::sync::{mpsc, oneshot};
use universalpubsub::{PubSub, RequestClass, SubjectCapacities};
use vbare::Versioned;

/// Runtime channels only. The state machine owns logical commands and publication progress.
#[derive(Default)]
pub(crate) struct Replies(Mutex<ReplyState>);
#[derive(Default)]
struct ReplyState {
	next: u64,
	pending: BTreeMap<u64, oneshot::Sender<EmergencyRotateResponse>>,
}
struct PendingReply {
	replies: Arc<Replies>,
	call: u64,
}
impl Drop for PendingReply {
	fn drop(&mut self) {
		self.replies.0.lock().pending.remove(&self.call);
	}
}
impl Replies {
	pub(crate) fn complete(&self, call: u64, response: Option<EmergencyRotateResponse>) {
		if let Some(sender) = self.0.lock().pending.remove(&call)
			&& let Some(response) = response
		{
			let _ = sender.send(response);
		}
	}
	pub(crate) fn close(&self) {
		self.0.lock().pending.clear();
	}
	async fn request(
		self: &Arc<Self>,
		inbox: &mpsc::Sender<Event>,
		request: EmergencyRotateRequest,
	) -> Result<EmergencyRotateResponse> {
		let (sender, receiver) = oneshot::channel();
		let call = {
			let mut state = self.0.lock();
			ensure!(
				state.pending.is_empty(),
				"JWT operator request already pending"
			);
			state.next = state
				.next
				.checked_add(1)
				.context("JWT reply sequence exhausted")?;
			let call = state.next;
			state.pending.insert(call, sender);
			call
		};
		let _pending = PendingReply {
			replies: self.clone(),
			call,
		};
		inbox
			.send(Event::EmergencyRotateRequest { call, request })
			.await
			.context("JWT coordinator stopped")?;
		receiver
			.await
			.context("JWT coordinator lost authority before confirming rotation")
	}
}

fn capacities() -> SubjectCapacities {
	SubjectCapacities {
		incoming: 1,
		outgoing: 1,
	}
}

/// Discover the holder once outside this adapter, then send once under that exact tenure.
/// No adapter retries or changes the request ID after an uncertain outcome.
pub async fn request(
	pubsub: &PubSub,
	topology: &Topology,
	tenure: &CoordinatorTenure,
	request: EmergencyRotateRequest,
	admin_token: &str,
	timeout: Duration,
) -> Result<EmergencyRotateResponse> {
	ensure!(
		request.expected_coordinator_epoch == tenure.coordinator_generation,
		"coordinator tenure changed; inspect it before retrying"
	);
	ensure!(topology.may_admit()?, "JWT operator caller is not serving");
	let subject = JwtRotationSubject::new(tenure.owner.worker_id.clone());
	let _reservation =
		pubsub.reserve_request_subject(&subject, RequestClass::DepotCoordinator, capacities())?;
	let envelope = OperatorRequest {
		admin_token: admin_token.to_owned().into(),
		rotation: request,
	};
	let bytes = AuthJwtEmergencyRotateRequestData::serialize_with_embedded_version(
		&envelope,
		AuthJwtEmergencyRotateRequestData::LATEST,
	)?;
	ensure!(
		bytes.len() <= MAX_OPERATOR_BYTES,
		"JWT operator request exceeds byte limit"
	);
	let reply = pubsub.request_with_timeout_guarded(RequestClass::DepotCoordinator, &tenure.owner.worker_id, subject, &bytes, timeout, &|| topology.may_admit().unwrap_or(false)).await.context("JWT rotation outcome is unknown; inspect `auth jwt show` and retry only with the same request ID and body")?;
	ensure!(
		topology.may_admit()?,
		"JWT operator caller lost its lease before the reply"
	);
	ensure!(
		reply.len() <= MAX_OPERATOR_BYTES,
		"JWT operator reply exceeds byte limit"
	);
	AuthJwtEmergencyRotateResponseData::deserialize_with_embedded_version(&reply)
}

/// The subscription is owned by one tenure and dropped before the runner and its effects join.
/// Serial admission bounds channels independently of request IDs chosen by an operator.
pub(crate) async fn serve(
	pubsub: PubSub,
	authority: Authority,
	inbox: mpsc::Sender<Event>,
	replies: Arc<Replies>,
	config: Config,
) -> Result<()> {
	ensure!(
		authority.tenure().owner.worker_id == *pubsub.worker_id()
			&& authority.tenure().owner.instance_id.as_bytes() == pubsub.instance_id().as_bytes(),
		"JWT singleton and UPS must share worker identity"
	);
	let subject = JwtRotationSubject::new(pubsub.worker_id().clone());
	let _reservation =
		pubsub.reserve_request_subject(&subject, RequestClass::DepotCoordinator, capacities())?;
	let mut requests = pubsub.subscribe_request(subject)?;
	while let Some(message) = requests.next().await? {
		if !authority.permits() || message.is_request_expired()? {
			continue;
		}
		if message.payload.len() > MAX_OPERATOR_BYTES {
			let _ = message
				.fail(&universalpubsub::errors::InvalidMessage.build())
				.await;
			continue;
		}
		let request = match AuthJwtEmergencyRotateRequestData::deserialize_with_embedded_version(
			&message.payload,
		) {
			Ok(request) => request,
			Err(_) => {
				let _ = message
					.fail(&universalpubsub::errors::InvalidMessage.build())
					.await;
				continue;
			}
		};
		if !admin_authorized(
			config.auth_required()?.admin_token.read(),
			&request.admin_token,
		) {
			let response = EmergencyRotateResponse::Rejected(Rejected::Unauthorized);
			let bytes = AuthJwtEmergencyRotateResponseData::serialize_with_embedded_version(
				&response,
				AuthJwtEmergencyRotateResponseData::LATEST,
			)?;
			if authority.permits() && !message.is_request_expired()? {
				let _ = message.reply(&bytes);
			}
			continue;
		}
		let response = match rivet_env::timeout(
			pubsub.env().clock().as_ref(),
			Duration::from_secs(30),
			replies.request(&inbox, request.rotation),
		)
		.await
		{
			Ok(Ok(response)) => response,
			Ok(Err(_)) | Err(_) => EmergencyRotateResponse::Rejected(Rejected::Unavailable),
		};
		let bytes = AuthJwtEmergencyRotateResponseData::serialize_with_embedded_version(
			&response,
			AuthJwtEmergencyRotateResponseData::LATEST,
		)?;
		if authority.permits() && !message.is_request_expired()? {
			let _ = message.reply(&bytes);
		}
	}
	Ok(())
}

/// Public operator diagnostics contain key IDs and receipts, never private seeds or ring bytes.
pub async fn status(store: &dyn object_store::ObjectStore) -> Result<serde_json::Value> {
	let ring = match crate::store::read(store).await {
		crate::key_ring_machine::types::ReadResult::Found(stored) => {
			crate::decode_signing_key_ring(&stored.bytes)?
		}
		crate::key_ring_machine::types::ReadResult::Missing => {
			anyhow::bail!("JWT key ring is not initialized")
		}
		crate::key_ring_machine::types::ReadResult::Unknown(error)
		| crate::key_ring_machine::types::ReadResult::Invalid(error) => {
			anyhow::bail!("JWT key ring unavailable: {error}")
		}
	};
	Ok(serde_json::json!({
		"ringVersion": ring.generation,
		"activeKid": ring.active.key.kid.to_string(),
		"pendingKid": ring.pending.as_ref().map(|key| key.key.kid.to_string()),
		"retiringKids": ring.retiring.iter().map(|key| key.key.kid.to_string()).collect::<Vec<_>>(),
		"lastEmergency": ring.last_emergency_receipt.map(|receipt| serde_json::json!({"requestId": uuid::Uuid::from_bytes(receipt.request_id), "expectedRingVersion": receipt.expected_generation, "committedRingVersion": receipt.committed_generation, "revokedKids": receipt.revoked_kids.iter().map(ToString::to_string).collect::<Vec<_>>(), "requestedRevokedKids": receipt.requested_revoked_kids.iter().map(ToString::to_string).collect::<Vec<_>>() })),
	}))
}

const MAX_OPERATOR_BYTES: usize = 4_096;

/// Compare fixed-size digests so credential length does not change comparison time.
fn admin_authorized(expected: &str, presented: &str) -> bool {
	!expected.is_empty()
		&& bool::from(
			Sha256::digest(expected.as_bytes()).ct_eq(&Sha256::digest(presented.as_bytes())),
		)
}


