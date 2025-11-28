use anyhow::Result;
use gas::prelude::*;
use rivet_types::keys::namespace::runner_config::RunnerConfigVariant;
use serde::{Deserialize, Serialize};
use universaldb::prelude::*;
use vbare::OwnedVersionedData;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActorNamesMetadata {
	pub actor_names: Vec<(String, serde_json::Map<String, serde_json::Value>)>,
	pub fetched_at: i64,
}

#[derive(Debug)]
pub struct DataKey {
	pub namespace_id: Id,
	pub name: String,
}

impl DataKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		DataKey { namespace_id, name }
	}

	pub fn subspace(namespace_id: Id) -> DataSubspaceKey {
		DataSubspaceKey::new(namespace_id)
	}
}

impl FormalKey for DataKey {
	type Value = rivet_types::runner_configs::RunnerConfig;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(
			rivet_data::versioned::NamespaceRunnerConfig::deserialize_with_embedded_version(raw)?
				.into(),
		)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_data::versioned::NamespaceRunnerConfig::wrap_latest(value.into())
			.serialize_with_embedded_version(rivet_data::PEGBOARD_NAMESPACE_RUNNER_CONFIG_VERSION)
	}
}

impl TuplePack for DataKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, CONFIG, DATA, self.namespace_id, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for DataKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, name)) =
			<(usize, usize, usize, Id, String)>::unpack(input, tuple_depth)?;

		let v = DataKey { namespace_id, name };

		Ok((input, v))
	}
}

pub struct DataSubspaceKey {
	pub namespace_id: Id,
}

impl DataSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		DataSubspaceKey { namespace_id }
	}
}

impl TuplePack for DataSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (RUNNER, CONFIG, DATA, self.namespace_id);
		offset += t.pack(w, tuple_depth)?;

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct ByVariantKey {
	pub namespace_id: Id,
	pub variant: RunnerConfigVariant,
	pub name: String,
}

impl ByVariantKey {
	pub fn new(namespace_id: Id, variant: RunnerConfigVariant, name: String) -> Self {
		ByVariantKey {
			namespace_id,
			name,
			variant,
		}
	}

	pub fn subspace(namespace_id: Id) -> ByVariantSubspaceKey {
		ByVariantSubspaceKey::new(namespace_id)
	}

	pub fn subspace_with_variant(
		namespace_id: Id,
		variant: RunnerConfigVariant,
	) -> ByVariantSubspaceKey {
		ByVariantSubspaceKey::new_with_variant(namespace_id, variant)
	}
}

impl FormalKey for ByVariantKey {
	type Value = rivet_types::runner_configs::RunnerConfig;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(
			rivet_data::versioned::NamespaceRunnerConfig::deserialize_with_embedded_version(raw)?
				.into(),
		)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_data::versioned::NamespaceRunnerConfig::wrap_latest(value.into())
			.serialize_with_embedded_version(rivet_data::PEGBOARD_NAMESPACE_RUNNER_CONFIG_VERSION)
	}
}

impl TuplePack for ByVariantKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			RUNNER,
			CONFIG,
			BY_VARIANT,
			self.namespace_id,
			self.variant as usize,
			&self.name,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ByVariantKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, variant, name)) =
			<(usize, usize, usize, Id, usize, String)>::unpack(input, tuple_depth)?;
		let variant = RunnerConfigVariant::from_repr(variant).ok_or_else(|| {
			PackError::Message(format!("invalid runner config variant `{variant}` in key").into())
		})?;

		let v = ByVariantKey {
			namespace_id,
			variant,
			name,
		};

		Ok((input, v))
	}
}

pub struct ByVariantSubspaceKey {
	pub namespace_id: Id,
	pub variant: Option<RunnerConfigVariant>,
}

impl ByVariantSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		ByVariantSubspaceKey {
			namespace_id,
			variant: None,
		}
	}

	pub fn new_with_variant(namespace_id: Id, variant: RunnerConfigVariant) -> Self {
		ByVariantSubspaceKey {
			namespace_id,
			variant: Some(variant),
		}
	}
}

impl TuplePack for ByVariantSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (RUNNER, CONFIG, BY_VARIANT, self.namespace_id);
		offset += t.pack(w, tuple_depth)?;

		if let Some(variant) = self.variant {
			offset += (variant as usize).pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}
