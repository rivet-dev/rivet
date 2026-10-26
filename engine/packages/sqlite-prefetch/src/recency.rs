//! Recency: the pages an opening reads first, for the next wake, and the pages read most often
//! across openings, for steady state. Both persist as rows of `main.db-prefetch`, written inside
//! commits the database makes anyway.
//!
//! The server sees misses only: pages served from the client's buffer never reach it. So the
//! early list is this opening's misses in order followed by the previous list's pages it did
//! not miss, which are likely the ones the buffer served, cut to [`EARLY_PAGES`]. A page an
//! opening never reads again ages out as later openings put their own misses ahead of it.
use std::collections::{BTreeMap, BTreeSet};

use rivet_data::generated::sqlite_db_recent_v1::{Data, Early, Frequent, FrequentSet};
use rivet_data::versioned::SqliteDbRecentData;
use vbare::Versioned;

use crate::page::Pgno;

/// The row holding the early pages.
pub const EARLY_KEY: &[u8] = b"early";
/// The row holding the frequent pages.
pub const FREQUENT_KEY: &[u8] = b"frequent";

/// Most early pages remembered and preloaded: 2 MiB.
pub const EARLY_PAGES: usize = 512;
/// How long after the open the early phase lasts, on the lane's clock.
pub const EARLY_PHASE_MS: u64 = 1_000;
/// Counters in the frequent table: 8 KiB.
pub const FREQUENT_COUNTERS: usize = 1_024;
/// Most frequent pages persisted.
pub const FREQUENT_PERSISTED: usize = 512;
/// A frequent page is persisted only once read at least this often.
const FREQUENT_MIN_COUNT: u32 = 2;
/// Counts are halved when one reaches this, so the table keeps adapting.
const FREQUENT_MAX_COUNT: u32 = 1 << 15;
/// The frequent row rides every this-many commits, or a commit of many pages.
const FREQUENT_EVERY_COMMITS: u32 = 4;
const FREQUENT_EVERY_DIRTY_PAGES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
	Early,
	/// The early phase ended; the early row is due with the next commit.
	Ended,
	/// The early row went out with a commit of this opening.
	Written,
}

pub struct Recency {
	previous_early: Vec<Pgno>,
	early: Vec<Pgno>,
	early_set: BTreeSet<Pgno>,
	phase: Phase,
	opened_ms: u64,
	frequent: BTreeMap<Pgno, u32>,
	frequent_dirty: bool,
	commits_since_frequent: u32,
	frequent_planned: bool,
}

impl Recency {
	/// Starts an opening from the persisted rows, halving the frequent counts so a page unread
	/// for k openings fades by 2^-k.
	pub fn open(early: Option<&[u8]>, frequent: Option<&[u8]>, now_ms: u64) -> Self {
		let previous_early = early.and_then(decode_early).unwrap_or_default();
		let frequent = frequent
			.and_then(decode_frequent)
			.unwrap_or_default()
			.into_iter()
			.map(|(pgno, count)| (pgno, count / 2))
			.filter(|(_, count)| *count > 0)
			.collect();
		Self {
			previous_early,
			early: Vec::new(),
			early_set: BTreeSet::new(),
			phase: Phase::Early,
			opened_ms: now_ms,
			frequent,
			frequent_dirty: false,
			commits_since_frequent: 0,
			frequent_planned: false,
		}
	}

	/// The pages to fetch with page 1 when the database opens: the last opening's early list.
	pub fn opening_pages(&self) -> &[Pgno] {
		&self.previous_early
	}

	pub fn in_early_phase(&self) -> bool {
		self.phase == Phase::Early
	}

	/// Ends the early phase once its time is up. Called at each plan with the lane's clock.
	pub fn tick(&mut self, now_ms: u64) {
		if self.phase == Phase::Early && now_ms.saturating_sub(self.opened_ms) >= EARLY_PHASE_MS {
			self.phase = Phase::Ended;
		}
	}

	/// Records a page SQLite missed.
	pub fn miss(&mut self, pgno: Pgno) {
		if self.phase == Phase::Early && self.early_set.insert(pgno) {
			self.early.push(pgno);
			if self.early.len() >= EARLY_PAGES {
				self.phase = Phase::Ended;
			}
		}
		self.count(pgno);
	}

