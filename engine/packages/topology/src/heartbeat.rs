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
pub type Heartbeat = rivet_data::generated::topology_worker_heartbeat_v2::Heartbeat;

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

/// Conditional conflicts trigger readback; uncertain renewals retain their original send
/// times until a later write or readback confirms them.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Write {
	/// Store-confirmed ETag used by the next CAS; the original send time still bounds the lease.
	Confirmed(String),
	/// The conditional create or update failed; read back to identify the stored write.
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
			zone: self.heartbeat.zone.as_deref().map(str::to_owned),
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
	match read_heartbeat_detail(store, role, id).await {
		HeartbeatRead::Found(observation) => Read::Found(observation),
		HeartbeatRead::Missing => Read::Missing,
		HeartbeatRead::Unreachable | HeartbeatRead::Unusable(_) => Read::Unknown,
	}
}

/// Result of `read_heartbeat_detail`: a [`Read`] that also tells apart why a heartbeat that
/// exists is not an observation. Storage discovery needs that to judge a worker it never
/// observed; lease validation reads every such case as [`Read::Unknown`].
pub(crate) enum HeartbeatRead {
	Found(Observation),
	Missing,
	/// The GET failed, so nothing is known about the heartbeat.
	Unreachable,
	/// The heartbeat exists but gives no observation: it has no ETag, or its body does not
	/// decode. A version header above the newest version this binary reads is undecodable too:
	/// the heartbeat schema ships a new version only after every reader decodes it, so no
	/// supported writer writes one, and such a header means corruption or an unsupported deploy
	/// skip, never a newer worker that reads what this binary writes.
	Unusable(UnobservedReason),
}

/// Why a storage listing could not observe a listed worker it never observed before. The
/// variants that read the heartbeat carry its `Last-Modified`, the lease record's renewal
/// time, which shows once the worker can no longer be serving.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnobservedReason {
	/// The heartbeat GET failed or timed out.
	Unreadable,
	/// The heartbeat body does not decode, including a version header above the newest version
	/// this binary reads.
	Undecodable { modified_unix_ms: i64 },
	/// The store answered the heartbeat without an ETag.
	NoEtag { modified_unix_ms: i64 },
}

impl UnobservedReason {
	/// The bounded `reason` label of `topology_unobserved_workers_total`.
	pub fn label(&self) -> &'static str {
		match self {
			Self::Unreadable => "read",
			Self::Undecodable { .. } => "decode",
			Self::NoEtag { .. } => "etag",
		}
	}

	/// The heartbeat's `Last-Modified`, when the GET returned one.
	pub fn modified_unix_ms(&self) -> Option<i64> {
		match self {
			Self::Unreadable => None,
			Self::Undecodable { modified_unix_ms } | Self::NoEtag { modified_unix_ms } => {
				Some(*modified_unix_ms)
			}
		}
	}
}

/// A listed storage worker that no listing has observed, and why.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnobservedHeartbeat {
	pub worker_id: WorkerId,
	pub reason: UnobservedReason,
}

/// See [`HeartbeatRead`].
pub(crate) async fn read_heartbeat_detail(
	store: &dyn ObjectStore,
	role: WorkerRole,
	id: &WorkerId,
) -> HeartbeatRead {
	let key = WorkerHeartbeatKey::new(role, id.clone());

	let result = match store.get(&key.pack()).await {
		Ok(result) => result,
		Err(object_store::Error::NotFound { .. }) => return HeartbeatRead::Missing,
		Err(_) => return HeartbeatRead::Unreachable,
	};

	let modified_unix_ms = result.meta.last_modified.timestamp_millis();
	let etag = result.meta.e_tag.clone();
	let bytes = match result.bytes().await {
		Ok(bytes) => bytes,
		Err(_) => return HeartbeatRead::Unreachable,
	};
	let Some(etag) = etag else {
		return HeartbeatRead::Unusable(UnobservedReason::NoEtag { modified_unix_ms });
	};

	match key.deserialize(bytes) {
		Ok(heartbeat) => HeartbeatRead::Found(Observation {
			worker_id: id.clone(),
			role,
			heartbeat,
			etag,
			modified_unix_ms,
		}),
		Err(_) => HeartbeatRead::Unusable(UnobservedReason::Undecodable { modified_unix_ms }),
	}
}
