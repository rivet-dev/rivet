use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};
use sqlite_embedded::vfs::SqliteOperationProfile;

use crate::SqliteProfilingConfig;

pub(super) const FINGERPRINT_FORMAT_VERSION: u8 = 2;

#[derive(Clone, Debug)]
pub(super) struct StatementFingerprint {
	pub(super) display: String,
	pub(super) hash: String,
}

/// Immutable allowlists compiled once per database or metric sink.
#[derive(Debug)]
pub(crate) struct QuerySelection {
	queries: HashMap<String, StatementFingerprint>,
	fingerprints: HashSet<String>,
	matches_fingerprints: bool,
	tracks_transactions: bool,
}

impl QuerySelection {
	pub(crate) fn new(config: &SqliteProfilingConfig) -> Self {
		let queries = config
			.queries
			.iter()
			.filter_map(|sql| {
				statement_fingerprint(sql).map(|fingerprint| (sql.clone(), fingerprint))
			})
			.collect::<HashMap<_, _>>();
		let mut fingerprints = config.fingerprints.iter().cloned().collect::<HashSet<_>>();
		fingerprints.extend(
			queries
				.values()
				.map(|fingerprint| fingerprint.display.clone()),
		);
		let matches_fingerprints = !config.fingerprints.is_empty();
		let tracks_transactions = config.fingerprints.iter().any(|fingerprint| {
			fingerprint.starts_with("txn-") || fingerprint.starts_with("shape-")
		});
		Self {
			queries,
			fingerprints,
			matches_fingerprints,
			tracks_transactions,
		}
	}

	pub(crate) fn contains(&self, fingerprint: &str) -> bool {
		!fingerprint.is_empty() && self.fingerprints.contains(fingerprint)
	}

	fn statement(&self, sql: &str) -> Option<StatementFingerprint> {
		if let Some(fingerprint) = self.queries.get(sql) {
			return Some(fingerprint.clone());
		}
		if !self.matches_fingerprints {
			return None;
		}
		let fingerprint = statement_fingerprint(sql)?;
		if self.contains(&fingerprint.display) {
			Some(fingerprint)
		} else {
			None
		}
	}

	pub(super) fn tracks_transactions(&self) -> bool {
		self.tracks_transactions
	}
}

#[derive(Debug)]
pub(super) struct SqliteProfilingState {
	pub(super) config: SqliteProfilingConfig,
	pub(super) selection: QuerySelection,
	cataloged: scc::HashSet<String>,
}

impl Default for SqliteProfilingState {
	fn default() -> Self {
		Self::new(SqliteProfilingConfig::default())
	}
}

impl SqliteProfilingState {
	pub(super) fn new(config: SqliteProfilingConfig) -> Self {
		let config = config.with_process_override();
		let selection = QuerySelection::new(&config);
		Self {
			config,
			selection,
			cataloged: scc::HashSet::new(),
		}
	}

	pub(super) fn statement_fingerprint(&self, sql: &str) -> Option<StatementFingerprint> {
		if self.selection.tracks_transactions() {
			return statement_fingerprint(sql);
		}
		if let Some(fingerprint) = self.selection.statement(sql) {
			return Some(fingerprint);
		}
		statement_class(sql)?;
		Some(StatementFingerprint {
			display: String::new(),
			hash: String::new(),
		})
	}

	pub(super) fn mark_cataloged(&self, fingerprint: &str) -> bool {
		let limit = self
			.config
			.max_tracked_statement_fingerprints
			.saturating_add(self.config.max_tracked_transaction_fingerprints);
		self.cataloged.len() < limit && self.cataloged.insert_sync(fingerprint.to_owned()).is_ok()
	}
}

#[derive(Clone, Debug)]
pub(super) struct StatementObservation {
	pub(super) fingerprint: StatementFingerprint,
	pub(super) total_ns: u64,
	pub(super) transaction_wait_ns: u64,
	pub(super) profile: SqliteOperationProfile,
}