	/// A space-saving top-K counter: a present page counts up; an absent one replaces the
	/// smallest counter with that counter's value plus one.
	fn count(&mut self, pgno: Pgno) {
		self.frequent_dirty = true;
		if let Some(count) = self.frequent.get_mut(&pgno) {
			*count += 1;
			if *count >= FREQUENT_MAX_COUNT {
				for count in self.frequent.values_mut() {
					*count /= 2;
				}
				self.frequent.retain(|_, count| *count > 0);
			}
			return;
		}
		if self.frequent.len() < FREQUENT_COUNTERS {
			self.frequent.insert(pgno, 1);
			return;
		}
		let smallest = self
			.frequent
			.iter()
			.min_by_key(|(pgno, count)| (**count, **pgno))
			.map(|(pgno, count)| (*pgno, *count));
		if let Some((evicted, count)) = smallest {
			self.frequent.remove(&evicted);
			self.frequent.insert(pgno, count + 1);
		}
	}

	/// The frequent pages to prefetch, most frequent first, once per opening on the first plan
	/// after the early phase.
	pub fn take_frequent(&mut self) -> Vec<Pgno> {
		if self.phase == Phase::Early || self.frequent_planned {
			return Vec::new();
		}
		self.frequent_planned = true;
		self.top(FREQUENT_PERSISTED)
	}

	fn top(&self, limit: usize) -> Vec<Pgno> {
		let mut entries: Vec<(Pgno, u32)> = self
			.frequent
			.iter()
			.filter(|(_, count)| **count >= FREQUENT_MIN_COUNT)
			.map(|(pgno, count)| (*pgno, *count))
			.collect();
		entries.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
		entries.truncate(limit);
		entries.into_iter().map(|(pgno, _)| pgno).collect()
	}

	/// The recent rows a commit of `dirty_pages` pages carries. A commit ends the early phase,
	/// so the opening's first commit carries the early row.
	pub fn rows_for_commit(&mut self, dirty_pages: usize) -> RecencyRows {
		if self.phase == Phase::Early {
			self.phase = Phase::Ended;
		}
		self.commits_since_frequent += 1;
		let early = match self.phase {
			Phase::Ended => encode_early(&self.merged_early()),
			Phase::Early | Phase::Written => None,
		};
		let frequent_due = self.frequent_dirty
			&& (self.commits_since_frequent >= FREQUENT_EVERY_COMMITS
				|| dirty_pages >= FREQUENT_EVERY_DIRTY_PAGES);
		let frequent = match frequent_due {
			true => encode_frequent(&self.persisted_frequent()),
			false => None,
		};
		RecencyRows { early, frequent }
	}

	/// Records which rows a durable commit carried.
	pub fn committed(&mut self, carried: &RecencyCarried) {
		if carried.early {
			self.phase = Phase::Written;
		}
		if carried.frequent {
			self.frequent_dirty = false;
			self.commits_since_frequent = 0;
		}
	}

	/// This opening's misses, then the previous list's pages it did not miss.
	fn merged_early(&self) -> Vec<Pgno> {
		let mut pages = self.early.clone();
		for pgno in &self.previous_early {
			if pages.len() >= EARLY_PAGES {
				break;
			}
			if !self.early_set.contains(pgno) {
				pages.push(*pgno);
			}
		}
		pages
	}

	fn persisted_frequent(&self) -> Vec<(Pgno, u32)> {
		let mut entries: Vec<(Pgno, u32)> = self
			.frequent
			.iter()
			.filter(|(_, count)| **count >= FREQUENT_MIN_COUNT)
			.map(|(pgno, count)| (*pgno, *count))
			.collect();
		entries.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
		entries.truncate(FREQUENT_PERSISTED);
		entries
	}

	/// Drops pages past the end of the file.
	pub fn truncate(&mut self, size_pages: u32) {
		self.previous_early.retain(|pgno| *pgno <= size_pages);
		self.frequent.retain(|pgno, _| *pgno <= size_pages);
	}

