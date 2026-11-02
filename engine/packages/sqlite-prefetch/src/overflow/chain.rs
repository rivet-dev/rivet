//! An overflow chain as runs of pages, and its rows in the `main.db-prefetch` KV inode.
use rivet_data::generated::sqlite_db_overflow_v1::{OverflowRun, RunRef};
use rivet_data::versioned::SqliteDbOverflowData;
use vbare::Versioned;

use crate::page::Pgno;

/// The most pages a run spans, from its first page to its last. A floor lookup for a page P
/// reads the rows whose run starts in `P - MAX_RUN_SPAN + 1 ..= P`, so bounding the span bounds
/// that read. A longer contiguous or strided stretch is split into several runs.
pub const MAX_RUN_SPAN: u32 = 256;

/// The pages `start + i * step` for `i < len`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Run {
	pub start: Pgno,
	pub len: u32,
	pub step: u16,
}

impl Run {
	pub fn last(&self) -> Pgno {
		self.start + (self.len - 1) * u32::from(self.step)
	}

	/// The index of `pgno` in this run.
	pub fn index_of(&self, pgno: Pgno) -> Option<u32> {
		if pgno < self.start || pgno > self.last() {
			return None;
		}
		let offset = pgno - self.start;
		let step = u32::from(self.step);
		match offset % step {
			0 => Some(offset / step),
			_ => None,
		}
	}

	pub fn page(&self, index: u32) -> Pgno {
		self.start + index * u32::from(self.step)
	}
}

/// A whole overflow chain: its pages in chain order, as runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chain {
	pub runs: Vec<Run>,
	/// The head the commit or read that learned the chain saw. Diagnostic only.
	pub learned_txid: u64,
}

impl Chain {
	/// Splits `pages`, in chain order, into runs. Returns `None` for an empty list or one that
	/// names a page twice or page zero.
	pub fn from_pages(pages: &[Pgno], learned_txid: u64) -> Option<Self> {
		if pages.is_empty() || pages.contains(&0) {
			return None;
		}
		let mut seen = std::collections::BTreeSet::new();
		if !pages.iter().all(|pgno| seen.insert(*pgno)) {
			return None;
		}
		let mut runs: Vec<Run> = Vec::new();
		for &pgno in pages {
			if let Some(run) = runs.last_mut() {
				let extends = match run.len {
					1 => pgno > run.start && pgno - run.start < MAX_RUN_SPAN,
					_ => {
						pgno > run.last()
							&& pgno - run.last() == u32::from(run.step)
							&& pgno - run.start < MAX_RUN_SPAN
					}
				};
				if extends {
					if run.len == 1 {
						let step = u16::try_from(pgno - run.start).ok()?;
						run.step = step;
					}
					run.len += 1;
					continue;
				}
			}
			runs.push(Run {
				start: pgno,
				len: 1,
				step: 1,
			});
		}
		Some(Self { runs, learned_txid })
	}

	pub fn head(&self) -> Pgno {
		self.runs[0].start
	}

	pub fn page_count(&self) -> u32 {
		self.runs.iter().map(|run| run.len).sum()
	}

