//! The process-wide group record cache: the records of the volume groups this storage worker
//! process has read, shared by every tablet maintainer it runs, so a tablet that holds no
//! loaded volume of a group answers a create's lineage walk and fork stamp, and a due alarm's
//! archived check, without reading the group volume again.
//!
//! Every successful record read fills it, and the events poll keeps it current: a fork event
//! raises the parent's `latestFork`, an archive event marks the group archived, and a lost
//! cursor (a gap) drops every record. A miss, a read that asks for fork child rows, and every
//! read while some shard's cursor is not positioned, its last read failed or its last page was
//! full go to the group volume. The cache holds at most `depot.groups.max_cached_records`
//! records and evicts the least recently used one beyond that.
//!
//! A read can lag the group volume's commits: a writerless read answers from the latest
//! applied snapshot, which trails the commit by however far the volume's apply is behind, as
//! during a fork burst or a maintainer's catch-up. Its answer carries that lag
//! (`GroupReadResponse.commitLag`, the gap between the snapshot and the highest commit the
//! volume's maintainer knew of), and only an answer with no lag fills the cache. A fork draw or
//! an archive commits before its event is posted, so an answer at the commit holds every event
//! the poll could have applied or missed in a gap, and a lagging answer is served but never
//! cached. The bound left is the maintainer's own view of the commit, which follows the shards'
//! reports of the appends.
//!
//! The fields the cache serves order every event by position in the record itself: `latestFork`
//! only rises and is what a fork event names, and an archive is final. So the cache also keeps
//! what the events said of each group as a floor (the highest fork, archived, has a record) for
//! groups it holds no record of, and every answer, cached or not, is raised to the floor and to
//! the record already cached, so a lagging answer never returns a fork or an archive the process
//! already applied as missing. A group an event named is never cached as having no record. Only
//! what an event said is kept as a floor: a record that leaves the cache keeps a floor only if
//! an event touched it, so the floors grow with the groups that had events, not with cache
//! churn. They are bounded by the same cap, oldest first, and a gap keeps them.
//!
//! The lock is a `parking_lot::Mutex` rather than an `scc` or `moka` map because an event, a
//! gap, a fill and a lookup each change several structures at once (entries, recency, floors,
//! reads in flight, the synced flag), which must stay consistent with each other. It is a short
//! synchronous critical section with no awaits: the poll applies events from a synchronous
//! publish, and the maintainer executor looks up and fills from its read tasks.

use super::{GroupCommandFuture, GroupCommands};
use parking_lot::Mutex;
use rivet_data::generated::depot_group_v1::{
	Event, GroupReadRequest, GroupReadResponse, GroupRecord, GroupStatus,
};
use rivet_keys::keys::codec::GroupId;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::collections::BTreeMap;

/// The share of the cap, in percent, at which the cache warns that it is nearly full.
const NEAR_CAP_PERCENT: usize = 90;

/// Group records shared by every tablet maintainer of one storage worker process. Owned by the
/// process's [`super::GroupEventsFeed`], which the events poll and every maintainer already
/// hold.
pub struct GroupRecordCache {
	capacity: usize,
	inner: Mutex<Records>,
}

struct Records {
	/// Every shard holds a cursor and its last read succeeded, so every event posted before
	/// the last read of each shard has been applied. Lookups and fills happen only while set.
	synced: bool,
	entries: BTreeMap<GroupId, Entry>,
	/// Entries by their last use, oldest first, for eviction.
	recency: BTreeMap<u64, GroupId>,
	next_use: u64,
	/// Groups with record reads in flight to the group volume. An event or a gap that names
	/// a group while its read is in flight marks the read stale, since its snapshot may
	/// predate the event; a stale read's answer does not fill the cache. Bounded by the reads
	/// in flight across the process's maintainers.
	reads: BTreeMap<GroupId, ReadsInFlight>,
	/// The cache warned that it is nearly full and has not dropped well below that since.
	warned: bool,
	/// What the events said of groups the cache holds no record of, merged into every read of
	/// them. A floor leaves when a read fills the group's record, which carries it from then
	/// on, or as the oldest one at the cap.
	floors: BTreeMap<GroupId, Floor>,
	/// The floors by when an event last named them, oldest first, for the cap.
	floor_order: BTreeMap<u64, GroupId>,
	next_floor: u64,
	/// The floors reached the cap and dropped one, and the warning was logged; cleared once
	/// they fall well below it.
	floors_warned: bool,
}

