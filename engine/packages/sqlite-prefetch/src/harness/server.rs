//! The page server of the testing VFS: the library driven through [`crate::driver`] exactly as
//! the page protocol server drives it, or one of the reference strategies it is scored
//! against.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bytes::Bytes;
use futures::executor::block_on;

use super::readahead::{Kernel, MainV2, ReadAhead, V3Adaptive};
use super::store::MemStore;
use crate::driver::{self, Store};
use crate::overflow::commit::Sidecar;
use crate::page::{self, Geometry, Pgno};
use crate::prefetcher::{Ablation, Config, OpenRows, Prefetcher, Source};
use crate::recency;
use crate::window::Window;

/// What plans the pages a reply carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
	/// The library.
	Library,
	/// Requested pages only.
	None,
	/// The contiguous window only.
	Window,
	/// Knows every page SQLite will read and sends the next ones the client lacks.
	Oracle,
	/// Plain Linux read-ahead in the client; see [`Kernel`].
	Kernel,
	/// The v2 VFS predictor on main, in the client; see [`MainV2`].
	MainV2,
	/// The adaptive v3 client read-ahead the library replaced; see [`V3Adaptive`].
	V3Adaptive,
	/// The library with one part switched off.
	LibraryWithout(Part),
}

/// A part of the library an ablation run switches off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
	Window,
	Scan,
	Recency,
	OverflowMap,
	LeafPayloads,
}

impl Part {
	pub const ALL: [Part; 5] = [
		Part::Window,
		Part::Scan,
		Part::Recency,
		Part::OverflowMap,
		Part::LeafPayloads,
	];

	fn ablation(self) -> Ablation {
		let mut ablation = Ablation::default();
		match self {
			Part::Window => ablation.window = true,
			Part::Scan => ablation.scan = true,
			Part::Recency => ablation.recency = true,
			Part::OverflowMap => ablation.overflow_map = true,
			Part::LeafPayloads => ablation.leaf_payloads = true,
		}
		ablation
	}
}

impl Strategy {
	pub fn name(self) -> &'static str {
		match self {
			Self::Library => "library",
			Self::None => "none",
			Self::Window => "window",
			Self::Oracle => "oracle",
			Self::Kernel => "kernel",
			Self::MainV2 => "main-v2",
			Self::V3Adaptive => "v3-adapt",
			Self::LibraryWithout(Part::Window) => "-window",
			Self::LibraryWithout(Part::Scan) => "-scan",
			Self::LibraryWithout(Part::Recency) => "-recency",
			Self::LibraryWithout(Part::OverflowMap) => "-ovf-map",
			Self::LibraryWithout(Part::LeafPayloads) => "-leaf",
		}
	}

	/// Whether the library plans the replies, with all its parts or without one.
	pub fn is_library(self) -> bool {
		match self {
			Self::Library | Self::LibraryWithout(_) => true,
			Self::None
			| Self::Window
			| Self::Oracle
			| Self::Kernel
			| Self::MainV2
			| Self::V3Adaptive => false,
		}
	}

	/// A fresh client-side read-ahead rule, for the strategies that are one.
	fn read_ahead(self) -> Option<Box<dyn ReadAhead>> {
		match self {
			Self::Kernel => Some(Box::new(Kernel::default())),
			Self::MainV2 => Some(Box::new(MainV2::default())),
			Self::V3Adaptive => Some(Box::new(V3Adaptive::default())),
			Self::Library | Self::LibraryWithout(_) | Self::None | Self::Window | Self::Oracle => {
				None
			}
		}
	}
}

/// Budget for a commit's KV ops: the room a 1 MiB page commit leaves in a 4 MiB entry.
pub const COMMIT_KV_BUDGET: usize = 3 << 20;

/// A commit outcome the test injects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitFault {
	/// The commit is refused before it runs: nothing is written.
	Refused,
	/// The commit runs but its reply is lost: the server cannot tell whether it landed.
	Unknown,
}

pub struct Server {
	pub strategy: Strategy,
	pub store: Arc<MemStore>,
	pub prefetcher: Option<Prefetcher>,
	window: Window,
	/// The client-side rule of a client read-ahead strategy, fresh at each opening.
	read_ahead: Option<Box<dyn ReadAhead>>,
	/// The oracle's knowledge: every page SQLite reads, in order.
	pub future: Vec<Pgno>,
	clock_ms: u64,
	request_id: u64,
	pub faults: Vec<CommitFault>,
	pub commit_lookup_rounds: u64,
	pub rows_put: u64,
	pub rows_deleted: u64,
}

impl Server {
	pub fn new(strategy: Strategy, store: Arc<MemStore>) -> Self {
		Self {
			strategy,
			store,
			prefetcher: None,
			window: Window::default(),
			read_ahead: strategy.read_ahead(),
			future: Vec::new(),
			clock_ms: 0,
			request_id: 0,
			faults: Vec::new(),
			commit_lookup_rounds: 0,
			rows_put: 0,
			rows_deleted: 0,
		}
	}

