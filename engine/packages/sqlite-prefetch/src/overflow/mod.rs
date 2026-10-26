//! The overflow map: what this opening knows of the database's overflow rows in `main.db-prefetch`.
//!
//! The rows obey one invariant at every committed head: every row belongs to a live chain and
//! describes it exactly, and a chain has either all of its rows or none. Commits keep it by
//! deleting the rows of every chain they kill in their own entry (see [`commit`]); rows are
//! only ever put for a whole chain the commit itself wrote, or one a read walked completely.
//!
//! From the invariant this opening derives, for any page, whether a row covers it:
//! - a page in a cached chain is covered by that chain's row;
//! - a page whose bytes at the current head prove it is a B-tree page, a page on the free
//!   list, a page past the end of the file, and the head of a chain whose head row is absent
//!   are covered by no row, which [`OverflowMap::no_row`] records;
//! - every other page is unknown, and only a floor lookup of the rows can tell.
//!
//! The map never decides what a read returns. Every page it names is read at the current head,
//! and a page whose bytes disagree with its chain is dropped from the reply, so a wrong row
//! costs an over-read at most.
pub mod chain;
pub mod commit;

use std::collections::{BTreeMap, BTreeSet};

use crate::bitset::PageBits;
use crate::page::{self, Geometry, OverflowRef, Pgno};
use chain::{Chain, MAX_RUN_SPAN};

/// Rows of whole chains kept in memory per open database: about 256 KiB.
pub const ROW_CACHE_ROWS: usize = 4_096;

/// Chain heads remembered from the cells of served pages, with their page counts.
const KNOWN_HEADS: usize = 8_192;

/// Chains of unrecorded heads followed through the pages a read returns, to learn them.
const WALKS: usize = 64;

/// Head lookups one served reply may queue; the rest are read when their head is missed.
const LOOKUPS_PER_REPLY: usize = 256;

/// What a read of the overflow rows asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lookup {
	/// The row keyed by a chain's head page.
	Head(Pgno),
	/// Every row whose run starts in `first..=last`.
	Floor { first: Pgno, last: Pgno },
	/// A free-list trunk page a commit must follow, read at the current head and given to
	/// [`OverflowMap::observe_trunk`].
	Trunk(Pgno),
}

impl Lookup {
	/// A floor lookup that resolves which row, if any, covers `pgno`.
	pub fn floor(pgno: Pgno) -> Self {
		Self::Floor {
			first: pgno.saturating_sub(MAX_RUN_SPAN - 1).max(1),
			last: pgno,
		}
	}
}

/// The rows a [`Lookup`] read.
#[derive(Debug, Clone)]
pub enum LookupResult {
	Head {
		head: Pgno,
		value: Option<Vec<u8>>,
	},
	Floor {
		first: Pgno,
		last: Pgno,
		rows: Vec<(Vec<u8>, Vec<u8>)>,
	},
}

/// Which row covers a page, as far as this opening knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStatus {
	/// The chain with this head page has a row covering the page.
	Chain(Pgno),
	/// No row covers the page.
	None,
	/// Only a lookup can tell.
	Unknown,
}

/// A chain of an unrecorded head, followed page by page as reads return it.
struct Walk {
	pages: Vec<Pgno>,
	total: u32,
	/// The next pointer of the last page walked.
	pending_next: Pgno,
}

