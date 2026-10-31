//! Keeping the overflow rows exact across a commit, from the commit's dirty pages alone.
//!
//! A commit kills a chain only by touching one of its pages: SQLite frees every page of a
//! chain it drops, writes a page it reuses, and truncates the file only when `VACUUM` rewrites
//! all of it. So the rows a commit must delete are those covering a page it writes, a page it
//! frees, or a page past its new end. Each such page's covering row is resolved from what this
//! opening knows (see the module comment of [`super`]); a page it cannot resolve is looked up
//! before the commit is sent, so no row of a dead chain survives the commit's entry.
//!
//! The rows a commit puts are those of the chains its own dirty pages hold whole: a dirty cell
//! names a dirty head, and every page of the chain is dirty with the right next pointer.
//! Everything else is left unrecorded, which the invariant allows.
use std::collections::{BTreeMap, BTreeSet};

use super::chain::Chain;
use super::{Lookup, OverflowMap, RowStatus};
use crate::page::{self, BtreeKind, Geometry, OverflowRef, Pgno, Trunk};

/// The bytes a KV op costs an entry: the filesystem's charge for metadata plus its key and
/// value, and the one-byte prefix the server stores each planner key under.
pub const KV_OP_OVERHEAD: usize = 97;

/// The most pages one floor lookup spans, and so the most rows it reads.
pub const MAX_FLOOR_PAGES: u32 = 2 * super::chain::MAX_RUN_SPAN;

/// The two KV inodes beside the database file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sidecar {
	/// The recent-page rows, under their own prefix of `main.db-prefetch`.
	Recent,
	/// The overflow run rows, under their own prefix of `main.db-prefetch`.
	Overflow,
}

/// A KV op for a commit's entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KvOp {
	Put {
		sidecar: Sidecar,
		key: Vec<u8>,
		value: Vec<u8>,
	},
	Delete {
		sidecar: Sidecar,
		key: Vec<u8>,
	},
	/// Deletes every row from `start`, inclusive, to `end`, exclusive, or to the end.
	DeleteRange {
		sidecar: Sidecar,
		start: Vec<u8>,
		end: Option<Vec<u8>>,
	},
}

impl KvOp {
	/// The bytes this op adds to its entry.
	pub fn cost(&self) -> usize {
		match self {
			Self::Put { key, value, .. } => KV_OP_OVERHEAD + key.len() + value.len(),
			Self::Delete { key, .. } => KV_OP_OVERHEAD + key.len(),
			Self::DeleteRange { start, end, .. } => {
				KV_OP_OVERHEAD + start.len() + end.as_ref().map_or(0, Vec::len)
			}
		}
	}

	pub fn sidecar(&self) -> Sidecar {
		match self {
			Self::Put { sidecar, .. }
			| Self::Delete { sidecar, .. }
			| Self::DeleteRange { sidecar, .. } => *sidecar,
		}
	}
}

/// What a dirty page's bytes say, kept instead of the bytes so a staged commit of many
/// segments holds only a few bytes per page.
#[derive(Debug, Clone)]
struct Dirty {
	btree: Option<BtreeKind>,
	refs: Vec<OverflowRef>,
	next: Pgno,
	trunk: Option<Trunk>,
}

/// The dirty pages of one commit, added as they arrive.
#[derive(Debug, Default)]
pub struct CommitBuilder {
	dirty: BTreeMap<Pgno, Dirty>,
	// Written interior pages may stay in SQLite's pager and never be read remotely.
	// Retain only their bounded shape until the commit is confirmed durable.
	shapes: BTreeMap<Pgno, Vec<Pgno>>,
	shape_children: usize,
	page1: Option<(Option<Geometry>, Pgno)>,
	parsed_geometry: Option<Geometry>,
	/// Earlier segments no longer have bytes available to reparse after a geometry change.
	geometry_changed: bool,
}

