use anyhow::{Result, bail};
use rivet_data::{
	AUTH_JWT_KEY_RING_VERSION, generated::auth_jwt_key_ring_v1, versioned::AuthJwtKeyRingData,
};
use vbare::Versioned;

use crate::{
	ActiveIssuer, ActiveKey, Algorithm, EmergencyReceipt, IssuerHistory, IssuerState, PendingKey,
	PublicKeyRecord, RetiringIssuer, RetiringKey, SigningKey, SigningKeyRecord, SigningKeyRing,
	key_ring::{
		key_id_from_bytes, private_seed_from_bytes, public_key_from_bytes, request_id_from_bytes,
	},
};

use auth_jwt_key_ring_v1 as key_ring_v1;

pub fn encode_signing_key_ring(ring: &SigningKeyRing) -> Result<Vec<u8>> {
	ring.validate()?;
	Ok(
		AuthJwtKeyRingData::seal_to_bytes(&ring.clone().into(), AUTH_JWT_KEY_RING_VERSION)?
			.to_vec(),
	)
}

pub fn decode_signing_key_ring(bytes: &[u8]) -> Result<SigningKeyRing> {
	let ring = SigningKeyRing::try_from(AuthJwtKeyRingData::decode(bytes.to_vec().into())?)?;
	ring.validate()?;
	Ok(ring)
}

impl From<Algorithm> for key_ring_v1::Algorithm {
	fn from(value: Algorithm) -> Self {
		match value {
			Algorithm::Ed25519 => Self::Ed25519,
		}
	}
}

impl From<key_ring_v1::Algorithm> for Algorithm {
	fn from(value: key_ring_v1::Algorithm) -> Self {
		match value {
			key_ring_v1::Algorithm::Ed25519 => Self::Ed25519,
		}
	}
}

impl From<PublicKeyRecord> for key_ring_v1::PublicKeyRecord {
	fn from(value: PublicKeyRecord) -> Self {
		Self {
			kid: value.kid.as_bytes().to_vec().into(),
			algorithm: value.algorithm.into(),
			public_key: value.public_key.to_vec().into(),
			created_ts: value.created_ts,
		}
	}
}

impl TryFrom<key_ring_v1::PublicKeyRecord> for PublicKeyRecord {
	type Error = anyhow::Error;

	fn try_from(value: key_ring_v1::PublicKeyRecord) -> Result<Self> {
		Ok(Self {
			kid: key_id_from_bytes(value.kid.to_vec())?,
			algorithm: value.algorithm.into(),
			public_key: public_key_from_bytes(value.public_key.to_vec())?,
			created_ts: value.created_ts,
		})
	}
}

impl From<SigningKeyRecord> for key_ring_v1::SigningKeyRecord {
	fn from(value: SigningKeyRecord) -> Self {
		Self {
			public: value.public.into(),
			seed: value.signing_key.seed().to_vec().into(),
		}
	}
}

impl TryFrom<key_ring_v1::SigningKeyRecord> for SigningKeyRecord {
	type Error = anyhow::Error;

	fn try_from(value: key_ring_v1::SigningKeyRecord) -> Result<Self> {
		let public = PublicKeyRecord::try_from(value.public)?;
		let signing_key =
			SigningKey::from_seed(public.kid, private_seed_from_bytes(value.seed.to_vec())?);
		if signing_key.public_key() != public.public_key {
			bail!("JWT signing seed does not match its public key");
		}
		Ok(Self {
			public,
			signing_key,
		})
	}
}

