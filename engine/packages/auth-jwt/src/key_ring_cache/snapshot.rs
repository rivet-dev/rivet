use std::{
	collections::{BTreeMap, BTreeSet},
	time::Instant,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::KeyRingCacheMode;
use crate::{CLAIMS_VERSION, KeyId, SigningKeyRing, VerificationKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReadPath {
	Owner,
	Linearizable,
	LocalFallback,
}
impl ReadPath {
	pub(super) fn label(self) -> &'static str {
		match self {
			Self::Owner => "owner",
			Self::Linearizable => "linearizable",
			Self::LocalFallback => "local_fallback",
		}
	}
	pub(super) fn authoritative(self) -> bool {
		self != Self::LocalFallback
	}
}

pub(super) struct FetchedSnapshot {
	pub(super) path: ReadPath,
	pub(super) fingerprint: Option<[u8; 32]>,
	pub(super) verification: VerificationSnapshot,
	pub(super) active_signer: Option<crate::ActiveKey>,
}

impl FetchedSnapshot {
	pub(super) fn from_authoritative(
		ring: SigningKeyRing,
		mode: KeyRingCacheMode,
		now: i64,
	) -> Result<Self> {
		ring.validate()?;
		let encoded = zeroize::Zeroizing::new(crate::encode_signing_key_ring(&ring)?);
		let fingerprint = Some(Sha256::digest(encoded.as_slice()).into());
		let verification = VerificationSnapshot::from_authoritative(&ring, now);
		// Dropping the ring zeroizes every private seed except the active key explicitly moved
		// into a signing-enabled cache. Pending private material is never retained here.
		let active_signer = match mode {
			KeyRingCacheMode::VerifyOnly => None,
			KeyRingCacheMode::SignAndVerify => Some(ring.active),
		};
		Ok(Self {
			path: ReadPath::Owner,
			fingerprint,
			verification,
			active_signer,
		})
	}
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationSnapshot {
	pub claims_version: u16,
	pub active_issuer: String,
	pub active_issuer_activated_ts: i64,
	pub retiring_issuers: Vec<VerificationIssuerEntry>,
	pub audience: String,
	pub generation: u64,
	pub keys: Vec<VerificationKeyEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationIssuerEntry {
	pub issuer: String,
	pub accept_until_ts: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationKeyEntry {
	pub kid: String,
	pub public_key: Vec<u8>,
	pub lifecycle: VerificationKeyLifecycle,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VerificationKeyLifecycle {
	Pending {
		activate_after_ts: i64,
	},
	Active {
		activated_ts: i64,
		sign_until_ts: i64,
	},
	Retiring {
		retired_ts: i64,
		max_token_exp_ts: i64,
		verify_until_ts: i64,
	},
}

impl VerificationSnapshot {
	pub fn from_authoritative(ring: &SigningKeyRing, now: i64) -> Self {
		let mut keys = Vec::with_capacity(ring.key_count());
		keys.push(VerificationKeyEntry::new(
			&ring.active.key,
			VerificationKeyLifecycle::Active {
				activated_ts: ring.active.activated_ts,
				sign_until_ts: ring.active.sign_until_ts,
			},
		));
		if let Some(pending) = &ring.pending {
			keys.push(VerificationKeyEntry::new(
				&pending.key,
				VerificationKeyLifecycle::Pending {
					activate_after_ts: pending.activate_after_ts,
				},
			));
		}
		keys.extend(ring.retiring.iter().map(|retiring| {
			VerificationKeyEntry::new(
				&retiring.key,
				VerificationKeyLifecycle::Retiring {
					retired_ts: retiring.retired_ts,
					max_token_exp_ts: retiring.max_token_exp_ts,
					verify_until_ts: retiring.verify_until_ts,
				},
			)
		}));

		Self {
			claims_version: ring.claims_version,
			active_issuer: ring.issuer_state.active.issuer.clone(),
			active_issuer_activated_ts: ring.issuer_state.active.activated_ts,
			retiring_issuers: ring
				.issuer_state
				.retiring
				.iter()
				.filter(|issuer| now < issuer.accept_until_ts)
				.map(|issuer| VerificationIssuerEntry {
					issuer: issuer.issuer.clone(),
					accept_until_ts: issuer.accept_until_ts,
				})
				.collect(),
			audience: ring.audience.clone(),
			generation: ring.generation,
			keys,
		}
	}

	pub(super) fn validate(&self) -> Result<()> {
		ensure!(
			self.claims_version == CLAIMS_VERSION,
			"unsupported JWT claims version"
		);
		ensure!(!self.active_issuer.is_empty(), "empty active JWT issuer");
		ensure!(!self.audience.is_empty(), "empty JWT audience");
		ensure!(self.generation > 0, "invalid JWT key-ring generation");
		ensure!(
			!self.keys.is_empty() && self.keys.len() <= crate::ABSOLUTE_KEY_LIMIT,
			"invalid JWT verification-key count"
		);

		let mut kids = BTreeSet::new();
		for key in &self.keys {
			let kid: KeyId = key.kid.parse().context("invalid JWT verification key id")?;
			ensure!(kids.insert(kid), "duplicate JWT verification key id");
			let _: [u8; crate::keys::PUBLIC_KEY_BYTES] = key
				.public_key
				.as_slice()
				.try_into()
				.context("invalid Ed25519 public key length")?;
		}
		let mut issuers = BTreeSet::from([self.active_issuer.as_str()]);
		for issuer in &self.retiring_issuers {
			ensure!(!issuer.issuer.is_empty(), "empty retiring JWT issuer");
			ensure!(
				issuers.insert(&issuer.issuer),
				"duplicate JWT verification issuer"
			);
		}
		Ok(())
	}
}

impl VerificationKeyEntry {
	fn new(key: &crate::PublicKeyRecord, lifecycle: VerificationKeyLifecycle) -> Self {
		Self {
			kid: key.kid.to_string(),
			public_key: key.public_key.to_vec(),
			lifecycle,
		}
	}
}

#[derive(Clone)]
pub(super) struct CachedKey {
	pub(super) key: VerificationKey,
	pub(super) lifecycle: VerificationKeyLifecycle,
}

pub(super) struct CachedSnapshot {
	pub(super) verification: VerificationSnapshot,
	pub(super) fingerprint: Option<[u8; 32]>,
	pub(super) path: ReadPath,
	pub(super) refreshed_at: Instant,
	pub(super) generation: u64,
	pub(super) fetched_at: Instant,
	pub(super) read_started_at: Instant,
	pub(super) read_started_ts: i64,
	pub(super) active_signer: Option<crate::ActiveKey>,
	pub(super) active_issuer: String,
	pub(super) retiring_issuers: Vec<VerificationIssuerEntry>,
	pub(super) audience: String,
	pub(super) keys: BTreeMap<KeyId, CachedKey>,
}

impl CachedSnapshot {
	pub(super) fn from_wire(snapshot: VerificationSnapshot) -> Result<Self> {
		let verification = snapshot.clone();
		let mut keys = BTreeMap::new();
		for entry in snapshot.keys {
			let kid: KeyId = entry.kid.parse()?;
			let public_key = entry
				.public_key
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid Ed25519 public key length"))?;
			ensure!(
				keys.insert(
					kid,
					CachedKey {
						key: VerificationKey::new(kid, public_key),
						lifecycle: entry.lifecycle,
					},
				)
				.is_none(),
				"duplicate JWT verification key id"
			);
		}
		Ok(Self {
			verification,
			fingerprint: None,
			path: ReadPath::Owner,
			refreshed_at: Instant::now(),
			generation: snapshot.generation,
			fetched_at: Instant::now(),
			read_started_at: Instant::now(),
			read_started_ts: rivet_util::timestamp::now(),
			active_signer: None,
			active_issuer: snapshot.active_issuer,
			retiring_issuers: snapshot.retiring_issuers,
			audience: snapshot.audience,
			keys,
		})
	}
}
