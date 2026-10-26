//! Scan detection from interior B-tree pages, forward and reverse.
//!
//! Every interior page the server serves goes into a bounded shape cache of its ordered child
//! pointers. When two consecutive misses are consecutive children of one parent, ascending or
//! descending, SQLite is walking that parent's children, and the plan fetches the rest of them
//! in that direction. A point lookup reads one child per level, so it never fires the rule. A
//! scan that reaches the last child of its parent moves to the parent's sibling, through the
//! grandparent's shape, and the plan fetches that sibling too so its children are known when the
//! walk gets there.
use std::collections::{BTreeMap, BTreeSet};

use crate::page::Pgno;

/// Child pointers kept in the shape cache: 4 bytes each, plus their index.
pub const SHAPE_CHILDREN: usize = 65_536;
/// Active scans tracked at once.
const SCANS: usize = 4;
/// Parents whose last missed child is remembered.
const LAST_MISSES: usize = 256;
/// Children a new scan fetches first; each continuation doubles it, as the window does, so a
/// short range scan reads little past its end. Small shapes start at 32 children; reverse
/// continuations stay at 256 to bound tail over-read on descending LIMIT queries.
/// Write-only shapes start at 32 and continue at most 64: retaining a freshly written
/// parent should not introduce a large speculative tail on a short read.
const FIRST_BATCH: usize = 256;

#[derive(Debug, Clone, Copy)]
struct Scan {
	parent: Pgno,
	forward: bool,
	/// The next child index the scan has not fetched yet.
	frontier: i64,
	/// Children the next plan fetches.
	batch: usize,
	tick: u64,
}

#[derive(Default)]
pub struct ScanDetector {
	shapes: BTreeMap<Pgno, Vec<Pgno>>,
	written_shapes: BTreeSet<Pgno>,
	order: BTreeMap<u64, Pgno>,
	shape_ticks: BTreeMap<Pgno, u64>,
	child_index: BTreeMap<Pgno, (Pgno, u32)>,
	children: usize,
	last_child_miss: BTreeMap<Pgno, (u32, u64)>,
	scans: Vec<Scan>,
	tick: u64,
}

impl ScanDetector {
	/// Remembers an interior page's child pointers, replacing what it held before.
	pub fn interior(&mut self, pgno: Pgno, children: Vec<Pgno>) {
		self.forget(pgno);
		if children.len() > SHAPE_CHILDREN {
			return;
		}
		self.tick += 1;
		for (index, child) in children.iter().enumerate() {
			self.child_index.insert(*child, (pgno, index as u32));
		}
		self.children += children.len();
		self.order.insert(self.tick, pgno);
		self.shape_ticks.insert(pgno, self.tick);
		self.shapes.insert(pgno, children);
		while self.children > SHAPE_CHILDREN {
			let Some((_, oldest)) = self.order.pop_first() else {
				break;
			};
			self.shape_ticks.remove(&oldest);
			self.drop_shape(oldest);
		}
	}

	/// A write supplies pointers without observing a reader's traversal. Start newly
	/// available reverse scans conservatively so a short LIMIT query does not pay for
	/// a full batch merely because its parent stayed in SQLite's pager.
	pub(crate) fn committed_interior(&mut self, pgno: Pgno, children: Vec<Pgno>) {
		self.interior(pgno, children);
		if self.shapes.contains_key(&pgno) {
			self.written_shapes.insert(pgno);
		}
	}

	fn reverse_limit(&self, parent: Pgno) -> usize {
		if self.written_shapes.contains(&parent) {
			64
		} else {
			256
		}
	}

	/// The parent of `pgno` and its index among the parent's children.
	pub fn position(&self, pgno: Pgno) -> Option<(Pgno, i64)> {
		self.child_index
			.get(&pgno)
			.map(|(parent, index)| (*parent, i64::from(*index)))
	}

