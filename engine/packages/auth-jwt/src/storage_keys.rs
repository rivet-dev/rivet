use anyhow::Result;
use universaldb::prelude::*;

use crate::{SigningKeyRing, decode_signing_key_ring, encode_signing_key_ring};

pub fn subspace() -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, AUTH, JWT))
}

#[derive(Debug, Clone, Copy)]
pub struct SigningKeyRingKey;

impl FormalKey for SigningKeyRingKey {
	type Value = SigningKeyRing;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		decode_signing_key_ring(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		encode_signing_key_ring(&value)
	}
}

impl TuplePack for SigningKeyRingKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		KEY_RING.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SigningKeyRingKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, value) = usize::unpack(input, tuple_depth)?;
		if value != KEY_RING {
			return Err(PackError::Message(
				"expected JWT signing-key-ring key".into(),
			));
		}
		Ok((input, Self))
	}
}

#[cfg(test)]
#[path = "../tests/unit/storage_keys.rs"]
mod tests;
