//! The read commands on a group volume and on an event shard. Both run on maintainer point
//! reads of the latest applied snapshot through a [`ReadHandle`], so a storage worker never
//! opens these volumes' writers; the group commands that write them are the namespace API's.

use super::{KvHandle, ReadHandle};
use crate::tablet_maintainer_machine::types::GROUP_READ_MAX_CHILDREN;
use anyhow::{Context, Result, ensure};
use depot_types::keys::kv::{
	EventRowKey, EventSeqKey, ForkChildKey, ForkOperationKey, GroupRecordKey,
};
use rivet_data::generated::depot_group_v1::{
	EventCounter, EventPostRequest, EventPostResponse, EventRow, EventsReadRequest,
	EventsReadResponse, ForkChild, ForkChildRow, ForkOperation, GroupArchiveRequest,
	GroupArchiveResponse, GroupBase, GroupChildArchivedRequest, GroupChildCreateRequest,
	GroupChildCreateResponse, GroupCreateRequest, GroupCreateResponse, GroupForkDrawRequest,
	GroupForkDrawResponse, GroupReadRequest, GroupReadResponse, GroupRecord, GroupStatus,
};
use rivet_keys::keys::{KeyAddress, SubspaceRange, codec::FormalKey};

#[derive(Debug, rivet_error::RivetError)]
#[error("group", "not_found", "The group does not exist.")]
pub struct GroupNotFound;

#[derive(Debug, rivet_error::RivetError)]
#[error("group", "exists", "A different group already exists with this id.")]
pub struct GroupExists;

#[derive(Debug, rivet_error::RivetError)]
#[error("group", "archived", "The group is archived and closed for good.")]
pub struct GroupArchived;

#[derive(Debug, rivet_error::RivetError)]
#[error("group", "has_children", "The group has a child that is not archived.")]
pub struct GroupHasChildren;

#[derive(Debug, rivet_error::RivetError)]
#[error(
	"group",
	"lineage_too_deep",
	"The group's lineage has reached the depth cap."
)]
pub struct GroupLineageTooDeep;

/// The most event rows one `EventsRead` returns.
pub const EVENTS_READ_MAX_ROWS: u32 = 256;

/// Reads the record and the requested fork child rows in one snapshot. A row that does not
/// exist is left out; a record that does not exist reads as `None`. The answer carries how far
/// the snapshot lagged the volume's commits, which decides whether a reader may cache it.
pub(super) async fn group_read(
	read: ReadHandle,
	request: GroupReadRequest,
) -> Result<GroupReadResponse> {
	ensure!(
		request.fork_numbers.len() <= GROUP_READ_MAX_CHILDREN,
		"a group read asks for at most {GROUP_READ_MAX_CHILDREN} fork children"
	);
	let record_key = GroupRecordKey::new();
	let mut keys = Vec::with_capacity(request.fork_numbers.len() + 1);
	keys.push(record_key.pack().to_vec());
	keys.extend(
		request
			.fork_numbers
			.iter()
			.map(|fork_number| ForkChildKey::new(*fork_number).pack().to_vec()),
	);
	let (values, commit_lag) = read.get_many_with_lag(&keys).await?;
	let mut values = values.into_iter();
	let record = values
		.next()
		.flatten()
		.map(|bytes| record_key.deserialize(bytes.into()))
		.transpose()
		.context("a group record row does not decode")?;
	let mut children = Vec::new();
	for (fork_number, value) in request.fork_numbers.iter().zip(values) {
		let Some(bytes) = value else {
			continue;
		};
		let child = ForkChildKey::new(*fork_number)
			.deserialize(bytes.into())
			.context("a fork child row does not decode")?;
		children.push(ForkChildRow {
			fork_number: *fork_number,
			child,
		});
	}
	Ok(GroupReadResponse {
		record,
		children,
		commit_lag,
	})
}

