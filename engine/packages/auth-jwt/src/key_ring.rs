use std::collections::BTreeSet;
use std::{fmt, ops::Deref};

use anyhow::{Context, Result, ensure};

use crate::{CLAIMS_VERSION, KeyId, SigningKey, VerificationKey};

pub const NORMAL_KEY_LIMIT: usize = 7;
pub const ABSOLUTE_KEY_LIMIT: usize = 8;
pub const ISSUER_HISTORY_LIMIT: usize = 16;
const MAX_ISSUER_BYTES: usize = 512;
const MAX_AUDIENCE_BYTES: usize = 128;
const REQUEST_ID_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
	Ed25519,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKeyRecord {
	pub kid: KeyId,
	pub algorithm: Algorithm,
	pub public_key: [u8; crate::keys::PUBLIC_KEY_BYTES],
	pub created_ts: i64,
}

impl PublicKeyRecord {
	pub fn verification_key(&self) -> VerificationKey {
		VerificationKey::new(self.kid, self.public_key)
	}
}

#[derive(Clone)]
pub struct SigningKeyRecord {
	pub public: PublicKeyRecord,
	pub signing_key: SigningKey,
}

impl SigningKeyRecord {
	pub fn generate(created_ts: i64) -> Self {
		let signing_key = SigningKey::generate();
		Self {
			public: PublicKeyRecord {
				kid: signing_key.kid(),
				algorithm: Algorithm::Ed25519,
				public_key: signing_key.public_key(),
				created_ts,
			},
			signing_key,
		}
	}

	pub fn validate(&self) -> Result<()> {
		ensure!(
			self.public.kid == self.signing_key.kid()
				&& self.public.public_key == self.signing_key.public_key(),
			"JWT signing material does not match its public key"
		);
		Ok(())
	}
}

impl Deref for SigningKeyRecord {
	type Target = PublicKeyRecord;

	fn deref(&self) -> &Self::Target {
		&self.public
	}
}

impl PartialEq for SigningKeyRecord {
	fn eq(&self, other: &Self) -> bool {
		self.public == other.public && self.signing_key.seed() == other.signing_key.seed()
	}
}

impl Eq for SigningKeyRecord {}