#[derive(Default)]
pub struct OverflowMap {
	chains: BTreeMap<Pgno, Chain>,
	/// Least recently used chain first, by a tick per touch.
	chain_ticks: BTreeMap<Pgno, u64>,
	ticks: BTreeMap<u64, Pgno>,
	next_tick: u64,
	cached_rows: usize,
	/// The first page of every run of a cached chain, to its chain's head.
	runs: BTreeMap<Pgno, Pgno>,
	no_row: PageBits,
	/// Pages on the free list at the current head. A free page keeps whatever bytes it held
	/// before SQLite freed it, so its cells and next pointer name dead chains: nothing is ever
	/// learned from one.
	free: PageBits,
	/// If bounded free-page knowledge was lost, reads cannot safely learn new chains until reset.
	read_learning_disabled: bool,
	/// Chain heads named by cells of pages SQLite itself read at the current head, so known to
	/// be live. Only these may start a walk or mark an absent head row as "no rows". A page
	/// the server prefetched may be a free page this opening does not know is free.
	trusted_heads: BTreeSet<Pgno>,
	/// Every row the inode holds is a chain this opening cached: a database this opening
	/// created, or one a commit purged since, with the chains its commits put since. Evicting
	/// any chain ends it, since the evicted rows would then be unknown.
	empty: bool,
	known_heads: BTreeMap<Pgno, u32>,
	known_head_order: BTreeMap<u64, Pgno>,
	known_head_ticks: BTreeMap<Pgno, u64>,
	/// Lookups to read once the current reply is out.
	pending_lookups: Vec<Lookup>,
	pending_heads: BTreeSet<Pgno>,
	/// Chains suspected stale: a trusted head H read another chain's middle row, so that
	/// chain's first row F is being read. Keyed by F, holding every such H. Cleared at each
	/// commit, since the suspicion holds only at the head it was found at.
	stale_heads: BTreeMap<Pgno, BTreeSet<Pgno>>,
	walks: BTreeMap<Pgno, Walk>,
	/// Keys of stale rows a read found, deleted by the next commit.
	pub(crate) pending_deletes: BTreeSet<[u8; 4]>,
	/// Chains a read walked completely, put by the next commit.
	pub(crate) pending_puts: Vec<Chain>,
	/// The free list's first trunk at the current head, from page 1.
	free_head: Option<Pgno>,
	/// The next pointer of every free-list trunk this opening has seen at the current head,
	/// 4 bytes a trunk.
	trunks: BTreeMap<Pgno, Pgno>,
	/// Trunks a commit read to follow its new free list, at the current head. They may be
	/// newly free, so nothing is concluded about their pages until the commit is planned.
	pub(crate) read_trunks: BTreeMap<Pgno, page::Trunk>,
	geometry: Geometry,
	pub(crate) stats: MapStats,
}

/// Counts the testing VFS reports.
#[derive(Debug, Default, Clone, Copy)]
pub struct MapStats {
	pub mismatches: u64,
	pub learned_chains: u64,
}

impl OverflowMap {
	pub fn new(empty: bool) -> Self {
		Self {
			empty,
			..Self::default()
		}
	}

	pub fn geometry(&self) -> Geometry {
		self.geometry
	}

	pub fn cached_rows(&self) -> usize {
		self.cached_rows
	}

	/// Bytes this map holds in memory, roughly, for the process cache budget.
	pub fn memory_bytes(&self) -> usize {
		let chain_bytes = |chain: &Chain| {
			std::mem::size_of::<Chain>() + chain.runs.capacity() * std::mem::size_of::<chain::Run>()
		};
		self.chains.values().map(chain_bytes).sum::<usize>()
			+ self.chains.len() * 64
			+ self.chain_ticks.len() * 48
			+ (self.ticks.len() + self.runs.len()) * 48
			+ self.no_row.bytes()
			+ self.free.bytes()
			+ self.trusted_heads.len() * 40
			+ (self.known_heads.len() + self.known_head_ticks.len()) * 48
			+ self.known_head_order.len() * 48
			+ self.pending_lookups.capacity() * std::mem::size_of::<Lookup>()
			+ self.pending_heads.len() * 40
			+ self.pending_deletes.len() * 40
			+ self.pending_puts.capacity() * std::mem::size_of::<Chain>()
			+ self.pending_puts.iter().map(chain_bytes).sum::<usize>()
			+ self.walks.len() * 64
			+ self
				.walks
				.values()
				.map(|walk| walk.pages.capacity() * 4)
				.sum::<usize>()
			+ self.trunks.len() * 48
			+ self.read_trunks.len() * 64
			+ self
				.read_trunks
				.values()
				.map(|trunk| trunk.leaves.capacity() * 4)
				.sum::<usize>()
	}

	/// Which row covers `pgno` in a database of `size_pages` pages.
	pub fn status(&self, pgno: Pgno, size_pages: u32) -> RowStatus {
		if pgno <= 1 || pgno > size_pages {
			return RowStatus::None;
		}
		if let Some(head) = self.covering(pgno) {
			return RowStatus::Chain(head);
		}
		match self.empty || self.no_row.get(pgno) {
			true => RowStatus::None,
			false => RowStatus::Unknown,
		}
	}