/// The counter of an event shard that holds none yet.
const EMPTY_EVENT_COUNTER: EventCounter = EventCounter { next: 0, oldest: 0 };

fn decode_event_counter(bytes: Option<Vec<u8>>) -> Result<EventCounter> {
	let Some(bytes) = bytes else {
		return Ok(EMPTY_EVENT_COUNTER);
	};
	EventSeqKey::new()
		.deserialize(bytes.into())
		.context("an event counter row does not decode")
}

/// Reads a page of an event shard from `first`, or the shard's tail when `first` is absent.
/// Rows from the counter's `oldest` on are numbered without gaps, since a post writes the
/// counter, its row and its trim in one commit, so the page stops at the first absent row. A
/// `first` below `oldest` reads no rows and answers `next` at `oldest`. The counter is read in
/// the same snapshot as the rows. A row this binary cannot decode is skipped and counted: a
/// newer kind is not for this reader.
pub(super) async fn events_read(
	read: ReadHandle,
	request: EventsReadRequest,
) -> Result<EventsReadResponse> {
	let counter_key = EventSeqKey::new().pack().to_vec();
	let Some(first) = request.first else {
		let counter = decode_event_counter(read.get(&counter_key).await?)?;
		return Ok(EventsReadResponse {
			rows: Vec::new(),
			next: counter.next,
			oldest: counter.oldest,
		});
	};
	let limit = u64::from(request.limit.clamp(1, EVENTS_READ_MAX_ROWS));
	let last = first.saturating_add(limit);
	let mut keys = Vec::with_capacity(limit as usize + 1);
	keys.push(counter_key);
	keys.extend((first..last).map(|sequence| EventRowKey::new(sequence).pack().to_vec()));
	let mut values = read.get_many(&keys).await?.into_iter();
	let counter = decode_event_counter(values.next().flatten())?;

	// The rows the reader asked for were trimmed: it resumes at the oldest row the shard holds.
	if first < counter.oldest {
		return Ok(EventsReadResponse {
			rows: Vec::new(),
			next: counter.oldest,
			oldest: counter.oldest,
		});
	}

	let mut rows = Vec::new();
	let mut next = first;
	for (sequence, value) in (first..last).zip(values) {
		let Some(bytes) = value else {
			break;
		};
		next = sequence + 1;
		match EventRowKey::new(sequence).deserialize(bytes.into()) {
			Ok(event) => rows.push(EventRow { sequence, event }),
			Err(error) => {
				crate::metrics::GROUP_EVENTS_SKIPPED.inc();
				tracing::debug!(
					?error,
					sequence,
					"skipping an event row this binary does not decode; a newer event kind"
				);
			}
		}
	}
	Ok(EventsReadResponse {
		rows,
		next,
		oldest: counter.oldest,
	})
}

/// Serializes the writes on one group volume. Every group write takes this lock before it
/// reads the record and holds it across its commit, so concurrent writes on one volume queue
/// in memory instead of conflicting at commit.
async fn lock_group(kv: &KvHandle) -> Result<super::RangeLockGuard> {
	kv.lock_partition(GroupRecordKey::new().range()).await
}

/// The stored record of a group volume, or `None` for a volume with no record.
async fn record(txn: &mut depot_client::filesystem::Transaction) -> Result<Option<GroupRecord>> {
	let key = GroupRecordKey::new();
	txn.get(&key.pack())
		.await?
		.map(|bytes| key.deserialize(bytes.into()))
		.transpose()
		.context("a group record row does not decode")
}

fn put_record(txn: &mut depot_client::filesystem::Transaction, record: &GroupRecord) -> Result<()> {
	let key = GroupRecordKey::new();
	txn.put(&key.pack(), &key.serialize(record.clone())?)?;
	Ok(())
}

