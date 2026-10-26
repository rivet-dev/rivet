//! The row-leak oracle. It asks SQLite itself, through the `dbstat` virtual table over a copy
//! of the file, for every live overflow chain, and compares the overflow rows against them. It
//! shares no parsing with the library, so a parser bug cannot hide a leak.
#![allow(
	clippy::disallowed_methods,
	reason = "the oracle copies the file to a temporary directory outside any runtime environment"
)]

use std::collections::{BTreeMap, BTreeSet};

use super::store::MemStore;
use crate::overflow::chain::{self, Chain};
use crate::overflow::commit::Sidecar;
use crate::page::Pgno;

/// What the oracle found.
#[derive(Debug, Default)]
pub struct LeakReport {
	/// Every live chain, by its cell's path, in chain order.
	pub live_chains: usize,
	pub recorded_chains: usize,
	pub rows: usize,
	/// Rows or chains that are not exactly a live chain. Empty means no leak.
	pub problems: Vec<String>,
	/// Live chains without rows, allowed by the invariant.
	pub unrecorded: usize,
}

impl LeakReport {
	pub fn assert_clean(&self, context: &str) {
		assert!(
			self.problems.is_empty(),
			"{context}: overflow map leaks: {:?}",
			self.problems
		);
	}
}

/// Every live overflow chain of the database file `bytes`, as SQLite's `dbstat` reports it.
pub fn live_chains(bytes: &[u8]) -> Vec<Vec<Pgno>> {
	if bytes.is_empty() {
		return Vec::new();
	}
	let dir = tempfile::tempdir().expect("temporary directory");
	let path = dir.path().join("oracle.db");
	std::fs::write(&path, bytes).expect("write the oracle copy");
	let connection =
		rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
			.expect("open the oracle copy");
	let check: String = connection
		.query_row("PRAGMA integrity_check", [], |row| row.get(0))
		.expect("integrity check");
	assert_eq!(check, "ok", "the database file is corrupt");
	let mut statement = connection
		.prepare("SELECT name, path, pageno FROM dbstat WHERE pagetype = 'overflow'")
		.expect("dbstat is compiled in");
	// A cell's path is unique within its B-tree only, so chains are keyed by both.
	let mut chains: BTreeMap<(String, String), BTreeMap<u32, Pgno>> = BTreeMap::new();
	let rows = statement
		.query_map([], |row| {
			let name: String = row.get(0)?;
			let path: String = row.get(1)?;
			let pgno: i64 = row.get(2)?;
			Ok((name, path, pgno))
		})
		.expect("query dbstat");
	for row in rows {
		let (name, path, pgno) = row.expect("dbstat row");
		let Some((cell, index)) = path.rsplit_once('+') else {
			continue;
		};
		let index = u32::from_str_radix(index, 16).expect("hex overflow index");
		chains
			.entry((name, cell.to_string()))
			.or_default()
			.insert(index, pgno as Pgno);
	}
	chains
		.into_values()
		.map(|pages| pages.into_values().collect())
		.collect()
}

/// Compares the store's overflow rows with the live chains of its file.
pub fn check(store: &MemStore) -> LeakReport {
	let live = live_chains(&store.file());
	let live_set: BTreeSet<Vec<Pgno>> = live.iter().cloned().collect();
	let rows = store.rows(Sidecar::Overflow);
	let mut report = LeakReport {
		live_chains: live.len(),
		rows: rows.len(),
		..LeakReport::default()
	};

	let mut decoded = BTreeMap::new();
	for (key, value) in &rows {
		match chain::decode_row(key, value) {
			Some(row) => {
				decoded.insert(row.run.start, row);
			}
			None => report.problems.push(format!("undecodable row {key:?}")),
		}
	}
	let mut claimed: BTreeSet<Pgno> = BTreeSet::new();
	for row in decoded.values() {
		let Some(chain) = &row.chain else {
			continue;
		};
		report.recorded_chains += 1;
		let pages: Vec<Pgno> = chain.pages().collect();
		if !live_set.contains(&pages) {
			report.problems.push(format!(
				"row chain at {} is not a live chain: {pages:?}",
				row.run.start
			));
		}
		for run in &chain.runs {
			claimed.insert(run.start);
			match decoded.get(&run.start) {
				Some(other) if other.run == *run && other.chain_first == chain.head() => {}
				Some(other) => report.problems.push(format!(
					"run {} of chain {} disagrees with its row {other:?}",
					run.start,
					chain.head()
				)),
				None => report.problems.push(format!(
					"run {} of chain {} has no row",
					run.start,
					chain.head()
				)),
			}
		}
	}
	for start in decoded.keys() {
		if !claimed.contains(start) {
			report
				.problems
				.push(format!("row {start} belongs to no recorded chain"));
		}
	}
	let recorded: BTreeSet<Vec<Pgno>> = decoded
		.values()
		.filter_map(|row| {
			row.chain
				.as_ref()
				.map(|chain: &Chain| chain.pages().collect())
		})
		.collect();
	report.unrecorded = live_set.difference(&recorded).count();
	report
}