impl fmt::Debug for SigningKeyRecord {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SigningKeyRecord")
			.field("public", &self.public)
			.field("seed", &"[redacted]")
			.finish()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveKey {
	pub key: SigningKeyRecord,
	pub activated_ts: i64,
	/// Hard issuance deadline. Normal rotation happens earlier.
	pub sign_until_ts: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingKey {
	pub key: SigningKeyRecord,
	/// The key is public before this timestamp, but cannot sign until activation.
	pub activate_after_ts: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetiringKey {
	pub key: PublicKeyRecord,
	pub retired_ts: i64,
	pub max_token_exp_ts: i64,
	pub verify_until_ts: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmergencyReceipt {
	pub request_id: [u8; REQUEST_ID_BYTES],
	pub expected_generation: u64,
	pub committed_generation: u64,
	pub revoked_kids: Vec<KeyId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveIssuer {
	pub issuer: String,
	pub activated_ts: i64,
	pub config_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetiringIssuer {
	pub issuer: String,
	pub retired_ts: i64,
	pub accept_until_ts: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerHistory {
	pub issuer: String,
	pub activated_ts: i64,
	pub retired_ts: Option<i64>,
	pub config_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerState {
	pub active: ActiveIssuer,
	pub retiring: Vec<RetiringIssuer>,
	pub history: Vec<IssuerHistory>,
}

impl IssuerState {
	pub fn accepts(&self, issuer: &str, now: i64) -> bool {
		self.active.issuer == issuer
			|| self
				.retiring
				.iter()
				.any(|retiring| retiring.issuer == issuer && now < retiring.accept_until_ts)
	}

	pub fn validate(&self) -> Result<()> {
		validate_issuer(&self.active.issuer)?;
		ensure!(
			self.active.config_generation > 0,
			"active issuer configuration generation must be positive"
		);
		ensure!(
			self.history.len() <= ISSUER_HISTORY_LIMIT,
			"JWT issuer history exceeds its bounded limit"
		);

		let mut issuers = BTreeSet::new();
		ensure!(
			issuers.insert(&self.active.issuer),
			"duplicate active issuer"
		);
		for retiring in &self.retiring {
			validate_issuer(&retiring.issuer)?;
			ensure!(
				retiring.accept_until_ts > retiring.retired_ts,
				"invalid retiring issuer timestamps"
			);
			ensure!(
				issuers.insert(&retiring.issuer),
				"duplicate trusted JWT issuer"
			);
		}
		for history in &self.history {
			validate_issuer(&history.issuer)?;
			ensure!(
				history.config_generation > 0,
				"issuer history configuration generation must be positive"
			);
			ensure!(
				history
					.retired_ts
					.is_none_or(|retired_ts| retired_ts >= history.activated_ts),
				"invalid issuer history timestamps"
			);
		}
		ensure!(
			self.history.iter().any(|entry| {
				entry.issuer == self.active.issuer
					&& entry.activated_ts == self.active.activated_ts
					&& entry.config_generation == self.active.config_generation
					&& entry.retired_ts.is_none()
			}),
			"active issuer is missing from issuer history"
		);
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningKeyRing {
	pub claims_version: u16,
	pub issuer_state: IssuerState,
	pub audience: String,
	pub leader_datacenter_id: u16,
	pub leader_epoch: u64,
	/// Configuration generation that authorized the current signing leader.
	pub leader_config_generation: u64,
	pub generation: u64,
	pub active: ActiveKey,
	pub pending: Option<PendingKey>,
	pub retiring: Vec<RetiringKey>,
	pub last_emergency_receipt: Option<EmergencyReceipt>,
}

impl SigningKeyRing {
	pub fn key_count(&self) -> usize {
		1 + usize::from(self.pending.is_some()) + self.retiring.len()
	}

	pub fn all_public_keys(&self) -> impl Iterator<Item = &PublicKeyRecord> {
		std::iter::once(&self.active.key.public)
			.chain(self.pending.iter().map(|pending| &pending.key.public))
			.chain(self.retiring.iter().map(|retiring| &retiring.key))
	}

	pub fn validate(&self) -> Result<()> {
		ensure!(
			self.claims_version == CLAIMS_VERSION,
			"unsupported claims version {}",
			self.claims_version
		);
		self.issuer_state.validate()?;
		ensure!(
			!self.audience.is_empty() && self.audience.len() <= MAX_AUDIENCE_BYTES,
			"invalid JWT audience length"
		);
		ensure!(self.leader_epoch > 0, "leader epoch must be positive");
		ensure!(
			self.leader_config_generation > 0
				&& self.leader_config_generation <= self.issuer_state.active.config_generation,
			"invalid leader configuration generation"
		);
		ensure!(self.generation > 0, "key-ring generation must be positive");
		ensure!(
			self.key_count() <= ABSOLUTE_KEY_LIMIT,
			"key ring exceeds absolute key limit"
		);
		self.active.key.validate()?;
		ensure!(
			self.active.activated_ts >= self.active.key.created_ts
				&& self.active.sign_until_ts > self.active.activated_ts,
			"invalid active-key timestamps"
		);

		if let Some(pending) = &self.pending {
			pending.key.validate()?;
			ensure!(
				pending.activate_after_ts > pending.key.created_ts,
				"pending activation must follow key creation"
			);
		}

		for retiring in &self.retiring {
			ensure!(
				retiring.retired_ts >= retiring.key.created_ts
					&& retiring.max_token_exp_ts >= retiring.retired_ts
					&& retiring.verify_until_ts >= retiring.max_token_exp_ts,
				"invalid retiring-key timestamps"
			);
		}

		let mut kids = BTreeSet::new();
		for key in self.all_public_keys() {
			ensure!(
				key.algorithm == Algorithm::Ed25519,
				"unsupported key algorithm"
			);
			ensure!(kids.insert(key.kid), "duplicate key id in key ring");
		}

		if let Some(receipt) = &self.last_emergency_receipt {
			ensure!(
				receipt.expected_generation < receipt.committed_generation
					&& receipt.committed_generation <= self.generation,
				"invalid emergency receipt generations"
			);
			ensure!(
				receipt.revoked_kids.len() <= ABSOLUTE_KEY_LIMIT,
				"emergency receipt has too many revoked key ids"
			);
			let unique = receipt
				.revoked_kids
				.iter()
				.copied()
				.collect::<BTreeSet<_>>();
			ensure!(
				unique.len() == receipt.revoked_kids.len(),
				"emergency receipt contains duplicate key ids"
			);
		}

		Ok(())
	}

	pub fn validate_successor(&self, successor: &Self) -> Result<()> {
		self.validate().context("invalid previous key ring")?;
		successor.validate().context("invalid successor key ring")?;
		ensure!(
			successor.generation
				== self
					.generation
					.checked_add(1)
					.context("generation overflow")?,
			"successor must increment generation exactly once"
		);
		ensure!(
			successor.claims_version == self.claims_version && successor.audience == self.audience,
			"successor cannot change the JWT wire contract"
		);
		ensure!(
			successor.leader_epoch >= self.leader_epoch,
			"successor cannot decrease leader epoch"
		);
		ensure!(
			successor.leader_config_generation >= self.leader_config_generation,
			"successor cannot decrease leader configuration generation"
		);
		if successor.leader_epoch == self.leader_epoch {
			ensure!(
				successor.leader_datacenter_id == self.leader_datacenter_id,
				"same leader epoch cannot name a different datacenter"
			);
		}
		Ok(())
	}
}

fn validate_issuer(issuer: &str) -> Result<()> {
	ensure!(
		!issuer.is_empty() && issuer.len() <= MAX_ISSUER_BYTES,
		"invalid JWT issuer length"
	);
	Ok(())
}

pub fn request_id_from_bytes(bytes: Vec<u8>) -> Result<[u8; REQUEST_ID_BYTES]> {
	bytes
		.try_into()
		.map_err(|_| anyhow::anyhow!("invalid request id length"))
}

pub fn key_id_from_bytes(bytes: Vec<u8>) -> Result<KeyId> {
	let bytes = bytes
		.try_into()
		.map_err(|_| anyhow::anyhow!("invalid key id length"))?;
	Ok(KeyId::from_bytes(bytes))
}

pub fn public_key_from_bytes(bytes: Vec<u8>) -> Result<[u8; crate::keys::PUBLIC_KEY_BYTES]> {
	bytes
		.try_into()
		.map_err(|_| anyhow::anyhow!("invalid Ed25519 public-key length"))
}

pub fn private_seed_from_bytes(bytes: Vec<u8>) -> Result<[u8; crate::keys::PRIVATE_KEY_BYTES]> {
	bytes
		.try_into()
		.map_err(|_| anyhow::anyhow!("invalid Ed25519 private-seed length"))
}
