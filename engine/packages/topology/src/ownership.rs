//! Safe ownership takeover evidence shared by storage grants and singleton coordination.
//! This extracts the former Depot owner check; it neither registers workers nor creates another lease.
use serde::{Deserialize, Serialize};

use crate::{
	heartbeat::{HeartbeatState, Read},
	worker::{WorkerIdentity, WorkerRole},
};

/// A missing exact heartbeat must remain unusable for a full lease plus takeover grace.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissingHeartbeatObservation {
	/// Reset the wait whenever the ownership record names a different process lifetime.
	pub identity: WorkerIdentity,
	/// Local monotonic observation time; never derived from an observer's wall clock.
	pub first_missing_boot_ms: u64,
}

/// True only when the exact previous owner can no longer serve under its worker lease.
/// Discovery absence and failed reads are never death evidence. The caller still conditionally
/// publishes ownership against the object version it inspected and checks its own live lease.
/// Capture `read_started_store_lower_unix_ms` BEFORE dispatch. Aging a returned snapshot using
/// response-delivery time can falsely expire an owner that renewed after the read linearized.
/// Do not reconstruct this value from a newer StoreClock anchor received while the read was pending.
pub fn observe_expiration(
	owner: &WorkerIdentity,
	role: WorkerRole,
	observation: &Read,
	now_boot_ms: u64,
	wait_ms: u64,
	read_started_store_lower_unix_ms: Option<i64>,
	missing: &mut Option<MissingHeartbeatObservation>,
) -> bool {
	match observation {
		Read::Unknown => false,
		Read::Missing => {
			let first = missing.get_or_insert_with(|| MissingHeartbeatObservation {
				identity: owner.clone(),
				first_missing_boot_ms: now_boot_ms,
			});
			if first.identity != *owner {
				*first = MissingHeartbeatObservation {
					identity: owner.clone(),
					first_missing_boot_ms: now_boot_ms,
				};
			}
			now_boot_ms.saturating_sub(first.first_missing_boot_ms) >= wait_ms
		}
		Read::Found(entry) => {
			*missing = None;
			if entry.worker_id != owner.worker_id || entry.role != role {
				return false;
			}
			entry.heartbeat.state == HeartbeatState::Released
				|| entry.heartbeat.instance_id != *owner.instance_id.as_bytes()
				|| read_started_store_lower_unix_ms.is_some_and(|lower_unix_ms| {
					i128::from(lower_unix_ms) - i128::from(entry.modified_unix_ms)
						>= i128::from(wait_ms)
				})
		}
	}
}
