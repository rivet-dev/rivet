use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use serde::Serialize;
use universaldb::prelude::*;
use utoipa::ToSchema;
use versioned_data_util::OwnedVersionedData;

pub fn subspace() -> universaldb::utils::Subspace {
	universaldb::utils::Subspace::new(&(RIVET, NAMESPACE))
}

#[derive(Debug)]
pub struct NameKey {
	namespace_id: Id,
}

impl NameKey {
	pub fn new(namespace_id: Id) -> Self {
		NameKey { namespace_id }
	}

	pub fn namespace_id(&self) -> Id {
		self.namespace_id
	}
}

impl FormalKey for NameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for NameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DATA, self.namespace_id, NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = NameKey { namespace_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct DisplayNameKey {
	namespace_id: Id,
}

impl DisplayNameKey {
	pub fn new(namespace_id: Id) -> Self {
		DisplayNameKey { namespace_id }
	}
}

impl FormalKey for DisplayNameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for DisplayNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DATA, self.namespace_id, DISPLAY_NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for DisplayNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = DisplayNameKey { namespace_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	namespace_id: Id,
}

impl CreateTsKey {
	pub fn new(namespace_id: Id) -> Self {
		CreateTsKey { namespace_id }
	}
}

impl FormalKey for CreateTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for CreateTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DATA, self.namespace_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _)) = <(usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { namespace_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ByNameKey {
	name: String,
}

impl ByNameKey {
	pub fn new(name: String) -> Self {
		ByNameKey { name }
	}
}

impl FormalKey for ByNameKey {
	/// Namespace id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for ByNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (BY_NAME, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ByNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, name)) = <(usize, String)>::unpack(input, tuple_depth)?;

		let v = ByNameKey { name };

		Ok((input, v))
	}
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, strum::FromRepr, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerConfigVariant {
	Serverless = 0,
}

impl RunnerConfigVariant {
	pub fn parse(v: &str) -> Option<Self> {
		match v {
			"serverless" => Some(RunnerConfigVariant::Serverless),
			_ => None,
		}
	}
}

impl std::fmt::Display for RunnerConfigVariant {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RunnerConfigVariant::Serverless => write!(f, "serverless"),
		}
	}
}

#[derive(Debug)]
pub struct RunnerConfigKey {
	pub namespace_id: Id,
	pub name: String,
}

impl RunnerConfigKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		RunnerConfigKey { namespace_id, name }
	}

	pub fn subspace(namespace_id: Id) -> RunnerConfigSubspaceKey {
		RunnerConfigSubspaceKey::new(namespace_id)
	}
}

impl FormalKey for RunnerConfigKey {
	type Value = rivet_types::namespaces::RunnerConfig;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(
			rivet_data::versioned::NamespaceRunnerConfig::deserialize_with_embedded_version(raw)?
				.into(),
		)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_data::versioned::NamespaceRunnerConfig::latest(value.into())
			.serialize_with_embedded_version(
				rivet_data::PEGBOARD_NAMESPACE_RUNNER_ALLOC_IDX_VERSION,
			)
	}
}

impl TuplePack for RunnerConfigKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, CONFIG, DATA, self.namespace_id, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerConfigKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, name)) =
			<(usize, usize, usize, Id, String)>::unpack(input, tuple_depth)?;

		let v = RunnerConfigKey { namespace_id, name };

		Ok((input, v))
	}
}

pub struct RunnerConfigSubspaceKey {
	pub namespace_id: Id,
}

impl RunnerConfigSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		RunnerConfigSubspaceKey { namespace_id }
	}
}

impl TuplePack for RunnerConfigSubspaceKey {
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
pub struct RunnerConfigByVariantKey {
	pub namespace_id: Id,
	pub variant: RunnerConfigVariant,
	pub name: String,
}

impl RunnerConfigByVariantKey {
	pub fn new(namespace_id: Id, variant: RunnerConfigVariant, name: String) -> Self {
		RunnerConfigByVariantKey {
			namespace_id,
			name,
			variant,
		}
	}

	pub fn subspace(namespace_id: Id) -> RunnerConfigByVariantSubspaceKey {
		RunnerConfigByVariantSubspaceKey::new(namespace_id)
	}

	pub fn subspace_with_variant(
		namespace_id: Id,
		variant: RunnerConfigVariant,
	) -> RunnerConfigByVariantSubspaceKey {
		RunnerConfigByVariantSubspaceKey::new_with_variant(namespace_id, variant)
	}
}

impl FormalKey for RunnerConfigByVariantKey {
	type Value = rivet_types::namespaces::RunnerConfig;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(
			rivet_data::versioned::NamespaceRunnerConfig::deserialize_with_embedded_version(raw)?
				.into(),
		)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_data::versioned::NamespaceRunnerConfig::latest(value.into())
			.serialize_with_embedded_version(
				rivet_data::PEGBOARD_NAMESPACE_RUNNER_ALLOC_IDX_VERSION,
			)
	}
}

impl TuplePack for RunnerConfigByVariantKey {
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

impl<'de> TupleUnpack<'de> for RunnerConfigByVariantKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, variant, name)) =
			<(usize, usize, usize, Id, usize, String)>::unpack(input, tuple_depth)?;
		let variant = RunnerConfigVariant::from_repr(variant).ok_or_else(|| {
			PackError::Message(format!("invalid runner config variant `{variant}` in key").into())
		})?;

		let v = RunnerConfigByVariantKey {
			namespace_id,
			variant,
			name,
		};

		Ok((input, v))
	}
}

pub struct RunnerConfigByVariantSubspaceKey {
	pub namespace_id: Id,
	pub variant: Option<RunnerConfigVariant>,
}

impl RunnerConfigByVariantSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		RunnerConfigByVariantSubspaceKey {
			namespace_id,
			variant: None,
		}
	}

	pub fn new_with_variant(namespace_id: Id, variant: RunnerConfigVariant) -> Self {
		RunnerConfigByVariantSubspaceKey {
			namespace_id,
			variant: Some(variant),
		}
	}
}

impl TuplePack for RunnerConfigByVariantSubspaceKey {
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
