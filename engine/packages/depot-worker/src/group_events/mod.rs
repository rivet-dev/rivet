//! The depot-wide events every storage worker polls, and how they reach the maintainers of the
//! process. The group commands post an event to any shard when a group is forked or archived.
//! One poll per process reads every shard once per interval from an in-memory cursor that
//! starts at the shard's tail, and the feed hands each page to every maintainer running in the
//! process. Nothing here is durable, and a dropped page is harmless: the group record a
//! maintainer reads on load is the source of truth, and the events only keep already loaded
//! groups current between loads. Groups cached before every shard holds a cursor are read
//! again once the last shard is positioned, since an event may have landed below a cursor's
//! starting tail. The poll applies each page to the process's group record cache
//! ([`records::GroupRecordCache`]) before handing it to the maintainers.

pub mod poll;
pub mod records;

use crate::tablet_maintainer_machine::event::MaintainerEvent;
use rivet_data::generated::depot_group_v1::{
	Event, EventsReadRequest, EventsReadResponse, GroupReadRequest, GroupReadResponse,
};
use rivet_keys::keys::codec::GroupId;
use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
};
use tokio::sync::mpsc;

pub type GroupCommandFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// Runs the group read commands for this storage worker. The engine implements it over its
/// depot client; a read of a volume that does not exist answers as empty, not as an error.
pub trait GroupCommands: Send + Sync + 'static {
	fn group_read(
		&self,
		group: GroupId,
		request: GroupReadRequest,
	) -> GroupCommandFuture<'_, GroupReadResponse>;

	fn events_read(
		&self,
		shard: u64,
		request: EventsReadRequest,
	) -> GroupCommandFuture<'_, EventsReadResponse>;
}

/// Hands every page of events the poll reads to every maintainer of the process. A maintainer
/// subscribes its inbox when it starts and unsubscribes when it shuts down; a page that finds
/// an inbox full or closed is dropped for that maintainer, since the events are idempotent and
/// the group record makes up for a missed one at the next load.
pub struct GroupEventsFeed {
	/// The process's group records, kept current by the pages the poll publishes here.
	records: Arc<records::GroupRecordCache>,
	listeners: scc::HashMap<u64, mpsc::Sender<MaintainerEvent>>,
	/// Listeners a page could not reach, told of the gap with their next page.
	gapped: scc::HashSet<u64>,
	next_listener: AtomicU64,
}

impl GroupEventsFeed {
	/// A feed whose group record cache holds at most `max_cached_records` groups
	/// (`depot.groups.max_cached_records`).
	pub fn new(max_cached_records: usize) -> Self {
		Self {
			records: Arc::new(records::GroupRecordCache::new(max_cached_records)),
			listeners: scc::HashMap::new(),
			gapped: scc::HashSet::new(),
			next_listener: AtomicU64::new(1),
		}
	}

	/// Registers `inbox` and returns the id that unsubscribes it. Called from the maintainer's
	/// synchronous start, and nothing awaits on this map, so the synchronous map operations
	/// are safe here.
	pub fn subscribe(&self, inbox: mpsc::Sender<MaintainerEvent>) -> u64 {
		let id = self.next_listener.fetch_add(1, Ordering::Relaxed);
		let _ = self.listeners.insert_sync(id, inbox);
		crate::metrics::GROUP_EVENT_LISTENERS.set(self.listeners.len() as i64);
		id
	}

	pub fn unsubscribe(&self, id: u64) {
		self.listeners.remove_sync(&id);
		crate::metrics::GROUP_EVENT_LISTENERS.set(self.listeners.len() as i64);
	}

	/// Offers `events` to every listener without waiting, and forgets the listeners whose
	/// maintainer stopped. Returns how many listeners took the page.
	pub fn publish(&self, events: Vec<Event>) -> usize {
		self.publish_with(events, false)
	}

	/// Offers `events` like [`Self::publish`], telling every listener that events before them
	/// may be missing.
	pub fn publish_gap(&self, events: Vec<Event>) -> usize {
		self.publish_with(events, true)
	}

	fn publish_with(&self, events: Vec<Event>, gap: bool) -> usize {
		let mut delivered = 0;
		let mut closed = Vec::new();
		self.listeners.iter_sync(|id, inbox| {
			// A listener that missed a page learns of the gap with the next one it takes.
			let missed = self.gapped.contains_sync(id);
			if events.is_empty() && !gap && !missed {
				return true;
			}
			match inbox.try_send(MaintainerEvent::GroupEvents {
				events: events.clone(),
				gap: gap || missed,
			}) {
				Ok(()) => {
					delivered += 1;
					if missed {
						self.gapped.remove_sync(id);
					}
				}
				Err(mpsc::error::TrySendError::Full(_)) => {
					let _ = self.gapped.insert_sync(*id);
					crate::metrics::GROUP_EVENTS_DROPPED
						.with_label_values(&["inbox_full"])
						.inc();
				}
				Err(mpsc::error::TrySendError::Closed(_)) => closed.push(*id),
			}
			true
		});
		if !closed.is_empty() {
			for id in closed {
				self.listeners.remove_sync(&id);
				self.gapped.remove_sync(&id);
			}
			crate::metrics::GROUP_EVENT_LISTENERS.set(self.listeners.len() as i64);
		}
		delivered
	}

	pub fn listeners(&self) -> usize {
		self.listeners.len()
	}

	/// The process's group record cache, which the maintainers' group reads consult.
	pub fn records(&self) -> &Arc<records::GroupRecordCache> {
		&self.records
	}
}