impl CommitBuilder {
	/// Adds a dirty page. `geometry` is the database's page geometry.
	pub fn add(&mut self, pgno: Pgno, bytes: &[u8], geometry: Geometry) {
		if pgno == 1 {
			let declared = Geometry::from_page1(bytes);
			// Even the first staged page can change the previous map's interpretation.
			self.geometry_changed |= declared != Some(geometry);
			let free_head = page::free_list_head(bytes).map_or(0, |(head, _)| head);
			self.page1 = Some((declared, free_head));
		}
		let geometry = self
			.page1
			.and_then(|(declared, _)| declared)
			.unwrap_or(geometry);
		self.geometry_changed |= self
			.parsed_geometry
			.is_some_and(|previous| previous != geometry);
		self.parsed_geometry = Some(geometry);
		let btree = match page::classify(pgno, bytes) {
			page::PageClass::Btree(kind) => Some(kind),
			page::PageClass::Other => None,
		};
		let refs = match btree {
			Some(_) => page::overflow_refs(pgno, bytes, geometry),
			None => Vec::new(),
		};
		let trunk = match btree {
			Some(_) => None,
			None => page::free_list(bytes, geometry),
		};
		if let Some(old) = self.shapes.remove(&pgno) {
			self.shape_children -= old.len();
		}
		if let Some(children) = page::children(pgno, bytes) {
			if self.shape_children + children.len() <= crate::scan::SHAPE_CHILDREN {
				self.shape_children += children.len();
				self.shapes.insert(pgno, children);
			}
		}
		self.dirty.insert(
			pgno,
			Dirty {
				btree,
				refs,
				next: page::next(bytes).unwrap_or(0),
				trunk,
			},
		);
	}

	pub(crate) fn scan_shapes(&self, size_pages: u32) -> Option<Vec<(Pgno, Vec<Pgno>)>> {
		if self.geometry_changed || size_pages >= 1 << 24 {
			return None;
		}
		Some(
			self.shapes
				.iter()
				.filter(|(pgno, children)| {
					**pgno <= size_pages && children.iter().all(|child| *child <= size_pages)
				})
				.map(|(pgno, children)| (*pgno, children.clone()))
				.collect(),
		)
	}

	pub fn len(&self) -> usize {
		self.dirty.len()
	}

	pub fn is_empty(&self) -> bool {
		self.dirty.is_empty()
	}

	pub fn pages(&self) -> impl Iterator<Item = Pgno> + '_ {
		self.dirty.keys().copied()
	}
}

/// What applying a commit changes in the map, once the commit is durable.
#[derive(Debug, Default)]
pub struct Apply {
	purge: bool,
	reset: bool,
	dead: Vec<Pgno>,
	put: Vec<Chain>,
	no_row: Vec<Pgno>,
	/// Pages on the free list at the new head that this commit put there or wrote.
	free: Vec<Pgno>,
	dirty: Vec<Pgno>,
	free_head: Option<Pgno>,
	trunks: BTreeMap<Pgno, Pgno>,
	geometry: Option<Geometry>,
	new_size: u32,
	wrote_pending: bool,
	refs: Vec<OverflowRef>,
}

impl Apply {
	pub(crate) fn freed_pages(&self) -> impl Iterator<Item = Pgno> + '_ {
		self.free.iter().copied()
	}

	pub fn purges(&self) -> bool {
		self.purge
	}
}

/// The overflow ops of one commit and their effect on the map.
#[derive(Debug, Default)]
pub struct OverflowPlan {
	pub ops: Vec<KvOp>,
	/// Rows this commit puts and deletes, for metrics.
	pub puts: usize,
	pub deletes: usize,
	pub apply: Apply,
}

/// The pages the new free list holds in the trunks this commit wrote, and the free list's
/// shape for the map.
struct FreeList {
	newly_free: BTreeSet<Pgno>,
	head: Option<Pgno>,
	trunks: BTreeMap<Pgno, Pgno>,
}

/// The outcome of following a commit's free list.
enum FreeListWalk {
	Followed(FreeList),
	/// A trunk this opening has not seen must be read first.
	Needs(Vec<Lookup>),
	/// The free list loops or names a page that cannot be a trunk; SQLite's own integrity
	/// check would fail, so the commit purges the rows instead of trusting it.
	Malformed,
}

