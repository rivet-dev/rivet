use std::{fmt, str::FromStr};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{SigningKey as DalekSigningKey, pkcs8::EncodePrivateKey};
use jsonwebtoken::{DecodingKey, EncodingKey};
use rand::{CryptoRng, RngCore, rngs::OsRng};
use zeroize::Zeroizing;

use crate::TokenError;

pub const KEY_ID_BYTES: usize = 16;
pub const PRIVATE_KEY_BYTES: usize = 32;
pub const PUBLIC_KEY_BYTES: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId([u8; KEY_ID_BYTES]);

impl KeyId {
	pub fn generate(rng: &mut (impl CryptoRng + RngCore)) -> Self {
		let mut bytes = [0; KEY_ID_BYTES];
		rng.fill_bytes(&mut bytes);
		Self(bytes)
	}

	pub fn from_bytes(bytes: [u8; KEY_ID_BYTES]) -> Self {
		Self(bytes)
	}

	pub fn as_bytes(&self) -> &[u8; KEY_ID_BYTES] {
		&self.0
	}
}

impl fmt::Debug for KeyId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

impl fmt::Display for KeyId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&URL_SAFE_NO_PAD.encode(self.0))
	}
}

impl FromStr for KeyId {
	type Err = TokenError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.contains('=') {
			return Err(TokenError::InvalidKeyId);
		}

		let decoded = URL_SAFE_NO_PAD
			.decode(value)
			.map_err(|_| TokenError::InvalidKeyId)?;
		let bytes: [u8; KEY_ID_BYTES] = decoded.try_into().map_err(|_| TokenError::InvalidKeyId)?;
		let key_id = Self(bytes);
		if key_id.to_string() != value {
			return Err(TokenError::InvalidKeyId);
		}

		Ok(key_id)
	}
}

#[derive(Clone)]
pub struct SigningKey {
	kid: KeyId,
	seed: Zeroizing<[u8; PRIVATE_KEY_BYTES]>,
	public_key: [u8; PUBLIC_KEY_BYTES],
}

impl SigningKey {
	pub fn generate() -> Self {
		Self::generate_with(&mut OsRng)
	}

	pub fn generate_with(rng: &mut (impl CryptoRng + RngCore)) -> Self {
		let kid = KeyId::generate(rng);
		let key = DalekSigningKey::generate(rng);
		Self {
			kid,
			seed: Zeroizing::new(key.to_bytes()),
			public_key: key.verifying_key().to_bytes(),
		}
	}

	pub fn from_seed(kid: KeyId, seed: [u8; PRIVATE_KEY_BYTES]) -> Self {
		let key = DalekSigningKey::from_bytes(&seed);
		Self {
			kid,
			seed: Zeroizing::new(seed),
			public_key: key.verifying_key().to_bytes(),
		}
	}

	pub fn kid(&self) -> KeyId {
		self.kid
	}

	pub fn seed(&self) -> &[u8; PRIVATE_KEY_BYTES] {
		&self.seed
	}

	pub fn public_key(&self) -> [u8; PUBLIC_KEY_BYTES] {
		self.public_key
	}

	pub fn verification_key(&self) -> VerificationKey {
		VerificationKey {
			kid: self.kid,
			public_key: self.public_key,
		}
	}

	pub(crate) fn encoding_key(&self) -> Result<EncodingKey, TokenError> {
		let key = DalekSigningKey::from_bytes(&self.seed);
		let document = key.to_pkcs8_der().map_err(|_| TokenError::InvalidKey)?;
		Ok(EncodingKey::from_ed_der(document.as_bytes()))
	}
}

impl fmt::Debug for SigningKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SigningKey")
			.field("kid", &self.kid)
			.field("seed", &"[redacted]")
			.field("public_key", &self.public_key)
			.finish()
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationKey {
	kid: KeyId,
	public_key: [u8; PUBLIC_KEY_BYTES],
}

impl VerificationKey {
	pub fn new(kid: KeyId, public_key: [u8; PUBLIC_KEY_BYTES]) -> Self {
		Self { kid, public_key }
	}

	pub fn kid(&self) -> KeyId {
		self.kid
	}

	pub fn public_key(&self) -> [u8; PUBLIC_KEY_BYTES] {
		self.public_key
	}

	pub(crate) fn decoding_key(&self) -> DecodingKey {
		// Despite the constructor name, jsonwebtoken's EdDSA implementation accepts the raw
		// 32-byte Ed25519 public key here. Codec tests exercise this representation end to end.
		DecodingKey::from_ed_der(&self.public_key)
	}
}