/// Creates the root record of the group whose volume this is. The group id is the volume's,
/// which the caller derived from the group key; the volume key does not carry it back, so
/// the record's `root` is read from the request's volume by the caller's own id.
pub(super) async fn group_create(
	kv: KvHandle,
	request: GroupCreateRequest,
) -> Result<GroupCreateResponse> {
	let group = depot_types::keys::partition::GroupVolume::from_volume_key(kv.volume_key())
		.context("a group create runs on a group volume")?
		.group();
	let _guard = lock_group(&kv).await?;
	let mut txn = kv.begin().await?;
	if let Some(record) = record(&mut txn).await? {
		if record.base.is_some() {
			return Err(GroupExists.build());
		}
		return Ok(GroupCreateResponse {
			created: false,
			record,
		});
	}
	let record = GroupRecord {
		status: GroupStatus::Live,
		base: None,
		root: group,
		depth: 0,
		latest_fork: 0,
		key: request.key,
		live_children: 0,
	};
	put_record(&mut txn, &record)?;
	kv.commit(txn).await?;
	Ok(GroupCreateResponse {
		created: true,
		record,
	})
}

pub(super) async fn group_fork_draw(
	kv: KvHandle,
	request: GroupForkDrawRequest,
) -> Result<GroupForkDrawResponse> {
	let _guard = lock_group(&kv).await?;
	let mut txn = kv.begin().await?;
	let Some(mut record) = record(&mut txn).await? else {
		return Err(GroupNotFound.build());
	};
	if record.status == GroupStatus::Archived {
		return Err(GroupArchived.build());
	}
	let operation = ForkOperationKey::new(uuid::Uuid::from_bytes(request.operation_id));
	if let Some(drawn) = txn.get(&operation.pack()).await? {
		let drawn = operation
			.deserialize(drawn.into())
			.context("a fork operation row does not decode")?;
		return Ok(GroupForkDrawResponse {
			fork_number: drawn.fork_number,
			root: record.root,
			depth: record.depth,
		});
	}
	if record.depth >= request.max_lineage_depth.saturating_sub(1) {
		return Err(GroupLineageTooDeep.build());
	}
	let fork_number = record
		.latest_fork
		.checked_add(1)
		.context("fork numbers exhausted")?;
	record.latest_fork = fork_number;
	record.live_children = record
		.live_children
		.checked_add(1)
		.context("live fork children exhausted")?;
	put_record(&mut txn, &record)?;
	let child_key = ForkChildKey::new(fork_number);
	txn.put(
		&child_key.pack(),
		&child_key.serialize(ForkChild {
			child: request.child,
			operation_id: request.operation_id,
			unix_ms: request.unix_ms,
			archived: false,
		})?,
	)?;
	txn.put(
		&operation.pack(),
		&operation.serialize(ForkOperation { fork_number })?,
	)?;
	kv.commit(txn).await?;
	Ok(GroupForkDrawResponse {
		fork_number,
		root: record.root,
		depth: record.depth,
	})
}

pub(super) async fn group_child_create(
	kv: KvHandle,
	request: GroupChildCreateRequest,
) -> Result<GroupChildCreateResponse> {
	let base = GroupBase {
		parent: request.parent,
		fork_number: request.fork_number,
	};
	let _guard = lock_group(&kv).await?;
	let mut txn = kv.begin().await?;
	if let Some(record) = record(&mut txn).await? {
		if record.base.as_ref() != Some(&base) {
			return Err(GroupExists.build());
		}
		return Ok(GroupChildCreateResponse {
			created: false,
			record,
		});
	}
	let record = GroupRecord {
		status: GroupStatus::Live,
		base: Some(base),
		root: request.root,
		depth: request.depth,
		latest_fork: 0,
		key: request.key,
		live_children: 0,
	};
	put_record(&mut txn, &record)?;
	kv.commit(txn).await?;
	Ok(GroupChildCreateResponse {
		created: true,
		record,
	})
}

