//! Volume-owned alarm identity and ordered deadline encoding.
use rivet_data::generated::shared_depot_alarms_v1::{AlarmRevision, AlarmSummary};

/// Inode zero is reserved for Depot filesystem metadata, outside application KV inodes.
pub const COLD_ALARM_KEY: &[u8] = b"\0depot.alarm";

pub fn ordered_deadline(unix_ms: i64) -> u64 {
	(unix_ms as u64) ^ (1u64 << 63)
}

pub fn revision(incarnation: [u8; 16], summary: &AlarmSummary) -> Option<AlarmRevision> {
	summary.pending.as_ref().map(|pending| AlarmRevision {
		volume_incarnation: incarnation,
		arm_id: pending.arm_id,
	})
}

/// Index only the requested Unix deadline. Retry cooldowns may use an owner's boot
/// clock, so including them would make index order disagree with delivery order.
pub fn index_deadline(summary: &AlarmSummary) -> Option<i64> {
	summary
		.pending
		.as_ref()
		.map(|pending| pending.deadline_unix_ms)
}
