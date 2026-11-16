//! The poll: one bounded read per shard per interval, from an in-memory cursor.
//!
//! This is a periodic poll by design and the one the spec allows: the group commands cannot
//! address every tablet that holds a group's volume, and a maintainer that misses a page still
//! reads the group record on its next load. The interval is the operator's
//! `depot.events.poll_interval_ms`; each pass reads every shard once and counts what it finds.

use super::{GroupCommands, GroupEventsFeed};
use anyhow::{Context, Result};
use rivet_data::generated::depot_group_v1::{Event, EventsReadRequest};
use rivet_env::Env;
use std::sync::Arc;
use tokio::sync::watch;

/// How the poll runs; `page` bounds one shard's read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PollConfig {
	pub shards: u16,
	pub poll_interval_ms: u64,
	pub page: u32,
}

/// Where the poll stands on each shard: `None` before the first read, which positions it at
/// the shard's tail, then the first row number not yet read. `failed` marks the shards whose
/// last read failed, whose events posted since their last read may be missing. `behind` marks
/// the shards whose last read returned a full page, which may have more events waiting; the
/// poll reads one page per shard per pass, so such a shard catches up over later passes.
pub struct Cursors {
	cursors: Vec<Option<u64>>,
	failed: Vec<bool>,
	behind: Vec<bool>,
}

impl Cursors {
	pub fn new(shards: u16) -> Self {
		Self {
			cursors: vec![None; usize::from(shards)],
			failed: vec![false; usize::from(shards)],
			behind: vec![false; usize::from(shards)],
		}
	}

	pub fn get(&self, shard: usize) -> Option<u64> {
		self.cursors[shard]
	}

	/// Every shard holds a cursor and its last read succeeded with a short page, so every
	/// event posted before each shard's last read has been read. A shard behind its tail keeps
	/// the cache unsynced until it returns a short page, so the cache never serves further
	/// behind than one poll interval.
	fn synced(&self) -> bool {
		self.cursors.iter().all(Option::is_some)
			&& !self.failed.iter().any(|failed| *failed)
			&& !self.behind.iter().any(|behind| *behind)
	}
}

/// Runs until `stop` says so. A shard whose read fails is retried at the next tick from the
/// same cursor; a shard that does not exist yet reads as empty.
///
/// A stop also ends a pass in flight. Once a worker stops, its reads cannot re-route and each
/// can wait out its command deadline, so finishing the pass would hold storage shutdown for
/// up to one deadline per shard. Dropping the pass is safe: each shard's cursor, cache and
/// publish change only after its read returns, so at most one read in flight is lost, and
/// the poll is a read-only cache refresh that ends with the process.
pub async fn run(
	env: Arc<dyn Env>,
	commands: Arc<dyn GroupCommands>,
	feed: Arc<GroupEventsFeed>,
	config: PollConfig,
	mut stop: watch::Receiver<bool>,
) -> Result<()> {
	let clock = env.clock();
	let mut cursors = Cursors::new(config.shards);
	let mut interval =
		rivet_env::capabilities::tokio::time::interval(clock.clone(), config.poll_interval_ms);
	loop {
		rivet_env::select_biased!("a stop ends the poll between ticks",
			changed = stop.changed() => {
				if changed.is_err() || *stop.borrow() {
					return Ok(());
				}
			},
			tick = interval.tick() => {
				tick.context("the events poll clock failed")?;
			},
		);
		if *stop.borrow() {
			return Ok(());
		}
		let pass = poll_once(commands.as_ref(), &feed, &mut cursors, config.page);
		rivet_env::select_biased!("a stop ends the pass in flight",
			changed = stop.changed() => {
				if changed.is_err() || *stop.borrow() {
					return Ok(());
				}
			},
			() = pass => {},
		);
	}
}

/// Reads every shard once from its cursor and publishes what it finds. Each page is applied
/// to the process's group record cache before any maintainer sees it, a gap drops the cache's
/// entries first, and the cache serves records only while every shard is positioned and its
/// last read succeeded with a short page.
pub async fn poll_once(
	commands: &dyn GroupCommands,
	feed: &GroupEventsFeed,
	cursors: &mut Cursors,
	page: u32,
) {
	let records = feed.records();
	for shard in 0..cursors.cursors.len() {
		let request = EventsReadRequest {
			first: cursors.cursors[shard],
			limit: page,
		};
		match commands.events_read(shard as u64, request).await {
			Ok(response) => {
				// A cursor below the shard's oldest row missed the trimmed rows, so the
				// maintainers read their groups again.
				let first = cursors.cursors[shard].is_none();
				let trimmed = cursors.cursors[shard].is_some_and(|cursor| cursor < response.oldest);
				cursors.cursors[shard] = Some(response.next);
				cursors.failed[shard] = false;
				let full = response.rows.len() >= page as usize && page > 0;
				if full {
					crate::metrics::GROUP_EVENTS_FULL_PAGES.inc();
				}
				cursors.behind[shard] = full;
				// A first read positions the shard at its tail, past any event posted to it
				// before then, while maintainers may already have cached groups that event
				// changed: tablets serve before the poll starts, and the shards are positioned
				// one after another, or over several passes when reads fail. Once the last
				// shard holds a cursor, one gap makes every maintainer read the groups it
				// cached before then again; every event posted since is read through the
				// cursors.
				let positioned = first && cursors.cursors.iter().all(Option::is_some);
				let gap = trimmed || positioned;
				let events: Vec<Event> = response.rows.into_iter().map(|row| row.event).collect();
				for event in &events {
					crate::metrics::GROUP_EVENTS
						.with_label_values(&[event_kind(event)])
						.inc();
				}
				// The cache learns the page before the maintainers, so a maintainer that reads
				// a group after it saw an event finds the event applied.
				if gap {
					records.gap();
				}
				records.apply(&events);
				records.set_synced(cursors.synced());
				if gap {
					feed.publish_gap(events);
				} else {
					feed.publish(events);
				}
			}
			Err(error) => {
				cursors.failed[shard] = true;
				records.set_synced(false);
				crate::metrics::GROUP_EVENTS_POLL_ERRORS.inc();
				tracing::debug!(
					%error,
					shard,
					"an event shard read failed; the next tick retries from the same cursor"
				);
			}
		}
	}
}

/// The bounded metric label of an event.
pub fn event_kind(event: &Event) -> &'static str {
	match event {
		Event::EventGroupForked(_) => "group_forked",
		Event::EventGroupArchived(_) => "group_archived",
	}
}