impl From<SigningKeyRing> for key_ring_v1::Data {
	fn from(value: SigningKeyRing) -> Self {
		Self {
			claims_version: value.claims_version,
			issuer_state: key_ring_v1::IssuerState {
				active: key_ring_v1::ActiveIssuer {
					issuer: value.issuer_state.active.issuer.into(),
					activated_ts: value.issuer_state.active.activated_ts,
					config_generation: value.issuer_state.active.config_generation,
				},
				retiring: value
					.issuer_state
					.retiring
					.into_iter()
					.map(|issuer| key_ring_v1::RetiringIssuer {
						issuer: issuer.issuer.into(),
						retired_ts: issuer.retired_ts,
						accept_until_ts: issuer.accept_until_ts,
					})
					.collect(),
				history: value
					.issuer_state
					.history
					.into_iter()
					.map(|issuer| key_ring_v1::IssuerHistory {
						issuer: issuer.issuer.into(),
						activated_ts: issuer.activated_ts,
						retired_ts: issuer.retired_ts,
						config_generation: issuer.config_generation,
					})
					.collect(),
			},
			audience: value.audience.into(),
			leader_datacenter_id: value.leader_datacenter_id,
			leader_epoch: value.leader_epoch,
			leader_config_generation: value.leader_config_generation,
			generation: value.generation,
			active: key_ring_v1::ActiveKey {
				key: value.active.key.into(),
				activated_ts: value.active.activated_ts,
				sign_until_ts: value.active.sign_until_ts,
			},
			pending: value.pending.map(|pending| key_ring_v1::PendingKey {
				key: pending.key.into(),
				activate_after_ts: pending.activate_after_ts,
			}),
			retiring: value
				.retiring
				.into_iter()
				.map(|retiring| key_ring_v1::RetiringKey {
					key: retiring.key.into(),
					retired_ts: retiring.retired_ts,
					max_token_exp_ts: retiring.max_token_exp_ts,
					verify_until_ts: retiring.verify_until_ts,
				})
				.collect(),
			last_emergency_receipt: value.last_emergency_receipt.map(|receipt| {
				key_ring_v1::EmergencyReceipt {
					request_id: receipt.request_id.to_vec().into(),
					requested_revoked_kids: receipt
						.requested_revoked_kids
						.into_iter()
						.map(|kid| kid.as_bytes().to_vec().into())
						.collect(),
					expected_generation: receipt.expected_generation,
					committed_generation: receipt.committed_generation,
					revoked_kids: receipt
						.revoked_kids
						.into_iter()
						.map(|kid| kid.as_bytes().to_vec().into())
						.collect(),
				}
			}),
		}
	}
}

impl TryFrom<key_ring_v1::Data> for SigningKeyRing {
	type Error = anyhow::Error;

	fn try_from(value: key_ring_v1::Data) -> Result<Self> {
		Ok(Self {
			claims_version: value.claims_version,
			issuer_state: IssuerState {
				active: ActiveIssuer {
					issuer: value.issuer_state.active.issuer.to_string(),
					activated_ts: value.issuer_state.active.activated_ts,
					config_generation: value.issuer_state.active.config_generation,
				},
				retiring: value
					.issuer_state
					.retiring
					.into_iter()
					.map(|issuer| RetiringIssuer {
						issuer: issuer.issuer.to_string(),
						retired_ts: issuer.retired_ts,
						accept_until_ts: issuer.accept_until_ts,
					})
					.collect(),
				history: value
					.issuer_state
					.history
					.into_iter()
					.map(|issuer| IssuerHistory {
						issuer: issuer.issuer.to_string(),
						activated_ts: issuer.activated_ts,
						retired_ts: issuer.retired_ts,
						config_generation: issuer.config_generation,
					})
					.collect(),
			},
			audience: value.audience.to_string(),
			leader_datacenter_id: value.leader_datacenter_id,
			leader_epoch: value.leader_epoch,
			leader_config_generation: value.leader_config_generation,
			generation: value.generation,
			active: ActiveKey {
				key: value.active.key.try_into()?,
				activated_ts: value.active.activated_ts,
				sign_until_ts: value.active.sign_until_ts,
			},
			pending: value
				.pending
				.map(|pending| -> Result<PendingKey> {
					Ok(PendingKey {
						key: pending.key.try_into()?,
						activate_after_ts: pending.activate_after_ts,
					})
				})
				.transpose()?,
			retiring: value
				.retiring
				.into_iter()
				.map(|retiring| -> Result<RetiringKey> {
					Ok(RetiringKey {
						key: retiring.key.try_into()?,
						retired_ts: retiring.retired_ts,
						max_token_exp_ts: retiring.max_token_exp_ts,
						verify_until_ts: retiring.verify_until_ts,
					})
				})
				.collect::<Result<Vec<_>>>()?,
			last_emergency_receipt: value
				.last_emergency_receipt
				.map(|receipt| -> Result<EmergencyReceipt> {
					Ok(EmergencyReceipt {
						request_id: request_id_from_bytes(receipt.request_id.to_vec())?,
						requested_revoked_kids: receipt
							.requested_revoked_kids
							.into_iter()
							.map(|kid| key_id_from_bytes(kid.to_vec()))
							.collect::<Result<Vec<_>>>()?,
						expected_generation: receipt.expected_generation,
						committed_generation: receipt.committed_generation,
						revoked_kids: receipt
							.revoked_kids
							.into_iter()
							.map(|kid| key_id_from_bytes(kid.to_vec()))
							.collect::<Result<Vec<_>>>()?,
					})
				})
				.transpose()?,
		})
	}
}
