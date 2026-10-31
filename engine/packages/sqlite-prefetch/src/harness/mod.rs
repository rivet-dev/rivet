//! The testing VFS, the trace log parser and the workload suite, behind the `harness` feature.
//!
//! A run drives real SQLite through [`vfs::TestVfs`], a pass-through client whose misses go to a
//! [`server::Server`] that runs the library exactly as the page protocol server does, over an
//! in-memory store that counts requests. There is no clock, network or thread: a run is
//! deterministic and takes milliseconds, so the whole suite fits `cargo test`.
pub mod oracle;
pub mod readahead;
pub mod server;
pub mod store;
pub mod trace;
pub mod vfs;
pub mod workloads;

use std::collections::BTreeMap;
use std::sync::Arc;

use server::{Server, Strategy};
use store::{MemStore, StoreCounts};
use vfs::{Client, ClientCounts, Event, TestVfs};

/// Pages SQLite's own cache holds per connection: 2,000 pages, SQLite's default.
pub const PAGER_CACHE_PAGES: u32 = 2_000;

/// A database under test: the VFS, its connection and its store.
pub struct Harness {
	// The connection must close before the VFS unregisters.
	connection: Option<rusqlite::Connection>,
	vfs: TestVfs,
	pub store: Arc<MemStore>,
	cache_pages: u32,
	marks: Vec<(String, Snapshot)>,
	start: Snapshot,
}

/// Counts at one point of a run.
#[derive(Debug, Default, Clone)]
pub struct Snapshot {
	pub client: ClientCounts,
	pub store: StoreCounts,
	pub commit_lookups: u64,
	pub rows_put: u64,
	pub rows_deleted: u64,
}

/// The counts a workload reports, for one strategy.
#[derive(Debug, Default, Clone)]
pub struct Report {
	pub workload: String,
	pub strategy: &'static str,
	pub round_trips: u64,
	pub reads: u64,
	pub hits: u64,
	pub prefetched: u64,
	pub false_hits: u64,
	pub false_hit_bytes: u64,
	pub false_hits_by_source: BTreeMap<&'static str, u64>,
	pub sent_by_source: BTreeMap<&'static str, u64>,
	pub object_store_requests: u64,
	pub depot_requests: u64,
	pub commit_lookups: u64,
	pub commits: u64,
	pub map_rows: usize,
	pub rows_put: u64,
	pub rows_deleted: u64,
	/// Round trips, reads and buffer hits between named marks, in order.
	pub phases: Vec<(String, u64, u64, u64)>,
}

impl Report {
	/// Pages the server sent: requested and prefetched.
	pub fn pages_sent(&self) -> u64 {
		self.sent_by_source.values().sum()
	}

	/// Pages SQLite read of those the server sent.
	pub fn pages_used(&self) -> u64 {
		self.pages_sent().saturating_sub(self.false_hits)
	}

	/// Pages used over pages sent, in thousandths; 1,000 when nothing was sent.
	pub fn hit_rate_permille(&self) -> u64 {
		match self.pages_sent() {
			0 => 1_000,
			sent => self.pages_used() * 1_000 / sent,
		}
	}

	/// The three KPIs: round trips, pages sent, and the page hit rate as a percentage.
	pub fn kpis(&self) -> String {
		format!(
			"round_trips={:<5} pages_sent={:<6} hit_rate={:>5.1}%",
			self.round_trips,
			self.pages_sent(),
			self.hit_rate_permille() as f64 / 10.0
		)
	}