impl OverflowMap {
	/// Plans the overflow ops for a commit of `builder` that takes the file from `old_size` to
	/// `new_size` pages within `budget` bytes of entry, or returns the lookups it needs first.
	/// Lookups go through [`OverflowMap::apply`] or [`OverflowMap::observe_trunk`], and the
	/// commit is planned again.
	pub fn plan_commit(
		&self,
		builder: &CommitBuilder,
		old_size: u32,
		new_size: u32,
		txid: u64,
		budget: usize,
	) -> Result<OverflowPlan, Vec<Lookup>> {
		let proven = |pgno: Pgno, dirty: &Dirty| -> bool {
			dirty.btree.is_some() && (pgno == 1 || new_size < 1 << 24)
		};

		// A commit that rewrites every page, as `VACUUM` does, is planned from scratch: purge
		// every row and record every chain the commit holds whole. So is a truncation, which
		// SQLite does only as part of such a rewrite.
		let rewrites_all = new_size > 0
			&& builder.dirty.len() == new_size as usize
			&& builder.dirty.keys().next() == Some(&1)
			&& builder.dirty.keys().next_back() == Some(&new_size);
		if builder.geometry_changed || rewrites_all || new_size < old_size {
			return Ok(self.plan_purge(builder, new_size, txid, budget));
		}

		let free = match self.follow_free_list(builder) {
			FreeListWalk::Followed(free) => free,
			FreeListWalk::Needs(lookups) => return Err(lookups),
			FreeListWalk::Malformed => {
				return Ok(self.plan_purge(builder, new_size, txid, budget));
			}
		};

		// Every page this commit writes or frees may end the chain covering it.
		let mut suspects: BTreeSet<Pgno> = builder
			.dirty
			.keys()
			.copied()
			.filter(|pgno| *pgno <= old_size)
			.collect();
		suspects.extend(
			free.newly_free
				.iter()
				.copied()
				.filter(|pgno| *pgno <= old_size),
		);

		let mut dead: BTreeSet<Pgno> = BTreeSet::new();
		let mut unknown: Vec<Pgno> = Vec::new();
		for &pgno in &suspects {
			match self.status(pgno, old_size) {
				RowStatus::None => {}
				RowStatus::Unknown => unknown.push(pgno),
				RowStatus::Chain(head) => {
					// A page this commit frees, or writes as anything but the same page of the
					// same chain, ends the chain.
					let consistent = !free.newly_free.contains(&pgno)
						&& builder.dirty.get(&pgno).is_some_and(|dirty| {
							let expected = self.chain(head).and_then(|chain| {
								chain
									.position(pgno)
									.map(|position| chain.next_after(position))
							});
							!proven(pgno, dirty) && expected == Some(dirty.next)
						});
					if !consistent {
						dead.insert(head);
					}
				}
			}
		}
		if !unknown.is_empty() {
			return Err(merge_floors(&unknown));
		}

		// Chains whose every page this commit wrote, named by a cell it wrote.
		let mut put: Vec<Chain> = Vec::new();
		let mut in_new_chain: BTreeSet<Pgno> = BTreeSet::new();
		let mut refs = Vec::new();
		for (&pgno, dirty) in &builder.dirty {
			// A page this commit wrote and then freed keeps its cells, which name dead chains.
			if !proven(pgno, dirty) || free.newly_free.contains(&pgno) {
				continue;
			}
			for overflow in &dirty.refs {
				refs.push(*overflow);
				if !builder.dirty.contains_key(&overflow.head) {
					continue;
				}
				let survives = matches!(
					self.status(overflow.head, old_size),
					RowStatus::Chain(head) if head == overflow.head && !dead.contains(&head)
				);
				if survives {
					continue;
				}
				if let Some(pages) =
					walk_dirty(builder, *overflow, &free.newly_free, new_size, &proven)
				{
					if pages.iter().any(|pgno| in_new_chain.contains(pgno)) {
						continue;
					}
					if let Some(chain) = Chain::from_pages(&pages, txid) {
						in_new_chain.extend(pages);
						put.push(chain);
					}
				}
			}
		}

		// A chain this commit writes takes its pages from whatever held them. Any other chain
		// still covering one of them is dead, even when the shared pages kept their next
		// pointers, as when a freed chain's pages become the tail of a longer one.
		for chain in &put {
			for pgno in chain.pages() {
				if let RowStatus::Chain(head) = self.status(pgno, old_size)
					&& self.chain(head) != Some(chain)
				{
					dead.insert(head);
				}
			}
		}

		// Chains a read walked whole, untouched by this commit and not on the free list.
		let pending: Vec<Chain> = self
			.pending_puts
			.iter()
			.filter(|chain| {
				chain.pages().all(|pgno| {
					!builder.dirty.contains_key(&pgno)
						&& !free.newly_free.contains(&pgno)
						&& !self.is_free(pgno)
						&& !in_new_chain.contains(&pgno)
						&& pgno <= new_size
				})
			})
			.cloned()
			.collect();

		// Deletes first, so a put may reuse a key a dead chain held.
		let mut delete_keys: BTreeSet<[u8; 4]> = self.pending_deletes.clone();
		for head in &dead {
			if let Some(chain) = self.chain(*head) {
				delete_keys.extend(chain.keys());
			}
		}
		let mut ops: Vec<KvOp> = delete_keys
			.iter()
			.map(|key| KvOp::Delete {
				sidecar: Sidecar::Overflow,
				key: key.to_vec(),
			})
			.collect();
		let delete_cost: usize = ops.iter().map(KvOp::cost).sum();
		if delete_cost > budget {
			return Ok(self.plan_purge(builder, new_size, txid, budget));
		}
		let deletes = ops.len();

		let mut spent = delete_cost;
		let mut recorded = Vec::new();
		let mut puts = 0;
		for chain in put.into_iter().chain(pending) {
			let Ok(rows) = chain.rows() else {
				continue;
			};
			let cost: usize = rows
				.iter()
				.map(|(key, value)| KV_OP_OVERHEAD + key.len() + value.len())
				.sum();
			if spent + cost > budget {
				continue;
			}
			spent += cost;
			puts += rows.len();
			for (key, value) in rows {
				ops.push(KvOp::Put {
					sidecar: Sidecar::Overflow,
					key: key.to_vec(),
					value,
				});
			}
			recorded.push(chain);
		}

		// After the commit no row covers a page it wrote unless a recorded or surviving chain
		// holds it, nor a page it freed.
		let recorded_pages: BTreeSet<Pgno> = recorded.iter().flat_map(Chain::pages).collect();
		let mut no_row: Vec<Pgno> = Vec::new();
		for &pgno in builder.dirty.keys() {
			if recorded_pages.contains(&pgno) {
				continue;
			}
			let survives = matches!(
				self.status(pgno, old_size),
				RowStatus::Chain(head) if !dead.contains(&head)
			);
			if !survives {
				no_row.push(pgno);
			}
		}
		no_row.extend(free.newly_free.iter().copied());

		Ok(OverflowPlan {
			ops,
			puts,
			deletes,
			apply: Apply {
				purge: false,
				reset: false,
				dead: dead.into_iter().collect(),
				put: recorded,
				no_row,
				free: free.newly_free.into_iter().collect(),
				dirty: builder.dirty.keys().copied().collect(),
				free_head: free.head,
				trunks: free.trunks,
				geometry: builder.page1.and_then(|(geometry, _)| geometry),
				new_size,
				wrote_pending: true,
				refs,
			},
		})
	}