/// What the events said of one group, monotonic like the events themselves. A group an event
/// named has a record: a fork's parent and child both do, and so does an archived group.
struct Floor {
	/// The highest fork a fork event drew with this group as the parent, or 0.
	latest_fork: u64,
	/// An archive event named the group.
	archived: bool,
	/// The floor's place in `floor_order`.
	order: u64,
	/// Counts this floor in `depot_group_record_cache_event_floors` while it is kept.
	_usage: IntGaugeGuard,
}

struct Entry {
	/// The group's record, or `None` for a group volume that holds no record yet, which a
	/// reader treats as a root with no forks. `liveChildren` stays as last read: no maintainer
	/// path uses it, and the archive command reads the group volume itself.
	record: Option<GroupRecord>,
	last_use: u64,
	/// An event raised the record, or the read that filled it was raised to an event's floor.
	/// Only such a record leaves a floor when it leaves the cache.
	event_touched: bool,
	/// Counts this entry in `depot_group_record_cache_entries` while it is cached.
	_usage: IntGaugeGuard,
}

#[derive(Default)]
struct ReadsInFlight {
	count: u32,
	stale: bool,
}

/// One record read of `group` in flight to the group volume. Its answer fills the cache
/// through [`ReadTicket::complete`]; dropping it unanswered, as an aborted read task does,
/// only releases its place.
pub struct ReadTicket<'a> {
	cache: &'a GroupRecordCache,
	group: GroupId,
}

impl GroupRecordCache {
	pub fn new(capacity: usize) -> Self {
		crate::metrics::GROUP_RECORD_CACHE_CAPACITY
			.set(i64::try_from(capacity).unwrap_or(i64::MAX));
		Self {
			capacity: capacity.max(1),
			inner: Mutex::new(Records {
				synced: false,
				entries: BTreeMap::new(),
				recency: BTreeMap::new(),
				next_use: 0,
				reads: BTreeMap::new(),
				warned: false,
				floors: BTreeMap::new(),
				floor_order: BTreeMap::new(),
				next_floor: 0,
				floors_warned: false,
			}),
		}
	}

	/// The cached record of `group`, freshened as the most recently used. The outer `None` is
	/// a miss, and so is every lookup while the cache is not synced; `Some(None)` is a group
	/// whose volume held no record when it was read.
	pub fn get(&self, group: GroupId) -> Option<Option<GroupRecord>> {
		let mut records = self.inner.lock();
		if !records.synced {
			crate::metrics::GROUP_RECORD_CACHE
				.with_label_values(&["unsynced"])
				.inc();
			return None;
		}
		let use_seq = records.next_use;
		let Some(entry) = records.entries.get_mut(&group) else {
			crate::metrics::GROUP_RECORD_CACHE
				.with_label_values(&["miss"])
				.inc();
			return None;
		};
		let previous = std::mem::replace(&mut entry.last_use, use_seq);
		let record = entry.record.clone();
		records.recency.remove(&previous);
		records.recency.insert(use_seq, group);
		records.next_use += 1;
		crate::metrics::GROUP_RECORD_CACHE
			.with_label_values(&["hit"])
			.inc();
		Some(record)
	}