	/// Opens the database as a new owner would: the open prefetch returns the recent rows.
	pub fn open(&mut self) {
		if let Some(prefetcher) = self.prefetcher.take() {
			prefetcher.close();
		}
		let recent = self.store.rows(Sidecar::Recent);
		let overflow_empty = self.store.rows(Sidecar::Overflow).is_empty();
		let rows = OpenRows {
			early: recent.get(recency::EARLY_KEY).cloned(),
			frequent: recent.get(recency::FREQUENT_KEY).cloned(),
			// Only a database this opening created may skip floor lookups; an existing one
			// starts from what its rows say.
			overflow_empty: overflow_empty && self.store.size_pages() == 0,
		};
		let ablation = match self.strategy {
			Strategy::LibraryWithout(part) => part.ablation(),
			Strategy::Library
			| Strategy::None
			| Strategy::Window
			| Strategy::Oracle
			| Strategy::Kernel
			| Strategy::MainV2
			| Strategy::V3Adaptive => Ablation::default(),
		};
		self.prefetcher = Some(Prefetcher::open(
			rows,
			self.store.size_pages(),
			self.clock_ms,
			Config {
				ablation,
				..Config::default()
			},
		));
		self.window = Window::default();
		self.read_ahead = self.strategy.read_ahead();
	}

	/// Serves one page request. `position` is how many reads SQLite made before this one, for
	/// the oracle; `buffered` says which pages the client holds.
	pub fn get_pages(
		&mut self,
		requested: &[Pgno],
		opening: bool,
		position: usize,
		buffered: &dyn Fn(Pgno) -> bool,
	) -> Vec<(Pgno, Option<Bytes>, Source)> {
		self.request_id += 1;
		self.clock_ms += 1;
		let size = self.store.size_pages();
		match self.strategy {
			Strategy::Library | Strategy::LibraryWithout(_) => {
				let prefetcher = self.prefetcher.as_mut().expect("open before reading");
				prefetcher.head(size);
				let reply = block_on(driver::read(
					prefetcher,
					self.store.as_ref(),
					self.request_id,
					requested,
					self.clock_ms,
					opening,
				))
				.expect("in-memory reads do not fail");
				// The after-reply lookups overlap the client's next round trip; here they run
				// before the next request, as the server's spawned task would finish first.
				if !reply.after_reply.is_empty() {
					block_on(driver::run_lookups(
						prefetcher,
						self.store.as_ref(),
						reply.after_reply,
						"after_reply",
					))
					.expect("in-memory reads do not fail");
				}
				let requested_count = requested.len();
				let mut sources = reply.sources.into_iter();
				reply
					.pages
					.into_iter()
					.enumerate()
					.map(|(index, (pgno, bytes))| {
						let source = match index < requested_count {
							true => Source::Requested,
							false => sources.next().unwrap_or(Source::Window),
						};
						(pgno, bytes, source)
					})
					.collect()
			}
			Strategy::None => self.read_exact(requested, &[]),
			Strategy::Window => {
				let mut extra = Vec::new();
				for &pgno in requested {
					if let Some(span) = self.window.miss(pgno, size) {
						extra.extend(span.pages().map(|page| (page, Source::Window)));
					}
				}
				extra.truncate(1_024usize.saturating_sub(requested.len()));
				self.read_exact(requested, &extra)
			}
			Strategy::Oracle => {
				let mut seen: BTreeSet<Pgno> = requested.iter().copied().collect();
				let mut extra = Vec::new();
				for &pgno in self.future.iter().skip(position + 1) {
					if extra.len() + requested.len() >= 1_024 {
						break;
					}
					if pgno > size || buffered(pgno) || !seen.insert(pgno) {
						continue;
					}
					extra.push((pgno, Source::Window));
				}
				self.read_exact(requested, &extra)
			}
			Strategy::Kernel | Strategy::MainV2 | Strategy::V3Adaptive => {
				let read_ahead = self.read_ahead.as_mut().expect("client read-ahead rule");
				let mut extra = Vec::new();
				for &pgno in requested {
					for page in read_ahead.miss(pgno, size) {
						if !buffered(page) && !requested.contains(&page) {
							extra.push((page, Source::Window));
						}
					}
				}
				let mut reply = self.read_exact(requested, &extra);
				if self.strategy == Strategy::MainV2 {
					self.expand_overflow(&mut reply);
				}
				reply
			}
		}
	}

