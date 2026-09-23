use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{auth_jwt_key_ring_v1, auth_jwt_key_ring_v2, auth_jwt_key_ring_v3};

pub enum AuthJwtKeyRingData {
	V1(auth_jwt_key_ring_v1::Data),
	V2(auth_jwt_key_ring_v2::Data),
	V3(auth_jwt_key_ring_v3::Data),
}

impl OwnedVersionedData for AuthJwtKeyRingData {
	type Latest = auth_jwt_key_ring_v3::Data;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let Self::V3(data) = self {
			Ok(data)
		} else {
			bail!("version not latest")
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl AuthJwtKeyRingData {
	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(_) = self else {
			bail!("unexpected JWT key-ring version")
		};
		bail!("public-only JWT key rings cannot be migrated without signing material")
	}

	fn v3_to_v2(self) -> Result<Self> {
		let Self::V3(value) = self else {
			bail!("unexpected JWT key-ring version")
		};
		let owner_datacenter_id = value.leader_datacenter_id;
		let owner_epoch = value.leader_epoch;
		let generation = value.generation;
		Ok(Self::V2(auth_jwt_key_ring_v2::Data {
			claims_version: value.claims_version,
			issuer_state: auth_jwt_key_ring_v2::IssuerState {
				active: auth_jwt_key_ring_v2::ActiveIssuer {
					issuer: value.issuer_state.active.issuer,
					activated_ts: value.issuer_state.active.activated_ts,
					config_generation: value.issuer_state.active.config_generation,
				},
				retiring: value
					.issuer_state
					.retiring
					.into_iter()
					.map(|issuer| auth_jwt_key_ring_v2::RetiringIssuer {
						issuer: issuer.issuer,
						retired_ts: issuer.retired_ts,
						accept_until_ts: issuer.accept_until_ts,
					})
					.collect(),
				history: value
					.issuer_state
					.history
					.into_iter()
					.map(|issuer| auth_jwt_key_ring_v2::IssuerHistory {
						issuer: issuer.issuer,
						activated_ts: issuer.activated_ts,
						retired_ts: issuer.retired_ts,
						config_generation: issuer.config_generation,
					})
					.collect(),
			},
			audience: value.audience,
			leader_datacenter_id: value.leader_datacenter_id,
			leader_epoch: value.leader_epoch,
			leader_config_generation: value.leader_config_generation,
			generation,
			active: auth_jwt_key_ring_v2::ActiveKey {
				key: public_v3_to_v2(
					value.active.key.public,
					owner_datacenter_id,
					owner_epoch,
					generation,
				),
				activated_ts: value.active.activated_ts,
				sign_until_ts: value.active.sign_until_ts,
			},
			pending: value
				.pending
				.map(|pending| auth_jwt_key_ring_v2::PendingKey {
					key: public_v3_to_v2(
						pending.key.public,
						owner_datacenter_id,
						owner_epoch,
						generation,
					),
					activate_after_ts: pending.activate_after_ts,
				}),
			retiring: value
				.retiring
				.into_iter()
				.map(|retiring| auth_jwt_key_ring_v2::RetiringKey {
					key: public_v3_to_v2(
						retiring.key,
						owner_datacenter_id,
						owner_epoch,
						generation,
					),
					retired_ts: retiring.retired_ts,
					max_token_exp_ts: retiring.max_token_exp_ts,
					verify_until_ts: retiring.verify_until_ts,
				})
				.collect(),
			last_emergency_receipt: value.last_emergency_receipt.map(|receipt| {
				auth_jwt_key_ring_v2::EmergencyReceipt {
					request_id: receipt.request_id,
					expected_generation: receipt.expected_generation,
					committed_generation: receipt.committed_generation,
					revoked_kids: receipt.revoked_kids,
					obsolete_private_keys: Vec::new(),
				}
			}),
		}))
	}

	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(value) = self else {
			bail!("unexpected JWT key-ring version")
		};
		let issuer = value.issuer;
		Ok(Self::V2(auth_jwt_key_ring_v2::Data {
			claims_version: value.claims_version,
			issuer_state: auth_jwt_key_ring_v2::IssuerState {
				active: auth_jwt_key_ring_v2::ActiveIssuer {
					issuer: issuer.clone(),
					activated_ts: value.active.activated_ts,
					config_generation: 1,
				},
				retiring: Vec::new(),
				history: vec![auth_jwt_key_ring_v2::IssuerHistory {
					issuer,
					activated_ts: value.active.activated_ts,
					retired_ts: None,
					config_generation: 1,
				}],
			},
			audience: value.audience,
			leader_datacenter_id: value.leader_datacenter_id,
			leader_epoch: value.leader_epoch,
			leader_config_generation: 1,
			generation: value.generation,
			active: auth_jwt_key_ring_v2::ActiveKey {
				key: convert_public_key_v1_to_v2(value.active.key),
				activated_ts: value.active.activated_ts,
				sign_until_ts: value.active.sign_until_ts,
			},
			pending: value
				.pending
				.map(|pending| auth_jwt_key_ring_v2::PendingKey {
					key: convert_public_key_v1_to_v2(pending.key),
					activate_after_ts: pending.activate_after_ts,
				}),
			retiring: value
				.retiring
				.into_iter()
				.map(|retiring| auth_jwt_key_ring_v2::RetiringKey {
					key: convert_public_key_v1_to_v2(retiring.key),
					retired_ts: retiring.retired_ts,
					max_token_exp_ts: retiring.max_token_exp_ts,
					verify_until_ts: retiring.verify_until_ts,
				})
				.collect(),
			last_emergency_receipt: value.last_emergency_receipt.map(|receipt| {
				auth_jwt_key_ring_v2::EmergencyReceipt {
					request_id: receipt.request_id,
					expected_generation: receipt.expected_generation,
					committed_generation: receipt.committed_generation,
					revoked_kids: receipt.revoked_kids,
					obsolete_private_keys: receipt
						.obsolete_private_keys
						.into_iter()
						.map(|key| auth_jwt_key_ring_v2::PrivateKeyRef {
							owner_datacenter_id: key.owner_datacenter_id,
							owner_epoch: key.owner_epoch,
							target_generation: key.target_generation,
						})
						.collect(),
				}
			}),
		}))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(value) = self else {
			bail!("unexpected JWT key-ring version")
		};
		if !value.issuer_state.retiring.is_empty()
			|| value.issuer_state.history.len() != 1
			|| value.issuer_state.active.config_generation != 1
			|| value.issuer_state.history[0].config_generation != 1
			|| value.leader_config_generation != 1
		{
			bail!("JWT key ring with issuer migration state cannot be encoded as v1");
		}
		Ok(Self::V1(auth_jwt_key_ring_v1::Data {
			claims_version: value.claims_version,
			issuer: value.issuer_state.active.issuer,
			audience: value.audience,
			leader_datacenter_id: value.leader_datacenter_id,
			leader_epoch: value.leader_epoch,
			generation: value.generation,
			active: auth_jwt_key_ring_v1::ActiveKey {
				key: convert_public_key_v2_to_v1(value.active.key),
				activated_ts: value.active.activated_ts,
				sign_until_ts: value.active.sign_until_ts,
			},
			pending: value
				.pending
				.map(|pending| auth_jwt_key_ring_v1::PendingKey {
					key: convert_public_key_v2_to_v1(pending.key),
					activate_after_ts: pending.activate_after_ts,
				}),
			retiring: value
				.retiring
				.into_iter()
				.map(|retiring| auth_jwt_key_ring_v1::RetiringKey {
					key: convert_public_key_v2_to_v1(retiring.key),
					retired_ts: retiring.retired_ts,
					max_token_exp_ts: retiring.max_token_exp_ts,
					verify_until_ts: retiring.verify_until_ts,
				})
				.collect(),
			last_emergency_receipt: value.last_emergency_receipt.map(|receipt| {
				auth_jwt_key_ring_v1::EmergencyReceipt {
					request_id: receipt.request_id,
					expected_generation: receipt.expected_generation,
					committed_generation: receipt.committed_generation,
					revoked_kids: receipt.revoked_kids,
					obsolete_private_keys: receipt
						.obsolete_private_keys
						.into_iter()
						.map(|key| auth_jwt_key_ring_v1::PrivateKeyRef {
							owner_datacenter_id: key.owner_datacenter_id,
							owner_epoch: key.owner_epoch,
							target_generation: key.target_generation,
						})
						.collect(),
				}
			}),
		}))
	}
}

