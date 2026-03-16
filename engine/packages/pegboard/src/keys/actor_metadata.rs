use anyhow::Result;
use gas::prelude::*;
use universaldb::prelude::*;

const ACTOR_METADATA: &str = "actor_metadata";

pub fn subspace(actor_id: Id) -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(ACTOR_METADATA, actor_id))
}

#[derive(Debug)]
pub struct EntryKey {
	pub actor_id: Id,
	pub key: String,
}

impl EntryKey {
	pub fn new(actor_id: Id, key: String) -> Self {
		Self { actor_id, key }
	}
}

impl FormalKey for EntryKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(String::from_utf8(raw.to_vec())?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for EntryKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		(ACTOR_METADATA, self.actor_id, &self.key).pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for EntryKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (subspace, actor_id, key)) =
			<(String, Id, String)>::unpack(input, tuple_depth)?;
		if subspace != ACTOR_METADATA {
			return Err(PackError::Message("expected actor metadata key".into()));
		}

		Ok((input, Self { actor_id, key }))
	}
}