pub(super) async fn group_archive(
	kv: KvHandle,
	_: GroupArchiveRequest,
) -> Result<GroupArchiveResponse> {
	let _guard = lock_group(&kv).await?;
	let mut txn = kv.begin().await?;
	let Some(mut record) = record(&mut txn).await? else {
		return Err(GroupNotFound.build());
	};
	if record.status == GroupStatus::Archived {
		return Ok(GroupArchiveResponse {
			archived: false,
			record,
		});
	}
	// The record counts the children whose archive has not marked their fork child row, so the
	// archive costs one read however many forks the group had.
	if record.live_children > 0 {
		return Err(GroupHasChildren.build());
	}
	record.status = GroupStatus::Archived;
	put_record(&mut txn, &record)?;
	kv.commit(txn).await?;
	Ok(GroupArchiveResponse {
		archived: true,
		record,
	})
}

pub(super) async fn group_child_archived(
	kv: KvHandle,
	request: GroupChildArchivedRequest,
) -> Result<()> {
	let key = ForkChildKey::new(request.fork_number);
	let _guard = lock_group(&kv).await?;
	let mut txn = kv.begin().await?;
	let Some(bytes) = txn.get(&key.pack()).await? else {
		tracing::debug!(
			fork_number = request.fork_number,
			"a child archive names a fork child row the parent does not hold"
		);
		return Ok(());
	};
	let mut child = key
		.deserialize(bytes.into())
		.context("a fork child row does not decode")?;
	if child.child != request.child || child.archived {
		return Ok(());
	}
	child.archived = true;
	txn.put(&key.pack(), &key.serialize(child)?)?;
	// The child row and the count change in one commit, so the count matches the rows.
	if let Some(mut record) = record(&mut txn).await? {
		record.live_children = record.live_children.saturating_sub(1);
		put_record(&mut txn, &record)?;
	}
	kv.commit(txn).await?;
	Ok(())
}

/// The most event rows one post deletes. A post usually trims one row; this bounds the catch-up
/// after `retained_rows` is lowered, which spreads across later posts.
const MAX_EVENT_ROWS_TRIMMED_PER_POST: u64 = 16;

pub(super) async fn event_post(
	kv: KvHandle,
	request: EventPostRequest,
) -> Result<EventPostResponse> {
	ensure!(
		request.retained_rows > 0,
		"an event post retains at least one row"
	);
	let counter_key = EventSeqKey::new();
	let _guard = kv.lock_partition(counter_key.range()).await?;
	let mut txn = kv.begin().await?;
	let counter = decode_event_counter(txn.get(&counter_key.pack()).await?)?;
	let sequence = counter.next;
	let row_key = EventRowKey::new(sequence);
	txn.put(&row_key.pack(), &row_key.serialize(request.event)?)?;

	// Trim from `oldest` up to the row `retained_rows` below this one in the same commit,
	// deleting at most `MAX_EVENT_ROWS_TRIMMED_PER_POST` rows. Normally that is one row. After
	// `retained_rows` is lowered, the rows below the new bound go a bounded number per post,
	// oldest first, so no commit grows unbounded and the rows from `oldest` on stay without
	// gaps. A larger `retained_rows` trims nothing until the shard holds that many rows again.
	let mut oldest = counter.oldest;
	let mut trimmed = 0u64;
	if let Some(trim) = sequence.checked_sub(request.retained_rows) {
		while oldest <= trim && trimmed < MAX_EVENT_ROWS_TRIMMED_PER_POST {
			txn.delete(&EventRowKey::new(oldest).pack())?;
			oldest += 1;
			trimmed += 1;
		}
	}

	txn.put(
		&counter_key.pack(),
		&counter_key.serialize(EventCounter {
			next: sequence.checked_add(1).context("event numbers exhausted")?,
			oldest,
		})?,
	)?;
	kv.commit(txn).await?;
	crate::metrics::GROUP_EVENTS_TRIMMED.inc_by(trimmed);
	Ok(EventPostResponse { sequence })
}