	/// The direction a scan walks the parent of `pgno`: 1 forward, -1 backward, none when no
	/// scan walks it.
	pub fn direction(&self, pgno: Pgno) -> Option<i64> {
		let (parent, _) = self.child_index.get(&pgno).copied()?;
		self.scans
			.iter()
			.find(|scan| scan.parent == parent)
			.map(|scan| match scan.forward {
				true => 1,
				false => -1,
			})
	}

	/// Up to `count` siblings of `pgno` under its parent at `stride` children apart.
	pub fn following(&self, pgno: Pgno, stride: i64, count: usize) -> Vec<Pgno> {
		let Some((parent, index)) = self.child_index.get(&pgno).copied() else {
			return Vec::new();
		};
		let Some(children) = self.shapes.get(&parent) else {
			return Vec::new();
		};
		let mut pages = Vec::new();
		let mut at = i64::from(index) + stride;
		while stride != 0 && pages.len() < count && at >= 0 && (at as usize) < children.len() {
			pages.push(children[at as usize]);
			at += stride;
		}
		pages
	}

	/// The interior page whose shape lists `pgno` as a child.
	pub fn parent_of(&self, pgno: Pgno) -> Option<Pgno> {
		self.child_index.get(&pgno).map(|(parent, _)| *parent)
	}

	/// Forgets a page's shape, as when a commit rewrites it.
	pub fn forget(&mut self, pgno: Pgno) {
		if let Some(tick) = self.shape_ticks.remove(&pgno) {
			self.order.remove(&tick);
		}
		self.drop_shape(pgno);
	}

	fn drop_shape(&mut self, pgno: Pgno) {
		self.written_shapes.remove(&pgno);
		let Some(children) = self.shapes.remove(&pgno) else {
			return;
		};
		self.children -= children.len();
		for child in children {
			if self
				.child_index
				.get(&child)
				.is_some_and(|(parent, _)| *parent == pgno)
			{
				self.child_index.remove(&child);
			}
		}
		self.scans.retain(|scan| scan.parent != pgno);
		self.last_child_miss.remove(&pgno);
	}

	/// Forgets every shape, as after a commit rewrote most of the file.
	pub fn reset(&mut self) {
		*self = Self::default();
	}

	pub fn memory_bytes(&self) -> usize {
		self.shapes
			.values()
			.map(|children| children.capacity() * 4)
			.sum::<usize>()
			+ self.shapes.len() * 64
			+ self.written_shapes.len() * 40
			+ self.child_index.len() * 48
			+ self.order.len() * 48
			+ self.shape_ticks.len() * 48
			+ self.last_child_miss.len() * 56
			+ self.scans.capacity() * std::mem::size_of::<Scan>()
	}