#[derive(Debug)]
pub(super) struct TransactionProfile {
	pub(super) started_at: crate::time::Instant,
	pub(super) transaction_wait_ns: u64,
	pub(super) fingerprint: Option<String>,
	pub(super) name: Option<String>,
	shape: Option<Sha256>,
	pub(super) statement_fingerprint_hashes:
		[Option<[u8; 16]>; sqlite_embedded::vfs::MAX_PROFILED_TRANSACTION_STATEMENTS],
	pub(super) omitted_statement_fingerprints: u64,
	statement_fingerprint_limit: usize,
	pub(super) statement_count: u64,
	pub(super) worker_wait_ns: u64,
	pub(super) storage_ns: u64,
	pub(super) local_work_ns: u64,
	pub(super) get_pages_round_trips: u64,
	pub(super) commit_round_trips: u64,
	pub(super) dirty_pages: u64,
	pub(super) dirty_bytes: u64,
	pub(super) commit_ns: u64,
}

impl TransactionProfile {
	pub(super) fn new(
		name: Option<String>,
		started_at: crate::time::Instant,
		statement_fingerprint_limit: usize,
		fingerprints: bool,
	) -> Self {
		let fingerprint = name.as_deref().filter(|_| fingerprints).map(|name| {
			format!(
				"txn-{}",
				fingerprint_hash(b"rivetkit-sqlite-transaction", name.as_bytes())
			)
		});
		let shape = if fingerprints {
			let mut shape = Sha256::new();
			shape.update(b"rivetkit-sqlite-transaction-shape");
			shape.update([FINGERPRINT_FORMAT_VERSION]);
			Some(shape)
		} else {
			None
		};
		Self {
			started_at,
			transaction_wait_ns: 0,
			fingerprint,
			name,
			shape,
			statement_fingerprint_hashes: [None;
				sqlite_embedded::vfs::MAX_PROFILED_TRANSACTION_STATEMENTS],
			omitted_statement_fingerprints: 0,
			statement_fingerprint_limit: statement_fingerprint_limit
				.min(sqlite_embedded::vfs::MAX_PROFILED_TRANSACTION_STATEMENTS),
			statement_count: 0,
			worker_wait_ns: 0,
			storage_ns: 0,
			local_work_ns: 0,
			get_pages_round_trips: 0,
			commit_round_trips: 0,
			dirty_pages: 0,
			dirty_bytes: 0,
			commit_ns: 0,
		}
	}

	pub(super) fn record_control(
		&mut self,
		profile: &SqliteOperationProfile,
		duration_ns: u64,
		is_commit: bool,
	) {
		self.worker_wait_ns = self.worker_wait_ns.saturating_add(profile.worker_wait_ns);
		self.storage_ns = self.storage_ns.saturating_add(profile.storage_ns);
		self.local_work_ns = self.local_work_ns.saturating_add(
			duration_ns
				.saturating_sub(profile.worker_wait_ns)
				.saturating_sub(profile.storage_ns),
		);
		self.get_pages_round_trips = self
			.get_pages_round_trips
			.saturating_add(profile.get_pages_round_trips);
		self.commit_round_trips = self
			.commit_round_trips
			.saturating_add(profile.commit_round_trips);
		self.dirty_pages = self.dirty_pages.saturating_add(profile.dirty_pages);
		self.dirty_bytes = self.dirty_bytes.saturating_add(profile.dirty_bytes);
		if is_commit {
			self.commit_ns = duration_ns;
		}
	}