	pub fn line(&self) -> String {
		let phases: Vec<String> = self
			.phases
			.iter()
			.map(|(name, trips, reads, hits)| format!("{name}={trips}/{reads}/{hits}"))
			.collect();
		let unread: Vec<String> = self
			.false_hits_by_source
			.iter()
			.map(|(source, pages)| format!("{source}:{pages}"))
			.collect();
		format!(
			"WORKLOAD {:<28} {:<8} {} reads={:<6} hits={:<6} prefetched={:<6} false_hits={:<5} ({}) object_store={:<5} depot={:<5} commit_lookups={:<3} commits={:<4} rows={:<5} phases=[{}]",
			self.workload,
			self.strategy,
			self.kpis(),
			self.reads,
			self.hits,
			self.prefetched,
			self.false_hits,
			unread.join(" "),
			self.object_store_requests,
			self.depot_requests,
			self.commit_lookups,
			self.commits,
			self.map_rows,
			phases.join(" ")
		)
	}
}

impl Harness {
	/// A fresh, empty database served by `strategy`.
	pub fn new(strategy: Strategy) -> Self {
		Self::with_store(strategy, Arc::new(MemStore::default()), Vec::new())
	}

	/// A database over an existing store. `future` is the oracle's knowledge.
	pub fn with_store(strategy: Strategy, store: Arc<MemStore>, future: Vec<u32>) -> Self {
		let mut server = Server::new(strategy, store.clone());
		server.future = future;
		server.open();
		let vfs = TestVfs::register(Client::new(server));
		let connection = vfs.connect(PAGER_CACHE_PAGES);
		Self {
			connection: Some(connection),
			vfs,
			store,
			cache_pages: PAGER_CACHE_PAGES,
			marks: Vec::new(),
			start: Snapshot::default(),
		}
	}

	/// Bounds this connection's SQLite pager, including subsequent reopenings. This models
	/// an actor's reduced cache share under pressure without changing the workload's SQL.
	pub fn set_pager_cache_pages(&mut self, pages: u32) {
		assert!(
			pages > 0 && pages <= 65_536,
			"bounded positive pager capacity"
		);
		self.cache_pages = pages;
		self.exec(&format!("PRAGMA cache_size = {pages}"));
	}

	/// Uses production's removal of whole non-header pages after handing them to SQLite.
	/// Historical golden measurements predate this distinction and retain consumed pages.
	pub fn use_native_handoff(&mut self) {
		self.vfs.client.lock().native_handoff = true;
	}

	pub fn connection(&self) -> &rusqlite::Connection {
		self.connection.as_ref().expect("open connection")
	}

	pub fn exec(&self, sql: &str) {
		self.connection()
			.execute_batch(sql)
			.unwrap_or_else(|error| panic!("{sql}: {error}"));
		self.check_panics();
	}

	/// Runs a query and returns how many rows and value bytes it read.
	pub fn query(&self, sql: &str) -> (usize, usize) {
		let mut statement = self
			.connection()
			.prepare_cached(sql)
			.unwrap_or_else(|error| panic!("{sql}: {error}"));
		let columns = statement.column_count();
		let mut rows = statement.query([]).expect("query");
		let mut count = 0;
		let mut bytes = 0;
		while let Some(row) = rows.next().expect("row") {
			count += 1;
			for column in 0..columns {
				bytes += match row.get_ref(column).expect("column") {
					rusqlite::types::ValueRef::Blob(blob) => blob.len(),
					rusqlite::types::ValueRef::Text(text) => text.len(),
					rusqlite::types::ValueRef::Null
					| rusqlite::types::ValueRef::Integer(_)
					| rusqlite::types::ValueRef::Real(_) => 8,
				};
			}
		}
		drop(rows);
		self.check_panics();
		(count, bytes)
	}

	fn check_panics(&self) {
		let panics = std::mem::take(&mut *vfs::PANICS.lock());
		assert!(
			panics.is_empty(),
			"testing VFS callbacks panicked: {panics:?}"
		);
	}

	/// Closes and reopens the database as the actor's next start would: a new owner, an empty
	/// client buffer, and storage caches cold.
	pub fn reopen(&mut self) {
		self.connection = None;
		self.store.chill();
		self.vfs.client.lock().reopen();
		self.connection = Some(self.vfs.connect(self.cache_pages));
	}

