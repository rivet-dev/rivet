use std::fmt;

use anyhow::Result;
use uuid::Uuid;

/// Fixed for a worker ID's lifetime. Draining never changes its role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WorkerRole {
	/// Listed for storage discovery and tablet placement.
	Storage,
	/// Runs cluster services without storage. Never list this high-cardinality group;
	/// resolve stateless workers by exact worker ID.
	Stateless,
}

impl rivet_keys::keys::codec::TextComponent for WorkerRole {
	fn to_text(&self) -> String {
		match self {
			Self::Storage => "storage".into(),
			Self::Stateless => "stateless".into(),
		}
	}

	fn from_text(text: &str) -> anyhow::Result<Self> {
		match text {
			"storage" => Ok(Self::Storage),
			"stateless" => Ok(Self::Stateless),
			_ => anyhow::bail!("unknown worker role"),
		}
	}
}

/// The stable worker identity shared by cluster services, distinct from an Epoxy regional PeerId.
/// Stateless startup draws a UUID; storage startup restores it from the owned log directory.
#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct WorkerId(Uuid);

impl WorkerId {
	/// Parses a UUID at a textual transport boundary.
	pub fn new(id: impl AsRef<str>) -> Result<Self> {
		Ok(Self(Uuid::parse_str(id.as_ref())?))
	}

	/// Reconstructs the shared topology identity from its binary representation.
	pub fn from_bytes(bytes: [u8; 16]) -> Self {
		Self(Uuid::from_bytes(bytes))
	}

	/// Binary storage and wire schemas carry these bytes, without UUID text encoding.
	pub fn as_bytes(&self) -> &[u8; 16] {
		self.0.as_bytes()
	}
}

impl TryFrom<String> for WorkerId {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self> {
		Self::new(value)
	}
}

impl From<WorkerId> for String {
	fn from(value: WorkerId) -> Self {
		value.0.to_string()
	}
}

impl fmt::Display for WorkerId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl rivet_keys::keys::codec::TextComponent for WorkerId {
	fn to_text(&self) -> String {
		self.to_string()
	}

	fn from_text(text: &str) -> Result<Self> {
		let id = Self::new(text)?;
		anyhow::ensure!(id.to_string() == text, "noncanonical worker ID component");
		Ok(id)
	}
}

/// Identifies one process lifetime independently of the worker's routing identity.
#[derive(
	Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct WorkerInstanceId(Uuid);

impl WorkerInstanceId {
	pub fn from_bytes(bytes: [u8; 16]) -> Self {
		Self(Uuid::from_bytes(bytes))
	}

	pub fn as_bytes(&self) -> &[u8; 16] {
		self.0.as_bytes()
	}
}

impl From<Uuid> for WorkerInstanceId {
	fn from(value: Uuid) -> Self {
		Self(value)
	}
}

impl From<WorkerInstanceId> for Uuid {
	fn from(value: WorkerInstanceId) -> Self {
		value.0
	}
}

/// Created once at process startup and shared with storage and messaging.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorkerIdentity {
	/// Selected at startup and shared by every service in this worker process.
	pub worker_id: WorkerId,
	/// Separate UUID generated on every worker-process boot for lease fencing.
	pub instance_id: WorkerInstanceId,
}

impl WorkerIdentity {
	pub fn new(rng: &mut rivet_env::RngStream) -> Self {
		Self {
			worker_id: WorkerId(rng.uuid_v4()),
			instance_id: rng.uuid_v4().into(),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Worker {
	/// Positive static storage placement weight; 1000 represents one allocated CPU core.
	pub storage_capacity_millicores: u32,
	pub id: WorkerId,
	pub instance_id: WorkerInstanceId,
	/// Address for direct requests to this worker, including stateless workers.
	pub api_peer_address: String,
	/// Placement eligibility; a storage worker may currently own zero tablets.
	pub role: WorkerRole,
	/// Per-family version ranges from the worker's heartbeat, fixed for its incarnation.
	pub versions: std::sync::Arc<[crate::versions::FamilyVersions]>,
}

/// Worker-addressed formal keys retain the UUID tuple component rather than its display text.
impl rivet_keys::keys::codec::BinaryComponent for WorkerId {
	fn pack_into(&self, out: &mut Vec<u8>) {
		rivet_keys::keys::codec::BinaryComponent::pack_into(&self.0, out);
	}

	fn unpack_from(
		input: &mut &[u8],
		depth: foundationdb_tuple::TupleDepth,
	) -> foundationdb_tuple::PackResult<Self> {
		Ok(Self(
			<Uuid as rivet_keys::keys::codec::BinaryComponent>::unpack_from(input, depth)?,
		))
	}
}