	/// The cached chain with a run covering `pgno`.
	pub fn covering(&self, pgno: Pgno) -> Option<Pgno> {
		let lowest = pgno.saturating_sub(MAX_RUN_SPAN - 1);
		for (_, head) in self.runs.range(lowest..=pgno).rev() {
			let chain = self.chains.get(head)?;
			if chain.position(pgno).is_some() {
				return Some(*head);
			}
		}
		None
	}

	pub fn chain(&self, head: Pgno) -> Option<&Chain> {
		self.chains.get(&head)
	}

	/// Whether a cell on a page SQLite read at the current head names `head`.
	pub fn is_trusted_head(&self, head: Pgno) -> bool {
		self.trusted_heads.contains(&head)
	}

	/// Forgets lookups that were cancelled before their rows arrived, so a later request may
	/// queue them again. Nothing else this opening knows depends on them.
	pub fn lookups_cancelled(&mut self) {
		self.pending_heads.clear();
		self.stale_heads.clear();
	}

	/// The page count a served cell gave for the chain at `head`.
	pub fn known_head(&self, head: Pgno) -> Option<u32> {
		self.known_heads.get(&head).copied()
	}

	/// Marks a chain as used, so it is evicted last.
	pub fn touch(&mut self, head: Pgno) {
		if let Some(old) = self.chain_ticks.get(&head).copied() {
			self.ticks.remove(&old);
			let tick = self.tick();
			self.ticks.insert(tick, head);
			self.chain_ticks.insert(head, tick);
		}
	}

	fn tick(&mut self) -> u64 {
		self.next_tick += 1;
		self.next_tick
	}

	/// Caches a whole chain whose rows exist at the current head. A chain that overlaps a
	/// cached one contradicts the invariant, so both are dropped and their rows deleted.
	pub(crate) fn insert_chain(&mut self, chain: Chain) {
		let head = chain.head();
		if self.chains.contains_key(&head) {
			self.remove_chain(head);
		}
		let overlaps: BTreeSet<Pgno> = chain
			.pages()
			.filter_map(|pgno| self.covering(pgno))
			.collect();
		if !overlaps.is_empty() {
			for other in overlaps {
				self.poison(other);
			}
			self.pending_deletes.extend(chain.keys());
			self.stats.mismatches += 1;
			return;
		}
		for run in &chain.runs {
			self.runs.insert(run.start, head);
		}
		for pgno in chain.pages() {
			self.no_row.clear(pgno);
		}
		self.cached_rows += chain.runs.len();
		let tick = self.tick();
		self.ticks.insert(tick, head);
		self.chain_ticks.insert(head, tick);
		self.chains.insert(head, chain);
		self.evict();
	}

	fn evict(&mut self) {
		while self.cached_rows > ROW_CACHE_ROWS {
			let Some((_, head)) = self.ticks.pop_first() else {
				break;
			};
			self.chain_ticks.remove(&head);
			self.forget_chain(head);
			self.empty = false;
		}
	}

	/// Removes a chain from memory only: its rows stay as they are, so its pages become
	/// unknown again.
	fn forget_chain(&mut self, head: Pgno) -> Option<Chain> {
		let chain = self.chains.remove(&head)?;
		for run in &chain.runs {
			self.runs.remove(&run.start);
		}
		self.cached_rows -= chain.runs.len();
		Some(chain)
	}

	/// Removes a chain that a commit kills or a read found stale, together with its
	/// bookkeeping. The caller decides what becomes of its rows.
	pub(crate) fn remove_chain(&mut self, head: Pgno) -> Option<Chain> {
		if let Some(tick) = self.chain_ticks.remove(&head) {
			self.ticks.remove(&tick);
		}
		self.forget_chain(head)
	}

	/// Drops a chain whose rows contradict the file and deletes them with the next commit.
	pub(crate) fn poison(&mut self, head: Pgno) {
		if let Some(chain) = self.remove_chain(head) {
			self.pending_deletes.extend(chain.keys());
		}
		self.stats.mismatches += 1;
		crate::metrics::MISMATCH_TOTAL.inc();
	}

	pub(crate) fn mark_no_row(&mut self, pgno: Pgno) {
		if self.covering(pgno).is_none() {
			self.no_row.set(pgno);
		}
	}

	pub(crate) fn clear_no_row(&mut self, pgno: Pgno) {
		self.no_row.clear(pgno);
	}