	pub(super) fn record_statement(&mut self, observation: &StatementObservation) {
		if let Some(shape) = &mut self.shape {
			shape.update(observation.fingerprint.hash.as_bytes());
			shape.update([0]);
			let statement_index = self.statement_count as usize;
			if statement_index < self.statement_fingerprint_limit {
				let mut hash = [0; 16];
				let source = observation.fingerprint.hash.as_bytes();
				let copy_len = source.len().min(hash.len());
				hash[..copy_len].copy_from_slice(&source[..copy_len]);
				self.statement_fingerprint_hashes[statement_index] = Some(hash);
			} else {
				self.omitted_statement_fingerprints =
					self.omitted_statement_fingerprints.saturating_add(1);
			}
		}
		self.statement_count = self.statement_count.saturating_add(1);
		self.transaction_wait_ns = self
			.transaction_wait_ns
			.saturating_add(observation.transaction_wait_ns);
		self.worker_wait_ns = self
			.worker_wait_ns
			.saturating_add(observation.profile.worker_wait_ns);
		self.storage_ns = self
			.storage_ns
			.saturating_add(observation.profile.storage_ns);
		self.local_work_ns = self.local_work_ns.saturating_add(
			observation
				.total_ns
				.saturating_sub(observation.transaction_wait_ns)
				.saturating_sub(observation.profile.worker_wait_ns)
				.saturating_sub(observation.profile.storage_ns),
		);
		self.get_pages_round_trips = self
			.get_pages_round_trips
			.saturating_add(observation.profile.get_pages_round_trips);
		self.commit_round_trips = self
			.commit_round_trips
			.saturating_add(observation.profile.commit_round_trips);
		self.dirty_pages = self
			.dirty_pages
			.saturating_add(observation.profile.dirty_pages);
		self.dirty_bytes = self
			.dirty_bytes
			.saturating_add(observation.profile.dirty_bytes);
	}

	pub(super) fn fingerprint(&self) -> (String, &'static str) {
		if let Some(fingerprint) = &self.fingerprint {
			return (fingerprint.clone(), "name");
		}
		match &self.shape {
			Some(shape) => (
				format!("shape-{}", hex_prefix(&shape.clone().finalize())),
				"shape",
			),
			None => (String::new(), "unselected"),
		}
	}

	/// Keeps named transaction diagnostics useful without per-statement hashing.
	pub(super) fn diagnostic_fingerprint(&self) -> (String, &'static str) {
		if let Some(name) = &self.name {
			return (
				format!(
					"txn-{}",
					fingerprint_hash(b"rivetkit-sqlite-transaction", name.as_bytes())
				),
				"name",
			);
		}
		self.fingerprint()
	}

	pub(super) fn shape_fingerprint(&self) -> String {
		match &self.shape {
			Some(shape) => format!("shape-{}", hex_prefix(&shape.clone().finalize())),
			None => String::new(),
		}
	}
}

/// Hashes exact SQL only for fingerprint metrics or a selected diagnostic.
pub(super) fn statement_fingerprint(sql: &str) -> Option<StatementFingerprint> {
	let class = statement_class(sql)?;
	let hash = fingerprint_hash(b"rivetkit-sqlite-statement", sql.as_bytes());
	Some(StatementFingerprint {
		display: format!("{class}-{hash}"),
		hash,
	})
}

fn fingerprint_hash(domain: &[u8], value: &[u8]) -> String {
	let mut hasher = Sha256::new();
	hasher.update(domain);
	hasher.update([0, FINGERPRINT_FORMAT_VERSION, 0]);
	hasher.update(value);
	hex_prefix(&hasher.finalize())
}

fn hex_prefix(bytes: &[u8]) -> String {
	let mut output = String::with_capacity(16);
	for byte in bytes.iter().take(8) {
		use std::fmt::Write;
		let _ = write!(output, "{byte:02x}");
	}
	output
}

fn statement_class(sql: &str) -> Option<&'static str> {
	let keyword = sql.split_ascii_whitespace().next()?;
	if keyword.eq_ignore_ascii_case("select") || keyword.eq_ignore_ascii_case("values") {
		Some("select")
	} else if keyword.eq_ignore_ascii_case("insert") || keyword.eq_ignore_ascii_case("replace") {
		Some("insert")
	} else if keyword.eq_ignore_ascii_case("update") {
		Some("update")
	} else if keyword.eq_ignore_ascii_case("delete") {
		Some("delete")
	} else if keyword.eq_ignore_ascii_case("pragma") {
		Some("pragma")
	} else if ["begin", "commit", "end", "rollback", "savepoint", "release"]
		.iter()
		.any(|candidate| keyword.eq_ignore_ascii_case(candidate))
	{
		None
	} else if [
		"create", "alter", "drop", "vacuum", "reindex", "analyze", "attach", "detach",
	]
	.iter()
	.any(|candidate| keyword.eq_ignore_ascii_case(candidate))
	{
		Some("ddl")
	} else {
		Some("other")
	}
}