	/// Purges every row and records the chains the commit holds whole, within `budget`.
	pub(crate) fn plan_purge(
		&self,
		builder: &CommitBuilder,
		new_size: u32,
		txid: u64,
		budget: usize,
	) -> OverflowPlan {
		let proven =
			|pgno: Pgno, dirty: &Dirty| dirty.btree.is_some() && (pgno == 1 || new_size < 1 << 24);
		let purge = KvOp::DeleteRange {
			sidecar: Sidecar::Overflow,
			start: Vec::new(),
			end: None,
		};
		if builder.geometry_changed {
			// Page bytes still commit normally. The derived index is optional: never publish
			// guesses from staged segments parsed using a different page/reserved-byte size.
			return OverflowPlan {
				ops: vec![purge],
				deletes: 1,
				apply: Apply {
					purge: true,
					reset: true,
					new_size,
					geometry: builder.page1.and_then(|(geometry, _)| geometry),
					free_head: builder.page1.map(|(_, head)| head),
					..Apply::default()
				},
				..OverflowPlan::default()
			};
		}

		let mut spent = purge.cost();
		let mut ops = vec![purge];
		let free = self.dirty_free_pages(builder);
		let mut recorded = Vec::new();
		let mut seen: BTreeSet<Pgno> = BTreeSet::new();
		let mut puts = 0;
		let mut refs = Vec::new();
		for (&pgno, dirty) in &builder.dirty {
			if !proven(pgno, dirty) || free.contains(&pgno) {
				continue;
			}
			for overflow in &dirty.refs {
				refs.push(*overflow);
				let Some(pages) = walk_dirty(builder, *overflow, &free, new_size, &proven) else {
					continue;
				};
				if pages.iter().any(|pgno| seen.contains(pgno)) {
					continue;
				}
				let Some(chain) = Chain::from_pages(&pages, txid) else {
					continue;
				};
				let Ok(rows) = chain.rows() else {
					continue;
				};
				let cost: usize = rows
					.iter()
					.map(|(key, value)| KV_OP_OVERHEAD + key.len() + value.len())
					.sum();
				if spent + cost > budget {
					continue;
				}
				spent += cost;
				puts += rows.len();
				seen.extend(pages);
				for (key, value) in rows {
					ops.push(KvOp::Put {
						sidecar: Sidecar::Overflow,
						key: key.to_vec(),
						value,
					});
				}
				recorded.push(chain);
			}
		}
		OverflowPlan {
			ops,
			puts,
			deletes: 1,
			apply: Apply {
				purge: true,
				put: recorded,
				free: free.iter().copied().collect(),
				dirty: builder.dirty.keys().copied().collect(),
				new_size,
				geometry: builder.page1.and_then(|(geometry, _)| geometry),
				free_head: builder.page1.map(|(_, head)| head),
				wrote_pending: true,
				refs,
				..Apply::default()
			},
		}
	}