	pub fn memory_bytes(&self) -> usize {
		(self.previous_early.capacity() + self.early.capacity()) * 4
			+ self.early_set.len() * 40
			+ self.frequent.len() * 48
	}
}

/// The encoded recent rows a commit carries, when due.
#[derive(Debug, Default)]
pub struct RecencyRows {
	pub early: Option<Vec<u8>>,
	pub frequent: Option<Vec<u8>>,
}

/// Which recent rows a commit carried.
#[derive(Debug, Default, Clone, Copy)]
pub struct RecencyCarried {
	pub early: bool,
	pub frequent: bool,
}

fn encode_early(pages: &[Pgno]) -> Option<Vec<u8>> {
	let mut bytes = Vec::with_capacity(pages.len() * 2);
	let mut previous = 0i64;
	for &pgno in pages {
		push_varint(&mut bytes, zigzag(i64::from(pgno) - previous));
		previous = i64::from(pgno);
	}
	let data = Data::Early(Early {
		pages: bytes.into(),
	});
	SqliteDbRecentData::serialize_with_embedded_version(&data, rivet_data::SQLITE_DB_RECENT_VERSION)
		.ok()
}

fn encode_frequent(entries: &[(Pgno, u32)]) -> Option<Vec<u8>> {
	let data = Data::FrequentSet(FrequentSet {
		entries: entries
			.iter()
			.map(|(pgno, count)| Frequent {
				pgno: *pgno,
				count: *count,
			})
			.collect(),
	});
	SqliteDbRecentData::serialize_with_embedded_version(&data, rivet_data::SQLITE_DB_RECENT_VERSION)
		.ok()
}

/// Decodes the early row. A malformed row decodes as none: recency only speeds up reads.
pub fn decode_early(bytes: &[u8]) -> Option<Vec<Pgno>> {
	let Data::Early(early) = SqliteDbRecentData::deserialize_with_embedded_version(bytes).ok()?
	else {
		return None;
	};
	let mut rest: &[u8] = &early.pages;
	let mut pages = Vec::new();
	let mut seen = BTreeSet::new();
	let mut previous = 0i64;
	while !rest.is_empty() {
		if pages.len() >= EARLY_PAGES {
			return None;
		}
		let (value, tail) = read_varint(rest)?;
		rest = tail;
		let pgno = previous.checked_add(unzigzag(value))?;
		let pgno = u32::try_from(pgno).ok().filter(|pgno| *pgno > 0)?;
		if !seen.insert(pgno) {
			return None;
		}
		pages.push(pgno);
		previous = i64::from(pgno);
	}
	Some(pages)
}

/// Decodes the frequent row.
pub fn decode_frequent(bytes: &[u8]) -> Option<Vec<(Pgno, u32)>> {
	let Data::FrequentSet(set) =
		SqliteDbRecentData::deserialize_with_embedded_version(bytes).ok()?
	else {
		return None;
	};
	if set.entries.len() > FREQUENT_COUNTERS {
		return None;
	}
	Some(
		set.entries
			.into_iter()
			.filter(|entry| entry.pgno > 0)
			.map(|entry| (entry.pgno, entry.count.min(FREQUENT_MAX_COUNT)))
			.collect(),
	)
}

fn zigzag(value: i64) -> u64 {
	((value << 1) ^ (value >> 63)) as u64
}

fn unzigzag(value: u64) -> i64 {
	((value >> 1) as i64) ^ -((value & 1) as i64)
}

fn push_varint(bytes: &mut Vec<u8>, mut value: u64) {
	while value >= 0x80 {
		bytes.push((value as u8) | 0x80);
		value >>= 7;
	}
	bytes.push(value as u8);
}

/// Reads one LEB128 varint of at most ten bytes.
fn read_varint(bytes: &[u8]) -> Option<(u64, &[u8])> {
	let mut value = 0u64;
	for (index, byte) in bytes.iter().enumerate().take(10) {
		if index == 9 && *byte > 1 {
			return None;
		}
		value |= u64::from(byte & 0x7f) << (7 * index);
		if byte & 0x80 == 0 {
			return Some((value, &bytes[index + 1..]));
		}
	}
	None
}