	/// The chain's pages in chain order.
	pub fn pages(&self) -> impl Iterator<Item = Pgno> + '_ {
		self.runs
			.iter()
			.flat_map(|run| (0..run.len).map(move |index| run.page(index)))
	}

	/// The position of `pgno` in the chain.
	pub fn position(&self, pgno: Pgno) -> Option<u32> {
		let mut before = 0;
		for run in &self.runs {
			if let Some(index) = run.index_of(pgno) {
				return Some(before + index);
			}
			before += run.len;
		}
		None
	}

	/// The page after `position`, or zero after the last page, as the chain's next pointers
	/// must read.
	pub fn next_after(&self, position: u32) -> Pgno {
		self.pages().nth(position as usize + 1).unwrap_or(0)
	}

	/// The keys of every row this chain has.
	pub fn keys(&self) -> impl Iterator<Item = [u8; 4]> + '_ {
		self.runs.iter().map(|run| run.start.to_be_bytes())
	}

	/// Encodes the chain's rows, its first run first. Only the first run lists the others.
	pub fn rows(&self) -> anyhow::Result<Vec<([u8; 4], Vec<u8>)>> {
		let chain_pages = self.page_count();
		let mut rows = Vec::with_capacity(self.runs.len());
		let mut offset = 0;
		for (index, run) in self.runs.iter().enumerate() {
			let rest = match index {
				0 => self.runs[1..]
					.iter()
					.map(|run| RunRef {
						start: run.start,
						len: run.len,
						step: run.step,
					})
					.collect(),
				_ => Vec::new(),
			};
			let value = OverflowRun {
				len: run.len,
				step: run.step,
				chain_first: self.head(),
				offset_in_chain: offset,
				chain_pages,
				learned_txid: self.learned_txid,
				rest,
			};
			let bytes = SqliteDbOverflowData::serialize_with_embedded_version(
				&value,
				rivet_data::SQLITE_DB_OVERFLOW_VERSION,
			)?;
			rows.push((run.start.to_be_bytes(), bytes));
			offset += run.len;
		}
		Ok(rows)
	}
}

/// A decoded overflow row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
	pub run: Run,
	pub chain_first: Pgno,
	pub offset_in_chain: u32,
	pub chain_pages: u32,
	pub learned_txid: u64,
	/// The whole chain, on the chain's first run only.
	pub chain: Option<Chain>,
}

/// Decodes a row. Returns `None` for a row that is malformed or inconsistent with itself, so a
/// corrupt row only reads as a missing one and is deleted.
pub fn decode_row(key: &[u8], value: &[u8]) -> Option<Row> {
	let start = Pgno::from_be_bytes(key.try_into().ok()?);
	let row = SqliteDbOverflowData::deserialize_with_embedded_version(value).ok()?;
	let run = Run {
		start,
		len: row.len,
		step: row.step,
	};
	if start == 0 || run.len == 0 || run.step == 0 {
		return None;
	}
	let span = u64::from(run.len - 1) * u64::from(run.step);
	if span >= u64::from(MAX_RUN_SPAN) || u64::from(start) + span > u64::from(u32::MAX) {
		return None;
	}
	if row.chain_first == 0
		|| row.chain_pages == 0
		|| row.offset_in_chain.checked_add(row.len)? > row.chain_pages
	{
		return None;
	}
	let first = row.offset_in_chain == 0;
	if first != (row.chain_first == start) {
		return None;
	}
	let chain = match first {
		true => {
			let mut runs = vec![run];
			for rest in &row.rest {
				let rest = Run {
					start: rest.start,
					len: rest.len,
					step: rest.step,
				};
				let span = u64::from(rest.len.checked_sub(1)?) * u64::from(rest.step);
				if rest.start == 0
					|| rest.step == 0
					|| span >= u64::from(MAX_RUN_SPAN)
					|| u64::from(rest.start) + span > u64::from(u32::MAX)
				{
					return None;
				}
				runs.push(rest);
			}
			let chain = Chain {
				runs,
				learned_txid: row.learned_txid,
			};
			// Pages must be distinct and the length must be what the row says.
			let pages: Vec<Pgno> = chain.pages().collect();
			let distinct: std::collections::BTreeSet<Pgno> = pages.iter().copied().collect();
			if distinct.len() != pages.len() || chain.page_count() != row.chain_pages {
				return None;
			}
			Some(chain)
		}
		false => {
			if !row.rest.is_empty() {
				return None;
			}
			None
		}
	};
	Some(Row {
		run,
		chain_first: row.chain_first,
		offset_in_chain: row.offset_in_chain,
		chain_pages: row.chain_pages,
		learned_txid: row.learned_txid,
		chain,
	})
}