	/// Registers a record read of `group` about to go to the group volume, so events applied
	/// while it is in flight keep its answer out of the cache.
	pub fn start_read(&self, group: GroupId) -> ReadTicket<'_> {
		let mut records = self.inner.lock();
		records.reads.entry(group).or_default().count += 1;
		ReadTicket { cache: self, group }
	}

	/// Applies a page of events the poll read, before the page reaches any maintainer, so a
	/// maintainer that reads a group after it saw an event finds the event in the cache, and a
	/// read that lags the event cannot cache the record from before it.
	pub fn apply(&self, events: &[Event]) {
		if events.is_empty() {
			return;
		}
		let mut records = self.inner.lock();
		for event in events {
			match event {
				Event::EventGroupForked(fork) => {
					records.mark_stale(fork.parent);
					records.mark_stale(fork.child);
					let fork_number = fork.fork_number;
					let raised = match records.entries.get_mut(&fork.parent) {
						Some(Entry {
							record: Some(record),
							event_touched,
							..
						}) => {
							record.latest_fork = record.latest_fork.max(fork_number);
							*event_touched = true;
							true
						}
						// A parent read before its record was written has no record to raise.
						Some(Entry { record: None, .. }) => {
							records.invalidate(fork.parent, "event");
							false
						}
						None => false,
					};
					if !raised {
						records.note_floor(self.capacity, fork.parent, |floor| {
							floor.latest_fork = floor.latest_fork.max(fork_number);
						});
					}
					// A child read before its record was written cached as a root; its fork
					// says it has a base, and a read that still finds none is behind.
					match records.entries.get(&fork.child) {
						Some(Entry {
							record: Some(_), ..
						}) => {}
						Some(Entry { record: None, .. }) => {
							records.invalidate(fork.child, "event");
							records.note_floor(self.capacity, fork.child, |_| {});
						}
						None => records.note_floor(self.capacity, fork.child, |_| {}),
					}
				}
				Event::EventGroupArchived(archive) => {
					records.mark_stale(archive.group);
					let marked = match records.entries.get_mut(&archive.group) {
						Some(Entry {
							record: Some(record),
							event_touched,
							..
						}) => {
							record.status = GroupStatus::Archived;
							*event_touched = true;
							true
						}
						Some(Entry { record: None, .. }) => {
							records.invalidate(archive.group, "event");
							false
						}
						None => false,
					};
					if !marked {
						records.note_floor(self.capacity, archive.group, |floor| {
							floor.archived = true;
						});
					}
				}
			}
		}
	}

	/// The poll lost its cursor on a shard, or positioned its last shard, so events may have
	/// been missed: every record is dropped and every read in flight is kept out of the cache.
	/// What an event said of a dropped record stays as the group's floor, since it was true
	/// before the gap and the fields the cache serves never go back.
	pub fn gap(&self) {
		let mut records = self.inner.lock();
		let dropped = std::mem::take(&mut records.entries);
		records.recency.clear();
		for read in records.reads.values_mut() {
			read.stale = true;
		}
		records.warned = false;
		crate::metrics::GROUP_RECORD_CACHE_INVALIDATIONS
			.with_label_values(&["gap"])
			.inc_by(dropped.len() as u64);
		for (group, entry) in dropped {
			records.keep_as_floor(self.capacity, group, entry);
		}
	}

	/// Whether every shard holds a cursor and its last read succeeded; see [`Records::synced`].
	/// Reads that finish while unsynced do not fill the cache, since an event of a failing
	/// shard may be missing from their answer's future updates until that shard reads again.
	pub fn set_synced(&self, synced: bool) {
		let mut records = self.inner.lock();
		if records.synced && !synced {
			for read in records.reads.values_mut() {
				read.stale = true;
			}
		}
		records.synced = synced;
	}

	/// The records the cache holds, not counting the floors.
	pub fn len(&self) -> usize {
		self.inner.lock().entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl Records {
	fn mark_stale(&mut self, group: GroupId) {
		if let Some(read) = self.reads.get_mut(&group) {
			read.stale = true;
		}
	}

	/// Raises `group`'s floor with `raise`, noted as the newest. At `capacity` floors
	/// the oldest other one is dropped first, so a new group cannot drop its own floor; a read
	/// that lags the dropped group's events may then fill the cache without them.
	fn note_floor(&mut self, capacity: usize, group: GroupId, raise: impl FnOnce(&mut Floor)) {
		if !self.floors.contains_key(&group) && self.floors.len() >= capacity {
			if let Some((_, oldest)) = self.floor_order.pop_first() {
				self.floors.remove(&oldest);
				crate::metrics::GROUP_RECORD_CACHE_INVALIDATIONS
					.with_label_values(&["floor_evicted"])
					.inc();
				if !self.floors_warned {
					self.floors_warned = true;
					tracing::warn!(
						capacity,
						"more groups had events than depot.groups.max_cached_records; the oldest event floors are dropped, so a group read lagging their events may fill the cache without them"
					);
				}
			}
		} else if self.floors_warned && self.floors.len() * 100 < capacity * (NEAR_CAP_PERCENT - 10)
		{
			self.floors_warned = false;
		}
		let order = self.next_floor;
		self.next_floor += 1;
		let floor = self.floors.entry(group).or_insert_with(|| Floor {
			latest_fork: 0,
			archived: false,
			order,
			_usage: crate::metrics::GROUP_RECORD_CACHE_FLOORS.inc_guard(),
		});
		let previous = std::mem::replace(&mut floor.order, order);
		raise(floor);
		self.floor_order.remove(&previous);
		self.floor_order.insert(order, group);
	}

	/// Keeps what a record leaving the cache says as `group`'s floor when an event touched it.
	/// A record no event touched leaves nothing, so evictions by ordinary churn never take the
	/// floors' places, and neither does a group cached as having no record.
	fn keep_as_floor(&mut self, capacity: usize, group: GroupId, entry: Entry) {
		if !entry.event_touched {
			return;
		}
		let Some(record) = entry.record else {
			return;
		};
		self.note_floor(capacity, group, |floor| {
			floor.latest_fork = floor.latest_fork.max(record.latest_fork);
			floor.archived |= record.status == GroupStatus::Archived;
		});
	}

	/// `record` as read, raised to the record already cached and to the group's floor, and
	/// whether either raised it.
	fn merged(&self, group: GroupId, record: Option<&GroupRecord>) -> (Option<GroupRecord>, bool) {
		let Some(read) = record else {
			return (None, false);
		};
		let mut record = read.clone();
		let mut raised = false;
		let cached = self
			.entries
			.get(&group)
			.and_then(|entry| entry.record.as_ref());
		if let Some(cached) = cached {
			raised |= cached.latest_fork > record.latest_fork
				|| (cached.status == GroupStatus::Archived
					&& record.status != GroupStatus::Archived);
			record.latest_fork = record.latest_fork.max(cached.latest_fork);
			if cached.status == GroupStatus::Archived {
				record.status = GroupStatus::Archived;
			}
		}
		if let Some(floor) = self.floors.get(&group) {
			raised |= floor.latest_fork > record.latest_fork
				|| (floor.archived && record.status != GroupStatus::Archived);
			record.latest_fork = record.latest_fork.max(floor.latest_fork);
			if floor.archived {
				record.status = GroupStatus::Archived;
			}
		}
		(Some(record), raised)
	}

	fn invalidate(&mut self, group: GroupId, reason: &'static str) {
		if let Some(entry) = self.entries.remove(&group) {
			self.recency.remove(&entry.last_use);
			crate::metrics::GROUP_RECORD_CACHE_INVALIDATIONS
				.with_label_values(&[reason])
				.inc();
		}
	}

	/// Caches `record` for `group` as the most recently used entry and evicts the least
	/// recently used ones beyond `capacity`, keeping what an event said of each evicted record
	/// as a floor.
	fn insert(
		&mut self,
		capacity: usize,
		group: GroupId,
		record: Option<GroupRecord>,
		event_touched: bool,
	) {
		let use_seq = self.next_use;
		self.next_use += 1;
		let previous = self.entries.insert(
			group,
			Entry {
				record,
				last_use: use_seq,
				event_touched,
				_usage: crate::metrics::GROUP_RECORD_CACHE_ENTRIES.inc_guard(),
			},
		);
		if let Some(previous) = previous {
			self.recency.remove(&previous.last_use);
		}
		self.recency.insert(use_seq, group);
		while self.entries.len() > capacity {
			let Some((_, oldest)) = self.recency.pop_first() else {
				break;
			};
			if let Some(evicted) = self.entries.remove(&oldest) {
				self.keep_as_floor(capacity, oldest, evicted);
			}
			crate::metrics::GROUP_RECORD_CACHE_INVALIDATIONS
				.with_label_values(&["evicted"])
				.inc();
		}
		let near = self.entries.len() * 100 >= capacity * NEAR_CAP_PERCENT;
		if near && !self.warned {
			self.warned = true;
			tracing::warn!(
				entries = self.entries.len(),
				capacity,
				"the group record cache is nearly full, likely more active volume groups than depot.groups.max_cached_records; evicted groups are read from their group volume again"
			);
		} else if !near && self.entries.len() * 100 < capacity * (NEAR_CAP_PERCENT - 10) {
			self.warned = false;
		}
	}
}

impl ReadTicket<'_> {
	/// The read answered with `record`, from a snapshot `commit_lag` volume seqs behind the
	/// highest commit the group volume's maintainer knew of. Returns the answer raised to what
	/// the cache knows of the group, which is what the caller gets. Only an answer with no lag
	/// fills the cache, since a lagging one may miss a fork draw or an archive whose event the
	/// cache never saw. It also fills nothing if an event or a gap named the group while the
	/// read was in flight or the cache is not synced, and a read that found no record fills
	/// nothing for a group an event named or the cache holds a record of: that read is behind.
	/// A `commit_lag` of `u64::MAX` means the commit was unknown; it fills nothing and is not
	/// observed, since it is no distance.
	pub fn complete(self, record: Option<&GroupRecord>, commit_lag: u64) -> Option<GroupRecord> {
		if commit_lag != u64::MAX {
			crate::metrics::GROUP_RECORD_READ_LAG_ENTRIES.observe(commit_lag as f64);
		}
		let mut records = self.cache.inner.lock();
		let (merged, raised) = records.merged(self.group, record);
		let stale = records
			.reads
			.get(&self.group)
			.is_some_and(|read| read.stale);
		if !records.synced || stale || commit_lag > 0 {
			return merged;
		}
		match &merged {
			Some(_) => {
				// The record carries the floor from now on.
				let floor = records.floors.remove(&self.group);
				let had_floor = floor.is_some();
				if let Some(floor) = floor {
					records.floor_order.remove(&floor.order);
				}
				let event_touched = raised
					|| had_floor || records
					.entries
					.get(&self.group)
					.is_some_and(|entry| entry.event_touched);
				records.insert(
					self.cache.capacity,
					self.group,
					merged.clone(),
					event_touched,
				);
			}
			None => {
				let known = records.floors.contains_key(&self.group)
					|| records
						.entries
						.get(&self.group)
						.is_some_and(|entry| entry.record.is_some());
				if !known {
					records.insert(self.cache.capacity, self.group, None, false);
				}
			}
		}
		merged
	}
}