	/// Forgets everything learned about rows, as after a commit whose outcome is unknown. The
	/// rows themselves still obey the invariant; only this opening's knowledge of them goes.
	pub(crate) fn reset(&mut self) {
		let geometry = self.geometry;
		let stats = self.stats;
		*self = Self::default();
		self.geometry = geometry;
		self.stats = stats;
	}

	/// The lookups queued while pages were served, to read once the reply is out.
	pub fn take_lookups(&mut self) -> Vec<Lookup> {
		std::mem::take(&mut self.pending_lookups)
	}

	fn queue_head(&mut self, head: Pgno) {
		if self.pending_lookups.len() < LOOKUPS_PER_REPLY && self.pending_heads.insert(head) {
			self.pending_lookups.push(Lookup::Head(head));
		}
	}

	pub(crate) fn remember_head(&mut self, overflow: OverflowRef) {
		if let Some(old) = self.known_head_ticks.get(&overflow.head).copied() {
			self.known_head_order.remove(&old);
		}
		let tick = self.tick();
		self.known_head_order.insert(tick, overflow.head);
		self.known_head_ticks.insert(overflow.head, tick);
		self.known_heads.insert(overflow.head, overflow.pages);
		while self.known_heads.len() > KNOWN_HEADS {
			let Some((_, head)) = self.known_head_order.pop_first() else {
				break;
			};
			self.known_head_ticks.remove(&head);
			self.known_heads.remove(&head);
			self.trusted_heads.remove(&head);
		}
	}

	/// Learns what a page read at the current head shows. `size_pages` is the file's size;
	/// `requested` marks a page SQLite asked for, whose cells are therefore live.
	pub fn observe(&mut self, pgno: Pgno, bytes: &[u8], size_pages: u32, requested: bool) {
		if self.free.get(pgno) {
			return;
		}
		if pgno == 1 {
			if let Some(geometry) = Geometry::from_page1(bytes) {
				self.geometry = geometry;
			}
			self.free_head = page::free_list_head(bytes).map(|(head, _)| head);
			self.trunks.clear();
		}
		if page::proves_btree(pgno, bytes, size_pages).is_some() {
			if requested && let Some(head) = self.covering(pgno) {
				self.poison(head);
			}
			self.mark_no_row(pgno);
			for overflow in page::overflow_refs(pgno, bytes, self.geometry) {
				self.observe_ref(overflow, size_pages, requested);
			}
			return;
		}
		self.observe_other(pgno, bytes, size_pages, requested);
	}

	fn observe_ref(&mut self, overflow: OverflowRef, size_pages: u32, trusted: bool) {
		let trusted = trusted && !self.read_learning_disabled;
		if trusted || !self.trusted_heads.contains(&overflow.head) {
			self.remember_head(overflow);
		}
		if trusted {
			self.trusted_heads.insert(overflow.head);
		}
		match self.status(overflow.head, size_pages) {
			RowStatus::Chain(head) if head == overflow.head => {
				let pages = self.chains.get(&head).map(Chain::page_count);
				if trusted && pages != Some(overflow.pages) {
					self.poison(head);
				}
			}
			RowStatus::Chain(other) => {
				// The cell's head sits inside another chain: that chain is stale.
				if trusted {
					self.poison(other);
				}
				self.queue_head(overflow.head);
			}
			RowStatus::None if trusted => self.start_walk(overflow),
			RowStatus::None => {}
			// A one-page chain is its head, which the cell already names: its row tells
			// nothing a plan could use.
			RowStatus::Unknown if overflow.pages == 1 => {}
			RowStatus::Unknown => self.queue_head(overflow.head),
		}
	}

	/// A page that is not provably a B-tree page: an overflow page, a free-list page, or a
	/// page of a database too large to tell.
	fn observe_other(&mut self, pgno: Pgno, bytes: &[u8], size_pages: u32, requested: bool) {
		if self.is_trunk(pgno) {
			if let Some(trunk) = page::free_list(bytes, self.geometry) {
				self.mark_free(pgno);
				for &leaf in &trunk.leaves {
					if leaf > 1 && leaf <= size_pages {
						self.mark_free(leaf);
					}
				}
				self.trunks.insert(pgno, trunk.next);
			}
			return;
		}
		let Some(next) = page::next(bytes) else {
			return;
		};
		// Walks follow only pages SQLite read: a prefetched page may be a free page whose
		// stale next pointer would extend a walk into a dead chain.
		if requested {
			self.advance_walks(pgno, next);
		}
	}

