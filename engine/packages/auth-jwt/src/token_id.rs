use std::{fmt, str::FromStr};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{CryptoRng, RngCore};

use crate::TokenError;

pub const TOKEN_ID_BYTES: usize = 16;

/// Random identifier for one issued JWT. This is not a cryptographic key or a key selector.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenId([u8; TOKEN_ID_BYTES]);

impl TokenId {
	pub fn generate(rng: &mut (impl CryptoRng + RngCore)) -> Self {
		let mut bytes = [0; TOKEN_ID_BYTES];
		rng.fill_bytes(&mut bytes);
		Self(bytes)
	}

	pub fn from_bytes(bytes: [u8; TOKEN_ID_BYTES]) -> Self {
		Self(bytes)
	}
}

impl fmt::Debug for TokenId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

impl fmt::Display for TokenId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&URL_SAFE_NO_PAD.encode(self.0))
	}
}

impl FromStr for TokenId {
	type Err = TokenError;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.contains('=') {
			return Err(TokenError::InvalidClaims);
		}

		let decoded = URL_SAFE_NO_PAD
			.decode(value)
			.map_err(|_| TokenError::InvalidClaims)?;
		let bytes = decoded.try_into().map_err(|_| TokenError::InvalidClaims)?;
		let token_id = Self(bytes);
		if token_id.to_string() != value {
			return Err(TokenError::InvalidClaims);
		}

		Ok(token_id)
	}
}