impl Drop for ReadTicket<'_> {
	fn drop(&mut self) {
		let mut records = self.cache.inner.lock();
		if let Some(read) = records.reads.get_mut(&self.group) {
			read.count -= 1;
			if read.count == 0 {
				records.reads.remove(&self.group);
			}
		}
	}
}

/// Reads `group`'s record and the fork child rows `fork_numbers` names, from `cache` when it
/// holds the record and no rows are asked for, else from the group volume through `commands`,
/// filling `cache` with the answer when it is at the volume's commit. A record read from the
/// group volume is answered raised to what the cache knows of the group, so a lagging read never
/// returns a fork or an archive the process already applied as missing.
pub async fn read_group(
	commands: &dyn GroupCommands,
	cache: Option<&GroupRecordCache>,
	group: GroupId,
	fork_numbers: Vec<u64>,
) -> Result<GroupReadResponse, String> {
	let Some(cache) = cache else {
		return commands
			.group_read(group, GroupReadRequest { fork_numbers })
			.await;
	};
	if fork_numbers.is_empty() {
		if let Some(record) = cache.get(group) {
			return Ok(GroupReadResponse {
				commit_lag: 0,
				record,
				children: Vec::new(),
			});
		}
	} else {
		crate::metrics::GROUP_RECORD_CACHE
			.with_label_values(&["children"])
			.inc();
	}
	let ticket = cache.start_read(group);
	let read: GroupCommandFuture<'_, GroupReadResponse> =
		commands.group_read(group, GroupReadRequest { fork_numbers });
	let mut response = read.await?;
	response.record = ticket.complete(response.record.as_ref(), response.commit_lag);
	Ok(response)
}