	/// Feeds a miss and returns the children to prefetch, in walk order, at most `limit`.
	pub fn miss(&mut self, pgno: Pgno, limit: usize) -> Vec<Pgno> {
		let Some((parent, index)) = self.child_index.get(&pgno).copied() else {
			return Vec::new();
		};
		self.tick += 1;
		let tick = self.tick;
		let index = i64::from(index);

		// A miss at or just past an active scan's frontier continues it.
		if let Some(position) = self.scans.iter().position(|scan| {
			scan.parent == parent && {
				let step = if scan.forward { 1 } else { -1 };
				index == scan.frontier || index == scan.frontier - step
			}
		}) {
			let mut scan = self.scans[position];
			let step = if scan.forward { 1 } else { -1 };
			scan.frontier = index + step;
			scan.batch = scan.batch.saturating_mul(2).min(if scan.forward {
				usize::MAX
			} else {
				self.reverse_limit(parent)
			});
			scan.tick = tick;
			self.scans[position] = scan;
			return self.advance(position, limit);
		}

		// A miss on the first child of an exhausted scan's next sibling moves the scan there.
		if let Some(position) = self.scans.iter().position(|scan| {
			self.sibling(scan.parent, scan.forward) == Some(parent)
				&& self.exhausted(scan)
				&& index == self.first_index(parent, scan.forward)
		}) {
			let forward = self.scans[position].forward;
			let batch = self.scans[position]
				.batch
				.saturating_mul(2)
				.min(if forward {
					usize::MAX
				} else {
					self.reverse_limit(parent)
				});
			let step = if forward { 1 } else { -1 };
			self.scans[position] = Scan {
				parent,
				forward,
				frontier: index + step,
				batch,
				tick,
			};
			self.last_child_miss.insert(parent, (index as u32, tick));
			return self.advance(position, limit);
		}

		let previous = self.last_child_miss.insert(parent, (index as u32, tick));
		self.bound_last_misses();
		let Some((previous, _)) = previous else {
			return Vec::new();
		};
		let previous = i64::from(previous);
		let forward = match index - previous {
			1 => true,
			-1 => false,
			_ => return Vec::new(),
		};
		let step = if forward { 1 } else { -1 };
		if self.scans.len() >= SCANS {
			let oldest = self
				.scans
				.iter()
				.enumerate()
				.min_by_key(|(_, scan)| scan.tick)
				.map(|(position, _)| position);
			if let Some(oldest) = oldest {
				self.scans.remove(oldest);
			}
		}
		self.scans.push(Scan {
			parent,
			forward,
			frontier: index + step,
			batch: if !forward && self.written_shapes.contains(&parent) {
				32
			} else if self
				.shapes
				.get(&parent)
				.is_some_and(|children| children.len() <= 64)
			{
				32
			} else {
				FIRST_BATCH
			},
			tick,
		});
		let position = self.scans.len() - 1;
		self.advance(position, limit)
	}

	/// Plans the next children of a scan and moves its frontier past them. When the plan
	/// reaches the parent's end, the parent's next sibling is planned too.
	fn advance(&mut self, position: usize, limit: usize) -> Vec<Pgno> {
		let scan = self.scans[position];
		let Some(children) = self.shapes.get(&scan.parent) else {
			return Vec::new();
		};
		let step = if scan.forward { 1 } else { -1 };
		let limit = limit.min(scan.batch);
		let mut pages = Vec::new();
		let mut at = scan.frontier;
		while pages.len() < limit && at >= 0 && (at as usize) < children.len() {
			pages.push(children[at as usize]);
			at += step;
		}
		let reached_end = at < 0 || at as usize >= children.len();
		self.scans[position].frontier = at;
		if reached_end && pages.len() < limit {
			if let Some(sibling) = self.sibling(scan.parent, scan.forward) {
				pages.push(sibling);
			}
		}
		pages
	}

	fn exhausted(&self, scan: &Scan) -> bool {
		let Some(children) = self.shapes.get(&scan.parent) else {
			return true;
		};
		scan.frontier < 0 || scan.frontier as usize >= children.len()
	}

	fn first_index(&self, parent: Pgno, forward: bool) -> i64 {
		match forward {
			true => 0,
			false => self
				.shapes
				.get(&parent)
				.map_or(0, |children| children.len() as i64 - 1),
		}
	}

	/// The page after `pgno` among its parent's children, in the walk's direction.
	fn sibling(&self, pgno: Pgno, forward: bool) -> Option<Pgno> {
		let (parent, index) = self.child_index.get(&pgno).copied()?;
		let children = self.shapes.get(&parent)?;
		let index = match forward {
			true => index.checked_add(1)?,
			false => index.checked_sub(1)?,
		};
		children.get(index as usize).copied()
	}

	fn bound_last_misses(&mut self) {
		if self.last_child_miss.len() <= LAST_MISSES {
			return;
		}
		let oldest = self
			.last_child_miss
			.iter()
			.min_by_key(|(_, (_, tick))| *tick)
			.map(|(parent, _)| *parent);
		if let Some(oldest) = oldest {
			self.last_child_miss.remove(&oldest);
		}
	}
}


