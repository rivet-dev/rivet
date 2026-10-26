use crate::{
	keys::s3::WorkerHeartbeatKey,
	worker::{WorkerId, WorkerRole},
};
use object_store::{ObjectStore, ObjectStoreExt};
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use serde::{Deserialize, Serialize};

/// Persisted wire states: Active is eligible for placement, while Draining excludes new work
/// but keeps renewing. Released follows safe cleanup and allows immediate conditional takeover.
pub use rivet_data::generated::topology_worker_heartbeat_v1::HeartbeatState;

/// Versioned persisted payload used by heartbeat reads and conditional writes.
/// Incarnation and write sequence identify confirmations; diagnostics never establish expiration.
pub type Heartbeat = rivet_data::generated::topology_worker_heartbeat_v1::Data;

/// The scope of the exclusive lock a storage process holds on its log directory, which a later
/// process of the same worker compares to prove that process exited. See `WorkerRegistration`.
pub type DirectoryLock = rivet_data::generated::topology_worker_heartbeat_v1::DirectoryLock;

/// Result of `read_heartbeat`, also used for executor timeout responses.
/// Lease handlers distinguish missing evidence from failed I/O before deciding to claim or fence.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Read {
	/// Decoded exact-key evidence with an ETag; callers still validate incarnation and freshness.
	Found(Observation),
	/// The exact-key GET returned NotFound; registration may create, but a live or releasing lease fences.
	Missing,
	/// The store was unreachable or the value was undecodable. Never treat this as missing.
	Unknown,
}

/// Every outcome other than `Confirmed` is resolved by reading the heartbeat back.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Write {
	/// Store-confirmed ETag used by the next CAS; the original send time still bounds the lease.
	Confirmed(String),
	/// The CAS precondition failed. The executor currently reports this as `Unknown`.
	Conflict,
	/// The write may or may not have landed.
	Unknown,
}

/// Exact-key heartbeat evidence. Absence from a storage listing is not evidence
/// that a former owner expired; ownership protocols must read its heartbeat.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Observation {
	pub worker_id: crate::worker::WorkerId,
	/// Namespace actually read; role is not carried in the heartbeat payload.
	pub role: WorkerRole,
	pub heartbeat: Heartbeat,
	/// Matches a confirmed write or becomes the precondition for replacing this observation.
	pub etag: String,
	/// Store-clock sample from `Last-Modified`.
	pub modified_unix_ms: i64,
}

impl Observation {
	pub(crate) fn worker(&self) -> crate::worker::Worker {
		crate::worker::Worker {
			storage_capacity_millicores: self.heartbeat.storage_capacity_millicores,
			id: self.worker_id.clone(),
			instance_id: crate::worker::WorkerInstanceId::from_bytes(self.heartbeat.instance_id),
			api_peer_address: self.heartbeat.api_peer_address.to_string(),
			role: self.role,
			versions: self.heartbeat.versions.as_slice().into(),
		}
	}
}

/// Object-store time estimate used to expire discovery entries without trusting local wall time.
/// Live workers anchor on their confirmed heartbeat; observers and registering workers waiting
/// out a previous process's lease anchor on the newest listed timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoreClock {
	/// Store time known to have passed by `at_boot_ms`.
	pub lower_unix_ms: i64,
	/// Monotonic instant of the anchor; `lower_bound` advances from here.
	pub at_boot_ms: u64,
}

impl StoreClock {
	pub fn lower_bound(&self, now_boot_ms: u64) -> i64 {
		(self.lower_unix_ms as i128 + now_boot_ms.saturating_sub(self.at_boot_ms) as i128)
			.clamp(i64::MIN as i128, i64::MAX as i128) as i64
	}
}

/// Exact-key read for local lease validation, discovery, and caller inspections. Missing and
/// unreadable are distinct: an outage cannot be treated as an expired owner.
pub async fn read_heartbeat(store: &dyn ObjectStore, role: WorkerRole, id: &WorkerId) -> Read {
	let key = WorkerHeartbeatKey::new(role, id.clone());

	let result = match store.get(&key.pack()).await {
		Ok(result) => result,
		Err(object_store::Error::NotFound { .. }) => return Read::Missing,
		Err(_) => return Read::Unknown,
	};

	let Some(etag) = result.meta.e_tag.clone() else {
		return Read::Unknown;
	};

	let modified_unix_ms = result.meta.last_modified.timestamp_millis();

	let value = async { Ok::<_, anyhow::Error>(key.deserialize(result.bytes().await?)?) }.await;

	match value {
		Ok(heartbeat) => Read::Found(Observation {
			worker_id: id.clone(),
			role,
			heartbeat,
			etag,
			modified_unix_ms,
		}),
		Err(_) => Read::Unknown,
	}
}
