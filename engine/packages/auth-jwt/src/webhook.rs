//! Namespace-separated webhook keys. Internal random IDs remain private to key-ring bookkeeping;
//! public IDs are the DS-compatible thumbprints and never enter authentication-key discovery.
use crate::{Id, KeyId, PublicKeyRecord, RotationPolicy, SigningKeyRecord, SigningKeyRing};
use anyhow::{Context, Result, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rivet_data::{
	AUTH_WEBHOOK_KEY_RING_VERSION, generated::auth_webhook_key_ring_v1 as data,
	versioned::AuthWebhookKeyRingData,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vbare::OwnedVersionedData;

pub const FRESHNESS_SECONDS: u64 = 300;
// Stable internal planner identity, not a JWT issuer or a public discovery endpoint.
const PLANNER_ISSUER: &str = "https://rivet.dev/webhook-signing";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmergencyReceipt {
	pub request_id: [u8; 16],
	pub expected_generation: u64,
	pub committed_generation: u64,
	pub requested_kids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ring {
	pub namespace_id: Id,
	pub keys: SigningKeyRing,
	pub last_emergency: Option<EmergencyReceipt>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid webhook key request")]
	InvalidInput,
	#[error("webhook key generation or request ID conflict")]
	Conflict,
	#[error("webhook signing keys are not provisioned")]
	NotFound,
	#[error("webhook signing keys are unavailable")]
	Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Jwk {
	pub kty: String,
	pub crv: String,
	pub kid: String,
	#[serde(rename = "use")]
	pub public_key_use: String,
	pub alg: String,
	pub x: String,
}

pub fn public_kid(public: &[u8; 32]) -> String {
	let x = URL_SAFE_NO_PAD.encode(public);
	let canonical = format!(r#"{{"crv":"Ed25519","kty":"OKP","x":"{x}"}}"#);
	format!(
		"ds_{}",
		URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes()))
	)
}

pub fn policy() -> RotationPolicy {
	RotationPolicy {
		rotation_interval_ms: 7 * 86400 * 1000,
		publish_lead_ms: 600 * 1000,
		max_signing_lifetime_ms: 14 * 86400 * 1000,
		max_token_ttl_ms: FRESHNESS_SECONDS as i64 * 1000,
		clock_skew_ms: 30 * 1000,
	}
}

impl Ring {
	pub fn bootstrap(namespace_id: Id, dc: u16, config_generation: u64, now: i64) -> Result<Self> {
		Ok(Self {
			namespace_id,
			keys: crate::bootstrap(
				PLANNER_ISSUER.into(),
				config_generation,
				"rivet-webhooks".into(),
				dc,
				1,
				&SigningKeyRecord::generate(now),
				now,
				policy(),
			)?
			.successor,
			last_emergency: None,
		})
	}

	pub fn encode(&self) -> Result<Vec<u8>> {
		self.validate()?;
		AuthWebhookKeyRingData::wrap_latest(data::Data {
			namespace_id: self.namespace_id.as_bytes(),
			ring: crate::encode_signing_key_ring(&self.keys)?,
			last_emergency: self
				.last_emergency
				.as_ref()
				.map(|r| data::EmergencyReceipt {
					request_id: r.request_id.to_vec(),
					expected_generation: r.expected_generation,
					committed_generation: r.committed_generation,
					requested_kids: r.requested_kids.clone(),
				}),
		})
		.serialize_with_embedded_version(AUTH_WEBHOOK_KEY_RING_VERSION)
	}

	pub fn decode(namespace_id: Id, bytes: &[u8]) -> Result<Self> {
		ensure!(
			bytes.len() <= 64 * 1024,
			"webhook key ring exceeds storage bound"
		);
		let data = AuthWebhookKeyRingData::deserialize_with_embedded_version(bytes)?;
		ensure!(
			Id::from_slice(&data.namespace_id)? == namespace_id,
			"webhook key namespace mismatch"
		);
		let value = Self {
			namespace_id,
			keys: crate::decode_signing_key_ring(&data.ring)?,
			last_emergency: data
				.last_emergency
				.map(|r| -> Result<_> {
					Ok(EmergencyReceipt {
						request_id: r.request_id.try_into().map_err(|_| Error::InvalidInput)?,
						expected_generation: r.expected_generation,
						committed_generation: r.committed_generation,
						requested_kids: r.requested_kids,
					})
				})
				.transpose()?,
		};
		value.validate()?;
		Ok(value)
	}

	pub fn validate(&self) -> Result<()> {
		self.keys.validate()?;
		ensure!(
			self.keys.issuer_state.active.issuer == PLANNER_ISSUER
				&& self.keys.audience == "rivet-webhooks",
			"invalid webhook ring identity"
		);
		if let Some(r) = &self.last_emergency {
			ensure!(
				!r.requested_kids.is_empty()
					&& r.requested_kids.len() <= crate::ABSOLUTE_KEY_LIMIT
					&& r.requested_kids
						.iter()
						.all(|id| id.len() == 46 && id.starts_with("ds_"))
					&& r.requested_kids.windows(2).all(|pair| pair[0] < pair[1])
					&& r.expected_generation.checked_add(1) == Some(r.committed_generation)
					&& r.committed_generation <= self.keys.generation,
				"invalid webhook emergency receipt"
			);
		}
		Ok(())
	}

	pub fn public_keys(&self, now: i64) -> Vec<Jwk> {
		self.public_records(now)
			.into_iter()
			.map(|k| Jwk {
				kty: "OKP".into(),
				crv: "Ed25519".into(),
				kid: public_kid(&k.public_key),
				public_key_use: "sig".into(),
				alg: "EdDSA".into(),
				x: URL_SAFE_NO_PAD.encode(k.public_key),
			})
			.collect()
	}

	fn public_records(&self, now: i64) -> Vec<&PublicKeyRecord> {
		let mut keys = vec![&self.keys.active.key.public];
		if let Some(pending) = &self.keys.pending {
			keys.push(&pending.key.public);
		}
		keys.extend(
			self.keys
				.retiring
				.iter()
				.filter(|key| now < key.verify_until_ts)
				.map(|key| &key.key),
		);
		keys
	}

	/// One planner transition per CAS. Topology generation fences the old leader.
	pub fn reconcile_topology(
		&self,
		dc: u16,
		config_generation: u64,
		now: i64,
	) -> Result<Option<Self>> {
		let transition = match crate::reconcile_issuer(
			&self.keys,
			PLANNER_ISSUER,
			dc,
			config_generation,
			&[],
			now,
			policy(),
		)? {
			crate::IssuerReconcile::StaleConfiguration => return Err(Error::Unavailable.into()),
			crate::IssuerReconcile::Transition(t) => Some(t),
			crate::IssuerReconcile::Stable if self.keys.leader_datacenter_id != dc => {
				Some(crate::recover_leader(
					&self.keys,
					dc,
					self.keys
						.leader_epoch
						.checked_add(1)
						.context("leader epoch overflow")?,
					config_generation,
				)?)
			}
			crate::IssuerReconcile::Stable => None,
		};
		Ok(transition.map(|t| Self {
			keys: t.successor,
			..self.clone()
		}))
	}
	pub fn reconcile(&self, dc: u16, config_generation: u64, now: i64) -> Result<Option<Self>> {
		let transition = match crate::reconcile_issuer(
			&self.keys,
			PLANNER_ISSUER,
			dc,
			config_generation,
			&[],
			now,
			policy(),
		)? {
			crate::IssuerReconcile::StaleConfiguration => return Err(Error::Unavailable.into()),
			crate::IssuerReconcile::Transition(t) => Some(t),
			crate::IssuerReconcile::Stable if self.keys.leader_datacenter_id != dc => {
				Some(crate::recover_leader(
					&self.keys,
					dc,
					self.keys
						.leader_epoch
						.checked_add(1)
						.context("leader epoch overflow")?,
					config_generation,
				)?)
			}
			crate::IssuerReconcile::Stable => {
				if let Some(t) = crate::prune_retiring(&self.keys, now)? {
					Some(t)
				} else if self
					.keys
					.pending
					.as_ref()
					.is_some_and(|key| now >= key.activate_after_ts)
				{
					Some(crate::activate_pending(&self.keys, now, policy())?)
				} else if self.keys.pending.is_none()
					&& now
						>= self.keys.active.activated_ts + policy().rotation_interval_ms
							- policy().publish_lead_ms
				{
					Some(crate::stage_pending(
						&self.keys,
						&SigningKeyRecord::generate(now),
						now,
						policy(),
					)?)
				} else {
					None
				}
			}
		};
		Ok(transition.map(|t| Self {
			keys: t.successor,
			..self.clone()
		}))
	}

	pub fn rotate(&self, expected_generation: u64, now: i64) -> Result<Self> {
		if self.keys.generation != expected_generation {
			return Err(Error::Conflict.into());
		}
		if self.keys.pending.is_some() {
			return Ok(self.clone());
		}
		Ok(Self {
			keys: crate::stage_pending_forced(
				&self.keys,
				&SigningKeyRecord::generate(now),
				now,
				policy(),
			)?
			.successor,
			..self.clone()
		})
	}

	pub fn revoke(
		&self,
		request_id: [u8; 16],
		expected_generation: u64,
		requested_kids: &[String],
		now: i64,
	) -> Result<Self> {
		let mut requested = requested_kids.to_vec();
		requested.sort();
		if requested.is_empty()
			|| requested.len() > crate::ABSOLUTE_KEY_LIMIT
			|| requested.windows(2).any(|w| w[0] == w[1])
		{
			return Err(Error::InvalidInput.into());
		}
		// Compare original public arguments before resolving IDs removed by the first commit.
		if let Some(receipt) = &self.last_emergency {
			if receipt.request_id == request_id {
				return if receipt.expected_generation == expected_generation
					&& receipt.requested_kids == requested
				{
					Ok(self.clone())
				} else {
					Err(Error::Conflict.into())
				};
			}
		}
		if self.keys.generation != expected_generation {
			return Err(Error::Conflict.into());
		}
		let keys: Vec<KeyId> = requested
			.iter()
			.map(|id| {
				self.public_records(now)
					.into_iter()
					.find(|key| public_kid(&key.public_key) == *id)
					.map(|key| key.kid)
					.ok_or(Error::InvalidInput)
			})
			.collect::<Result<_, _>>()?;
		let successor = crate::emergency_rotate(
			&self.keys,
			&SigningKeyRecord::generate(now),
			request_id,
			expected_generation,
			&keys,
			now,
			policy(),
		)?
		.successor;
		Ok(Self {
			namespace_id: self.namespace_id,
			last_emergency: Some(EmergencyReceipt {
				request_id,
				expected_generation,
				committed_generation: successor.generation,
				requested_kids: requested,
			}),
			keys: successor,
		})
	}
}

#[cfg(test)]
#[path = "../tests/unit/webhook.rs"]
mod tests;

pub const MAX_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct Signature {
	pub timestamp: String,
	pub kid: String,
	pub signature: String,
}
impl std::fmt::Debug for Signature {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("WebhookSignature")
			.field("timestamp", &self.timestamp)
			.field("kid", &self.kid)
			.finish_non_exhaustive()
	}
}

/// Plain Ed25519 over decimal Unix seconds, '.', and the unchanged raw body.
/// Production callers supply Engine time and enforce their authoritative signing lease.
pub fn sign(ring: &Ring, body: &[u8], now: i64) -> Result<Signature> {
	use ed25519_dalek::Signer;
	ensure!(
		body.len() <= MAX_PAYLOAD_BYTES,
		"webhook payload exceeds maximum"
	);
	ensure!(
		now >= ring.keys.active.activated_ts && now < ring.keys.active.sign_until_ts,
		"webhook signer is outside its signing lifetime"
	);
	let timestamp = u64::try_from(now / 1000)?.to_string();
	let mut input = Vec::with_capacity(timestamp.len() + 1 + body.len());
	input.extend_from_slice(timestamp.as_bytes());
	input.push(b'.');
	input.extend_from_slice(body);
	let key = ed25519_dalek::SigningKey::from_bytes(ring.keys.active.key.signing_key.seed());
	let signature = key.sign(&input);
	Ok(Signature {
		timestamp,
		kid: public_kid(&key.verifying_key().to_bytes()),
		signature: URL_SAFE_NO_PAD.encode(signature.to_bytes()),
	})
}