	/// Starts measuring here.
	pub fn begin(&mut self) {
		self.start = self.snapshot();
		self.marks.clear();
	}

	/// Names the phase that ends here.
	pub fn mark(&mut self, name: &str) {
		let snapshot = self.snapshot();
		self.marks.push((name.to_string(), snapshot));
	}

	pub fn snapshot(&self) -> Snapshot {
		let client = self.vfs.client.lock();
		Snapshot {
			client: client.counts.clone(),
			store: self.store.counts(),
			commit_lookups: client.server.commit_lookup_rounds,
			rows_put: client.server.rows_put,
			rows_deleted: client.server.rows_deleted,
		}
	}

	/// Advances only the deterministic planner clock, without sleeping or changing SQL.
	pub fn advance_clock(&self, ms: u64) {
		self.vfs.client.lock().server.advance_clock(ms);
	}

	pub fn clock_ms(&self) -> u64 {
		self.vfs.client.lock().server.clock_ms()
	}

	pub fn events(&self) -> Vec<Event> {
		self.vfs.client.lock().events.clone()
	}

	/// Injects commit outcomes for the next commits.
	pub fn fault_next_commits(&self, faults: Vec<server::CommitFault>) {
		self.vfs.client.lock().server.faults = faults;
	}

	/// The counts since [`Harness::begin`], with everything still unread in the buffer counted
	/// as a false hit.
	pub fn report(&mut self, workload: &str, strategy: Strategy) -> Report {
		self.vfs.client.lock().drain_buffer();
		let end = self.snapshot();
		let start = &self.start;
		let delta = |source: &BTreeMap<&'static str, u64>, base: &BTreeMap<&'static str, u64>| {
			source
				.iter()
				.map(|(key, value)| (*key, value - base.get(key).copied().unwrap_or(0)))
				.filter(|(_, value)| *value > 0)
				.collect::<BTreeMap<_, _>>()
		};
		let unread = delta(&end.client.unread, &start.client.unread);
		let sent = delta(&end.client.sent, &start.client.sent);
		let prefetched: u64 = sent
			.iter()
			.filter(|(source, _)| **source != "requested")
			.map(|(_, pages)| pages)
			.sum();
		let false_hits: u64 = unread.values().sum();
		let mut phases = Vec::new();
		let mut previous = &start.client;
		for (name, snapshot) in &self.marks {
			phases.push((
				name.clone(),
				snapshot.client.round_trips - previous.round_trips,
				snapshot.client.reads - previous.reads,
				snapshot.client.hits - previous.hits,
			));
			previous = &snapshot.client;
		}
		Report {
			workload: workload.to_string(),
			strategy: strategy.name(),
			round_trips: end.client.round_trips - start.client.round_trips,
			reads: end.client.reads - start.client.reads,
			hits: end.client.hits - start.client.hits,
			prefetched,
			false_hits,
			false_hit_bytes: false_hits * store::PAGE as u64,
			false_hits_by_source: unread,
			sent_by_source: sent,
			object_store_requests: end.store.object_store_requests
				- start.store.object_store_requests,
			depot_requests: end.store.depot_requests() - start.store.depot_requests(),
			commit_lookups: end.commit_lookups - start.commit_lookups,
			commits: end.client.commits - start.client.commits,
			map_rows: self
				.store
				.rows(crate::overflow::commit::Sidecar::Overflow)
				.len(),
			rows_put: end.rows_put - start.rows_put,
			rows_deleted: end.rows_deleted - start.rows_deleted,
			phases,
		}
	}

	/// Checks the overflow rows against the file.
	pub fn leak_check(&self, context: &str) -> oracle::LeakReport {
		let report = oracle::check(&self.store);
		report.assert_clean(context);
		report
	}
}

impl Drop for Harness {
	fn drop(&mut self) {
		self.connection = None;
	}
}
