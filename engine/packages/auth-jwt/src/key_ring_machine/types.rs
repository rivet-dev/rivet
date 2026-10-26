//! Serializable I/O values. Ring bytes always use the generated v1 BARE schema.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Slot;
impl rivet_state_machine::Slot for Slot {}
pub type OpId = rivet_state_machine::OpId<Slot>;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoredRing {
	pub bytes: Vec<u8>,
	pub etag: String,
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mutation {
	pub bytes: Vec<u8>,
	pub expected_etag: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReadResult {
	Found(StoredRing),
	Missing,
	Unknown(String),
	Invalid(String),
}
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Material {
	pub kid: [u8; 16],
	pub seed: [u8; 32],
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Settings {
	pub epoch: u64,
	pub datacenter_id: u16,
	pub issuer: String,
	pub audience: String,
	pub policy: crate::RotationPolicy,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Timer {
	Io(rivet_state_machine_util::IoTimer<OpId>),
	Publication(rivet_object_store_util::conditional_write::Timer<OpId>),
	Wake,
}

impl std::fmt::Debug for StoredRing {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("StoredRing")
			.field("etag", &self.etag)
			.finish_non_exhaustive()
	}
}
impl std::fmt::Debug for Mutation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Mutation")
			.field("expected_etag", &self.expected_etag)
			.finish_non_exhaustive()
	}
}
impl std::fmt::Debug for Material {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Material")
			.field("kid", &self.kid)
			.finish_non_exhaustive()
	}
}

pub use rivet_data::generated::auth_jwt_rotation_v1::{
	Committed, EmergencyRotateRequest, EmergencyRotateResponse, Rejected,
};

/// One logical command survives key generation, conditional writes and uncertain readback.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmergencyCall {
	pub call: u64,
	pub request: EmergencyRotateRequest,
}