fn convert_public_key_v1_to_v2(
	value: auth_jwt_key_ring_v1::PublicKeyRecord,
) -> auth_jwt_key_ring_v2::PublicKeyRecord {
	auth_jwt_key_ring_v2::PublicKeyRecord {
		kid: value.kid,
		algorithm: match value.algorithm {
			auth_jwt_key_ring_v1::Algorithm::Ed25519 => auth_jwt_key_ring_v2::Algorithm::Ed25519,
		},
		public_key: value.public_key,
		owner_datacenter_id: value.owner_datacenter_id,
		owner_epoch: value.owner_epoch,
		created_ts: value.created_ts,
		private_key_generation: value.private_key_generation,
	}
}

fn convert_public_key_v2_to_v1(
	value: auth_jwt_key_ring_v2::PublicKeyRecord,
) -> auth_jwt_key_ring_v1::PublicKeyRecord {
	auth_jwt_key_ring_v1::PublicKeyRecord {
		kid: value.kid,
		algorithm: match value.algorithm {
			auth_jwt_key_ring_v2::Algorithm::Ed25519 => auth_jwt_key_ring_v1::Algorithm::Ed25519,
		},
		public_key: value.public_key,
		owner_datacenter_id: value.owner_datacenter_id,
		owner_epoch: value.owner_epoch,
		created_ts: value.created_ts,
		private_key_generation: value.private_key_generation,
	}
}

fn public_v3_to_v2(
	value: auth_jwt_key_ring_v3::PublicKeyRecord,
	owner_datacenter_id: u16,
	owner_epoch: u64,
	private_key_generation: u64,
) -> auth_jwt_key_ring_v2::PublicKeyRecord {
	auth_jwt_key_ring_v2::PublicKeyRecord {
		kid: value.kid,
		algorithm: match value.algorithm {
			auth_jwt_key_ring_v3::Algorithm::Ed25519 => auth_jwt_key_ring_v2::Algorithm::Ed25519,
		},
		public_key: value.public_key,
		owner_datacenter_id,
		owner_epoch,
		created_ts: value.created_ts,
		private_key_generation,
	}
}