	/// Follows the free list this commit leaves, from page 1, and returns the pages its
	/// written trunks hold. A trunk this commit did not write is unchanged and so are its
	/// leaves; one this opening has not seen is read first, for its next pointer.
	fn follow_free_list(&self, builder: &CommitBuilder) -> FreeListWalk {
		let Some((_, head)) = builder.page1 else {
			// SQLite rewrites page 1 whenever the free list changes, since it holds its length.
			return FreeListWalk::Followed(FreeList {
				newly_free: BTreeSet::new(),
				head: None,
				trunks: BTreeMap::new(),
			});
		};
		let mut newly_free = BTreeSet::new();
		let mut trunks = BTreeMap::new();
		let mut at = head;
		let mut steps = 0usize;
		while at != 0 {
			steps += 1;
			if steps > 1 << 20 || trunks.contains_key(&at) {
				return FreeListWalk::Malformed;
			}
			let next = match builder.dirty.get(&at) {
				Some(dirty) => {
					let Some(trunk) = dirty.trunk.as_ref() else {
						return FreeListWalk::Malformed;
					};
					newly_free.insert(at);
					newly_free.extend(trunk.leaves.iter().copied());
					trunk.next
				}
				// A trunk at the current head that this commit did not write is unchanged, and
				// so are the leaves it lists, which were free already.
				None if self.is_trunk(at) => {
					match self
						.trunk_next(at)
						.or_else(|| self.read_trunks.get(&at).map(|trunk| trunk.next))
					{
						Some(next) => next,
						None => return FreeListWalk::Needs(vec![Lookup::Trunk(at)]),
					}
				}
				// A page that becomes a trunk without its bytes changing, or one this opening
				// has not seen as a trunk: it and its leaves may be newly free.
				None => match self.read_trunks.get(&at) {
					Some(trunk) => {
						newly_free.insert(at);
						newly_free.extend(trunk.leaves.iter().copied());
						trunk.next
					}
					None => return FreeListWalk::Needs(vec![Lookup::Trunk(at)]),
				},
			};
			trunks.insert(at, next);
			at = next;
		}
		FreeListWalk::Followed(FreeList {
			newly_free,
			head: Some(head),
			trunks,
		})
	}