	/// Appends the overflow chains the reply's B-tree pages name, as main's engine did for
	/// every page read the v2 VFS sent: each chain is walked to its end, one batch of reads
	/// per chain position, up to 2,048 pages per reply. It does not know which pages the
	/// client holds, so it may send them again.
	fn expand_overflow(&mut self, reply: &mut Vec<(Pgno, Option<Bytes>, Source)>) {
		const MAX_EXPANSION_PAGES: usize = 2_048;
		let size = self.store.size_pages();
		let geometry = self
			.store
			.page(1)
			.and_then(|page1| Geometry::from_page1(&page1))
			.unwrap_or_default();
		let mut seen: BTreeSet<Pgno> = reply.iter().map(|(pgno, _, _)| *pgno).collect();
		let mut frontier: Vec<Pgno> = Vec::new();
		for (pgno, bytes, _) in reply.iter() {
			let Some(bytes) = bytes else {
				continue;
			};
			for overflow in page::overflow_refs(*pgno, bytes, geometry) {
				if overflow.head <= size && seen.insert(overflow.head) {
					frontier.push(overflow.head);
				}
			}
		}
		let mut budget = MAX_EXPANSION_PAGES;
		while !frontier.is_empty() && budget > 0 {
			frontier.truncate(budget);
			budget -= frontier.len();
			let bytes =
				block_on(self.store.read_pages(&frontier)).expect("in-memory reads do not fail");
			let mut next_frontier = Vec::new();
			for (pgno, bytes) in frontier.into_iter().zip(bytes) {
				let Some(bytes) = bytes else {
					continue;
				};
				if let Some(next) = page::next(&bytes)
					.filter(|next| *next != 0 && *next <= size && seen.insert(*next))
				{
					next_frontier.push(next);
				}
				reply.push((pgno, Some(bytes), Source::Chain));
			}
			frontier = next_frontier;
		}
	}

	/// Shows a client-side read-ahead rule a read the client served itself. Returns the pages
	/// the rule fetches at once, without SQLite waiting, which costs a request of its own.
	pub fn client_hit(
		&mut self,
		pgno: Pgno,
		bytes: &[u8],
		buffered: &dyn Fn(Pgno) -> bool,
	) -> Vec<(Pgno, Option<Bytes>, Source)> {
		let size = self.store.size_pages();
		let Some(read_ahead) = self.read_ahead.as_mut() else {
			return Vec::new();
		};
		let extra: Vec<(Pgno, Source)> = read_ahead
			.hit(pgno, bytes, size)
			.into_iter()
			.filter(|page| !buffered(*page))
			.map(|page| (page, Source::Window))
			.collect();
		if extra.is_empty() {
			return Vec::new();
		}
		self.request_id += 1;
		self.clock_ms += 1;
		self.read_exact(&[], &extra)
	}

	fn read_exact(
		&mut self,
		requested: &[Pgno],
		extra: &[(Pgno, Source)],
	) -> Vec<(Pgno, Option<Bytes>, Source)> {
		let mut pgnos: Vec<Pgno> = requested.to_vec();
		let mut seen: BTreeSet<Pgno> = requested.iter().copied().collect();
		let mut sources = vec![Source::Requested; requested.len()];
		for (pgno, source) in extra {
			if seen.insert(*pgno) {
				pgnos.push(*pgno);
				sources.push(*source);
			}
		}
		let bytes = block_on(self.store.read_pages(&pgnos)).expect("in-memory reads do not fail");
		pgnos
			.into_iter()
			.zip(bytes)
			.zip(sources)
			.filter(|((_, bytes), source)| bytes.is_some() || *source == Source::Requested)
			.map(|((pgno, bytes), source)| (pgno, bytes, source))
			.collect()
	}

	/// Commits dirty pages. Returns false when an injected fault refused it.
	pub fn commit(&mut self, dirty: &BTreeMap<Pgno, Bytes>, size_pages: u32) -> bool {
		self.clock_ms += 1;
		let fault = match self.faults.is_empty() {
			true => None,
			false => Some(self.faults.remove(0)),
		};
		let Some(prefetcher) = self
			.prefetcher
			.as_mut()
			.filter(|_| self.strategy.is_library())
		else {
			if fault == Some(CommitFault::Refused) {
				return false;
			}
			self.store.commit(dirty, size_pages, &[]);
			return true;
		};
		prefetcher.head(self.store.size_pages());
		let mut builder = prefetcher.commit_builder();
		for (pgno, bytes) in dirty {
			prefetcher.add_dirty(&mut builder, *pgno, bytes);
		}
		let before = self.store.counts().depot_requests();
		let txid = self.store.counts().commits + 1;
		let plan = block_on(driver::plan_commit(
			prefetcher,
			self.store.as_ref(),
			&builder,
			size_pages,
			txid,
			COMMIT_KV_BUDGET,
		))
		.expect("in-memory reads are never refused");
		self.commit_lookup_rounds += self.store.counts().depot_requests() - before;
		match fault {
			Some(CommitFault::Refused) => {
				prefetcher.commit_failed();
				false
			}
			Some(CommitFault::Unknown) => {
				// It landed, but the server never learns so.
				self.store.commit(dirty, size_pages, &plan.ops);
				prefetcher.commit_failed();
				true
			}
			None => {
				self.rows_put += plan.overflow_puts as u64;
				self.rows_deleted += plan.overflow_deletes as u64;
				self.store.commit(dirty, size_pages, &plan.ops);
				prefetcher.committed(plan);
				true
			}
		}
	}

	pub fn clock_ms(&self) -> u64 {
		self.clock_ms
	}

	pub fn advance_clock(&mut self, ms: u64) {
		self.clock_ms += ms;
	}
}