	/// Marks a page as on the free list at the current head, and so covered by no row.
	pub(crate) fn mark_free(&mut self, pgno: Pgno) {
		self.mark_no_row(pgno);
		if self.free.set(pgno) {
			self.read_learning_disabled = true;
			self.trusted_heads.clear();
			self.walks.clear();
			self.pending_puts.clear();
		}
	}

	pub(crate) fn is_free(&self, pgno: Pgno) -> bool {
		self.free.get(pgno)
	}

	pub(crate) fn clear_free(&mut self, pgno: Pgno) {
		self.free.clear(pgno);
	}

	/// Whether the free list at the current head has `pgno` as a trunk, following the trunks
	/// this opening has seen from page 1.
	pub(crate) fn is_trunk(&self, pgno: Pgno) -> bool {
		let mut at = self.free_head;
		let mut steps = 0;
		while let Some(trunk) = at.filter(|trunk| *trunk != 0) {
			if trunk == pgno {
				return true;
			}
			steps += 1;
			if steps > self.trunks.len() {
				return false;
			}
			at = self.trunks.get(&trunk).copied();
		}
		false
	}

	fn start_walk(&mut self, overflow: OverflowRef) {
		if self.read_learning_disabled
			|| self.walks.contains_key(&overflow.head)
			|| self.walks.len() >= WALKS
		{
			return;
		}
		if overflow.pages == 0 {
			return;
		}
		self.walks.insert(
			overflow.head,
			Walk {
				pages: Vec::new(),
				total: overflow.pages,
				pending_next: 0,
			},
		);
	}

	/// Follows every walk expecting `pgno`, whose next pointer is `next`. A walk that
	/// completes becomes a chain the next commit records.
	fn advance_walks(&mut self, pgno: Pgno, next: Pgno) {
		let mut finished = Vec::new();
		for (&head, walk) in self.walks.iter_mut() {
			let expected = match walk.pages.last() {
				None => head,
				Some(_) => walk.next_expected(),
			};
			if expected != pgno {
				continue;
			}
			walk.pages.push(pgno);
			walk.pending_next = next;
			if walk.pages.len() as u32 == walk.total {
				finished.push((head, next == 0));
			} else if next == 0 {
				finished.push((head, false));
			}
		}
		for (head, complete) in finished {
			let Some(walk) = self.walks.remove(&head) else {
				continue;
			};
			if !complete {
				continue;
			}
			if let Some(chain) = Chain::from_pages(&walk.pages, 0) {
				for pgno in chain.pages() {
					self.no_row.set(pgno);
				}
				self.pending_puts.push(chain);
				self.stats.learned_chains += 1;
			}
		}
	}

	/// Drops every walk: a commit may have changed the pages they followed.
	pub(crate) fn clear_walks(&mut self) {
		self.walks.clear();
	}

