//! Volume groups for the namespace API: create, fork and archive, each a few idempotent
//! commands in order on the group volumes and an event shard. The caller persists its
//! operation id before a fork and re-drives the same call after a crash; every step dedupes
//! on it, so the retry completes the same fork.

use crate::{Client, Overloaded, overload_reason_label};
use anyhow::{Context, Result};
use depot_protocol::command::{
	EventPost, GroupArchive, GroupChildArchived, GroupChildCreate, GroupCreate, GroupForkDraw,
	GroupRead,
};
use depot_types::keys::partition::{EventShardVolume, GroupVolume};
use rivet_data::generated::depot_group_v1::{
	Event, EventGroupArchived, EventGroupForked, EventPostRequest, GroupChildArchivedRequest,
	GroupChildCreateRequest, GroupCreateRequest, GroupForkDrawRequest, GroupReadRequest,
	GroupRecord,
};
use rivet_data::generated::depot_kv_command_v1::OverloadReason;
use rivet_keys::keys::codec::GroupId;

/// How a group fork is made: the operation id every step dedupes on, when it happened, and
/// the limits the parent is checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupFork {
	pub parent: GroupId,
	pub child: GroupId,
	/// The child's own key bytes, `[GROUP, NAMESPACE, name]`, kept on its record.
	pub child_key: Vec<u8>,
	pub operation_id: [u8; 16],
	pub unix_ms: i64,
	pub max_lineage_depth: u8,
	/// The event shards of this region; the event is posted to one of them.
	pub event_shards: u16,
	/// The most rows an event shard keeps, `depot.events.retained_rows`.
	pub event_retained_rows: u64,
}

/// Whether a command failed because its volume does not exist, which a read of a group that
/// was never created reports.
pub fn is_volume_not_found(error: &anyhow::Error) -> bool {
	let extracted = rivet_error::RivetError::extract(error);
	extracted.group() == "depot" && extracted.code() == "volume_not_found"
}

/// The retry hint of a group read that found no record in a lagging snapshot. A closed
/// writer's last appends reach the volume's maintainer within the shards' notice window, about
/// half a second.
const LAGGING_GROUP_READ_RETRY_AFTER_MS: u64 = 500;

/// The shard an operation's event is posted to: any shard will do, so the operation id
/// spreads them.
fn shard_of(operation_id: &[u8; 16], shards: u16) -> u64 {
	u64::from(u16::from_le_bytes([operation_id[0], operation_id[1]]) % shards.max(1))
}

impl Client {
	/// Creates `group` as a root. An existing root is left as it is.
	/// Creates the root group `group`, whose key bytes `key` hash to it, once.
	pub async fn create_group(&self, group: GroupId, key: Vec<u8>) -> Result<GroupRecord> {
		let response = self
			.command::<GroupCreate>(
				&GroupVolume::new(group).volume_key(),
				GroupCreateRequest { key: key.into() },
			)
			.await
			.context("creating the group record")?;
		Ok(response.record)
	}

	/// The record of `group`, or `None` when its volume holds none or does not exist. A read
	/// with no writer may answer from a snapshot behind the volume's commit; one that found no
	/// record there cannot prove the record absent, so it is refused as `volume_transition`
	/// with a retry hint instead.
	pub async fn read_group(&self, group: GroupId) -> Result<Option<GroupRecord>> {
		let response = self
			.command::<GroupRead>(
				&GroupVolume::new(group).volume_key(),
				GroupReadRequest {
					fork_numbers: Vec::new(),
				},
			)
			.await;
		match response {
			Ok(response) if response.record.is_none() && response.commit_lag > 0 => {
				Err(Overloaded {
					reason: overload_reason_label(OverloadReason::VolumeTransition).to_owned(),
					retry_after_ms: LAGGING_GROUP_READ_RETRY_AFTER_MS,
				}
				.build())
			}
			Ok(response) => Ok(response.record),
			Err(error) if is_volume_not_found(&error) => Ok(None),
			Err(error) => Err(error.context("reading the group record")),
		}
	}

	/// Forks `fork.parent` to `fork.child`: the parent draws the fork number, the event is
	/// posted, the child's record is written. Refused as the parent's draw is; a crash
	/// between the steps is finished by calling again with the same operation id.
	pub async fn fork_group(&self, fork: &GroupFork) -> Result<GroupRecord> {
		let drawn = self
			.command::<GroupForkDraw>(
				&GroupVolume::new(fork.parent).volume_key(),
				GroupForkDrawRequest {
					child: fork.child,
					operation_id: fork.operation_id,
					unix_ms: fork.unix_ms,
					max_lineage_depth: fork.max_lineage_depth,
				},
			)
			.await
			.context("drawing the fork number on the parent group")?;
		self.post_event(
			shard_of(&fork.operation_id, fork.event_shards),
			fork.event_retained_rows,
			Event::EventGroupForked(EventGroupForked {
				parent: fork.parent,
				child: fork.child,
				fork_number: drawn.fork_number,
			}),
		)
		.await?;
		let created = self
			.command::<GroupChildCreate>(
				&GroupVolume::new(fork.child).volume_key(),
				GroupChildCreateRequest {
					key: fork.child_key.clone().into(),
					parent: fork.parent,
					fork_number: drawn.fork_number,
					root: drawn.root,
					depth: drawn.depth.saturating_add(1),
				},
			)
			.await
			.context("writing the child group's record")?;
		Ok(created.record)
	}

	/// Archives `group` for good, once its children are archived: the status, the event, and
	/// the parent's fork child row. Calling again on an archived group repeats the last two
	/// steps, which a crash between them needs.
	pub async fn archive_group(
		&self,
		group: GroupId,
		event_shards: u16,
		event_retained_rows: u64,
	) -> Result<GroupRecord> {
		let archived = self
			.command::<GroupArchive>(&GroupVolume::new(group).volume_key(), ())
			.await
			.context("archiving the group")?;
		self.post_event(
			shard_of(&group, event_shards),
			event_retained_rows,
			Event::EventGroupArchived(EventGroupArchived { group }),
		)
		.await?;
		if let Some(base) = &archived.record.base {
			self.command::<GroupChildArchived>(
				&GroupVolume::new(base.parent).volume_key(),
				GroupChildArchivedRequest {
					fork_number: base.fork_number,
					child: group,
				},
			)
			.await
			.context("marking the parent's fork child archived")?;
		}
		Ok(archived.record)
	}

	async fn post_event(&self, shard: u64, retained_rows: u64, event: Event) -> Result<u64> {
		let posted = self
			.command::<EventPost>(
				&EventShardVolume::new(shard).volume_key(),
				EventPostRequest {
					event,
					retained_rows,
				},
			)
			.await
			.context("posting the group event")?;
		Ok(posted.sequence)
	}
}

/// Whether `record`'s group has a fork child that is not archived. The record counts them in
/// `liveChildren` under the group's write lock, so this needs no fork child row, however many
/// forks the group ever made.
pub fn has_live_forks(record: &GroupRecord) -> bool {
	record.live_children > 0
}