	/// Pages on the free list at the new head that sit in trunks this commit wrote, followed from
	/// page 1 for as far as the trunks are known without a read. Every page SQLite frees goes
	/// into the first trunk or becomes it, so this holds every page the commit wrote and freed.
	fn dirty_free_pages(&self, builder: &CommitBuilder) -> BTreeSet<Pgno> {
		let mut free = BTreeSet::new();
		let Some((_, head)) = builder.page1 else {
			return free;
		};
		let mut at = head;
		let mut seen = BTreeSet::new();
		while at != 0 && seen.insert(at) {
			let next = match builder.dirty.get(&at) {
				Some(dirty) => match dirty.trunk.as_ref() {
					Some(trunk) => {
						free.insert(at);
						free.extend(trunk.leaves.iter().copied());
						trunk.next
					}
					None => break,
				},
				None => match self
					.trunk_next(at)
					.or_else(|| self.read_trunks.get(&at).map(|trunk| trunk.next))
				{
					Some(next) => next,
					None => break,
				},
			};
			at = next;
		}
		free
	}

	/// Remembers a trunk page a commit had to read to follow its new free list.
	pub fn observe_trunk(&mut self, pgno: Pgno, bytes: &[u8]) {
		if let Some(trunk) = page::free_list(bytes, self.geometry) {
			self.read_trunks.insert(pgno, trunk);
		}
	}

	/// Applies a durable commit's plan.
	pub fn committed(&mut self, apply: Apply) {
		if apply.reset {
			self.reset();
		}
		if apply.purge {
			self.set_empty();
		}
		for head in &apply.dead {
			self.remove_chain(*head);
		}
		if apply.wrote_pending {
			self.pending_deletes.clear();
			self.pending_puts.clear();
		}
		self.clear_walks();
		self.read_trunks.clear();
		self.clear_trusted_heads();
		self.truncate(apply.new_size);
		for pgno in &apply.dirty {
			self.clear_free(*pgno);
		}
		for pgno in &apply.free {
			if *pgno > 1 && *pgno <= apply.new_size {
				self.mark_free(*pgno);
			}
		}
		if let Some(geometry) = apply.geometry {
			self.set_geometry(geometry);
		}
		for pgno in &apply.no_row {
			if *pgno > 1 && *pgno <= apply.new_size {
				self.clear_no_row(*pgno);
				self.mark_no_row(*pgno);
			}
		}
		match apply.free_head {
			Some(head) => self.set_free_list(Some(head), apply.trunks),
			None if apply.purge => self.set_free_list(None, BTreeMap::new()),
			None => {}
		}
		for chain in apply.put {
			self.insert_chain(chain);
		}
		for overflow in apply.refs {
			self.remember_head(overflow);
		}
	}
}

/// The pages of the chain `overflow` names, when this commit wrote all of them as a chain: each
/// page dirty, not a B-tree page, not free, inside the file, linked by its next pointer, and
/// the last one ending the chain.
fn walk_dirty(
	builder: &CommitBuilder,
	overflow: OverflowRef,
	free: &BTreeSet<Pgno>,
	new_size: u32,
	proven: &dyn Fn(Pgno, &Dirty) -> bool,
) -> Option<Vec<Pgno>> {
	if overflow.pages == 0 || overflow.pages > new_size {
		return None;
	}
	let mut pages = Vec::with_capacity(overflow.pages as usize);
	let mut at = overflow.head;
	for index in 0..overflow.pages {
		if at <= 1 || at > new_size || free.contains(&at) {
			return None;
		}
		let dirty = builder.dirty.get(&at)?;
		if proven(at, dirty) {
			return None;
		}
		pages.push(at);
		let last = index + 1 == overflow.pages;
		match last {
			true => {
				if dirty.next != 0 {
					return None;
				}
			}
			false => at = dirty.next,
		}
	}
	Some(pages)
}

/// Floor lookups covering `pages`, merged where their windows overlap. A floor lookup reads
/// every row of its range in one scan, so a merged range never spans more than
/// [`MAX_FLOOR_PAGES`] pages, which bounds its rows to the scan's limit.
fn merge_floors(pages: &[Pgno]) -> Vec<Lookup> {
	let mut lookups: Vec<Lookup> = Vec::new();
	for &pgno in pages {
		let Lookup::Floor { first, last } = Lookup::floor(pgno) else {
			continue;
		};
		if let Some(Lookup::Floor {
			first: prev_first,
			last: prev_last,
		}) = lookups.last_mut()
			&& first <= prev_last.saturating_add(1)
			&& last - *prev_first < MAX_FLOOR_PAGES
		{
			*prev_last = (*prev_last).max(last);
			continue;
		}
		lookups.push(Lookup::Floor { first, last });
	}
	lookups
}