	/// Applies rows a lookup read at the current head. Returns lookups still needed to see
	/// whole chains: the first run of a chain a floor lookup only found the middle of.
	pub fn apply(&mut self, result: LookupResult, size_pages: u32) -> Vec<Lookup> {
		let mut more = Vec::new();
		match result {
			LookupResult::Head { head, value } => {
				self.pending_heads.remove(&head);
				// The suspicion this lookup was read for, if any. F's row decides it: when F's
				// chain has a run at each H, it shares a live chain's page and goes whole;
				// otherwise each H's row is an orphan and goes alone.
				let suspects = self.stale_heads.remove(&head).unwrap_or_default();
				let decoded = value
					.as_ref()
					.and_then(|value| chain::decode_row(&head.to_be_bytes(), value))
					.and_then(|row| row.chain);
				if !suspects.is_empty() {
					match decoded {
						Some(chain)
							if suspects
								.iter()
								.all(|h| chain.runs.iter().any(|run| run.start == *h)) =>
						{
							self.remove_chain(head);
							self.pending_deletes.extend(chain.keys());
						}
						Some(_) | None => {
							for h in &suspects {
								self.pending_deletes.insert(h.to_be_bytes());
							}
						}
					}
					self.stats.mismatches += 1;
					return more;
				}
				let Some(value) = value else {
					// By the invariant a live head without its first row has no rows at all. Only
					// a head a page SQLite read names is known to be live.
					if self.trusted_heads.contains(&head) && self.covering(head).is_none() {
						self.no_row.set(head);
						if let Some(pages) = self.known_head(head) {
							self.start_walk(OverflowRef { head, pages });
						}
					}
					return more;
				};
				match chain::decode_row(&head.to_be_bytes(), &value) {
					Some(row) => match row.chain {
						Some(chain) => {
							if self.chains.get(&head) != Some(&chain) {
								let expected = self
									.known_head(head)
									.filter(|_| self.trusted_heads.contains(&head));
								if expected.is_some_and(|pages| pages != chain.page_count()) {
									self.pending_deletes.extend(chain.keys());
									self.stats.mismatches += 1;
								} else {
									self.insert_chain(chain);
								}
							}
						}
						None => {
							// A run of another chain starts at this head. When a cell SQLite read
							// names the head, that chain shares a live chain's page and is stale:
							// its first row is read and the whole chain deleted. A head only a
							// prefetched page named may come from a dead cell on a free page,
							// so it proves nothing and the row is left alone.
							if self.trusted_heads.contains(&head) {
								self.stale_heads
									.entry(row.chain_first)
									.or_default()
									.insert(head);
								more.push(Lookup::Head(row.chain_first));
							}
						}
					},
					None => {
						self.pending_deletes.insert(head.to_be_bytes());
					}
				}
			}
			LookupResult::Floor { first, last, rows } => {
				let mut covered: Vec<chain::Run> = Vec::new();
				for (key, value) in rows {
					let Ok(key4) = <[u8; 4]>::try_from(key.as_slice()) else {
						continue;
					};
					let Some(row) = chain::decode_row(&key, &value) else {
						self.pending_deletes.insert(key4);
						continue;
					};
					let start = row.run.start;
					if start < first || start > last {
						continue;
					}
					covered.push(row.run);
					match row.chain {
						Some(chain) => {
							let doomed =
								chain.keys().any(|key| self.pending_deletes.contains(&key));
							if !self.chains.contains_key(&start) && !doomed {
								self.insert_chain(chain);
							}
						}
						None => {
							if !self.chains.contains_key(&row.chain_first) {
								more.push(Lookup::Head(row.chain_first));
							}
						}
					}
				}
				// Every page whose floor window lies inside the scanned range is resolved.
				let resolved_from = first.saturating_add(MAX_RUN_SPAN - 1);
				let resolved_from = match first <= 1 {
					true => 1,
					false => resolved_from,
				};
				for pgno in resolved_from.max(2)..=last.min(size_pages) {
					let in_run = covered.iter().any(|run| run.index_of(pgno).is_some());
					if !in_run {
						self.mark_no_row(pgno);
					}
				}
			}
		}
		more
	}

	/// The next pointer of a free-list trunk this opening has seen.
	pub(crate) fn trunk_next(&self, pgno: Pgno) -> Option<Pgno> {
		self.trunks.get(&pgno).copied()
	}

	pub(crate) fn set_free_list(&mut self, head: Option<Pgno>, trunks: BTreeMap<Pgno, Pgno>) {
		self.free_head = head;
		self.trunks = trunks;
	}

	pub(crate) fn set_geometry(&mut self, geometry: Geometry) {
		self.geometry = geometry;
	}

	pub(crate) fn set_empty(&mut self) {
		self.chains.clear();
		self.chain_ticks.clear();
		self.ticks.clear();
		self.runs.clear();
		self.cached_rows = 0;
		self.no_row.reset();
		self.pending_deletes.clear();
		self.pending_puts.clear();
		self.known_heads.clear();
		self.known_head_order.clear();
		self.known_head_ticks.clear();
		self.trusted_heads.clear();
		self.stale_heads.clear();
		self.empty = true;
	}

	pub(crate) fn truncate(&mut self, size_pages: u32) {
		self.no_row.truncate(size_pages);
		self.free.truncate(size_pages);
	}

	/// Forgets which heads are known live: a commit may have removed the cells that named them.
	pub(crate) fn clear_trusted_heads(&mut self) {
		self.trusted_heads.clear();
		self.stale_heads.clear();
	}
}

impl Walk {
	fn next_expected(&self) -> Pgno {
		self.pending_next
	}
}


