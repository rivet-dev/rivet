//! Leaf payloads: the overflow chains of the rows a walk is about to read, sent before
//! SQLite misses them.
//!
//! The server sees a payload miss as a miss on a chain head that a served leaf's cell names.
//! Two payload misses in one table a few chains apart in leaf order, as a scan reads every row
//! or an index lookup reads every third, set the walk's stride. Each further miss one stride
//! past what the last reply covered continues the walk and doubles the batch, so a reply
//! carries the chains of the next rows at that stride. A miss anywhere else starts over. A
//! query that skips payloads never misses a head, and a point lookup misses one, so neither
//! carries anything extra; a query that stops early, as a LIMIT does, reads at most one batch
//! past its end.
use std::collections::{BTreeMap, VecDeque};

use crate::page::{OverflowRef, Pgno};
use crate::scan::ScanDetector;

/// Leaves and heads remembered for payload walks.
const REMEMBERED: usize = 8_192;
/// Tables with a walk remembered.
const TABLES: usize = 256;
/// Chains the first batch of a walk carries.
const FIRST_BATCH: usize = 8;
/// Chains one reply carries at most for one walk.
const MAX_BATCH: usize = 256;
/// The widest stride, in chains, still read as one walk.
const MAX_STRIDE: usize = 8;
/// How far, in chains, a miss may land from the last one and still be measured.
const MEASURE: usize = 4 * MAX_BATCH;

/// A table's walk over its rows' chains, in leaf order.
#[derive(Debug, Clone, Copy)]
struct Walk {
	/// The leaf and chain index of the last payload miss.
	last: (Pgno, usize),
	/// Walk direction: forward through cells and leaves, or backward.
	forward: bool,
	/// Chains between consecutive rows the walk reads; zero until two misses set it.
	stride: usize,
	batch: usize,
	/// Strides the last reply carried past `last`.
	sent: usize,
}

#[derive(Default)]
pub(crate) struct LeafPayloads {
	/// The leaf whose cell names each chain head.
	owner: BTreeMap<Pgno, Pgno>,
	owner_order: VecDeque<Pgno>,
	/// The chains each remembered leaf's cells name, in cell order.
	refs: BTreeMap<Pgno, Vec<OverflowRef>>,
	refs_order: VecDeque<Pgno>,
	walks: BTreeMap<Pgno, Walk>,
	walk_order: VecDeque<Pgno>,
	/// The chains this reply should carry, and the table whose walk asked for them.
	pending: Vec<OverflowRef>,
	pending_table: Option<Pgno>,
}

impl LeafPayloads {
	/// Remembers a served leaf's chains, in cell order.
	pub fn remember(&mut self, leaf: Pgno, refs: &[OverflowRef]) {
		for overflow in refs {
			if self.owner.insert(overflow.head, leaf).is_none() {
				self.owner_order.push_back(overflow.head);
			}
		}
		while self.owner_order.len() > REMEMBERED {
			if let Some(head) = self.owner_order.pop_front() {
				self.owner.remove(&head);
			}
		}
		if self.refs.insert(leaf, refs.to_vec()).is_none() {
			self.refs_order.push_back(leaf);
		}
		while self.refs_order.len() > REMEMBERED {
			if let Some(old) = self.refs_order.pop_front() {
				self.refs.remove(&old);
			}
		}
	}

	/// The leaf whose cell names `head`, when it is a remembered chain head.
	pub fn owner(&self, head: Pgno) -> Option<Pgno> {
		self.owner.get(&head).copied()
	}

	/// Starts a reply: nothing is pending yet.
	pub fn clear(&mut self) {
		self.pending.clear();
		self.pending_table = None;
	}

	/// Records how many of the pending chains the reply carried, at least in part, so the
	/// walk's next miss is measured from where the reply really ended.
	pub fn delivered(&mut self, chains: usize) {
		if let Some(walk) = self
			.pending_table
			.and_then(|table| self.walks.get_mut(&table))
		{
			walk.sent = chains;
		}
	}

	/// The chains this reply should carry.
	pub fn pending(&self) -> &[OverflowRef] {
		&self.pending
	}

	pub fn reset(&mut self) {
		*self = Self::default();
	}

	pub fn memory_bytes(&self) -> usize {
		self.owner.len() * 48
			+ self
				.refs
				.values()
				.map(|refs| refs.len() * 8 + 48)
				.sum::<usize>()
			+ self.walks.len() * 64
	}

	/// Records a payload miss on `head` in `table` and plans the chains this reply carries.
	pub fn miss(&mut self, head: Pgno, table: Pgno, scan: &ScanDetector) {
		let Some(leaf) = self.owner(head) else {
			return;
		};
		let Some(cell) = self
			.refs
			.get(&leaf)
			.and_then(|refs| refs.iter().position(|overflow| overflow.head == head))
		else {
			return;
		};
		let here = (leaf, cell);
		let previous = self.walks.get(&table).copied();
		let measured = previous.and_then(|walk| {
			let forward = self.distance(walk.last, here, true, scan);
			let backward = self.distance(walk.last, here, false, scan);
			match (forward, backward) {
				(Some(steps), _) => Some((steps, true)),
				(None, Some(steps)) => Some((steps, false)),
				(None, None) => None,
			}
		});
		let walk = match (previous, measured) {
			// One stride past what the last reply carried: the walk read all of it.
			(Some(walk), Some((steps, forward)))
				if walk.stride > 0
					&& forward == walk.forward
					&& steps == walk.stride * (walk.sent + 1) =>
			{
				Walk {
					last: here,
					forward,
					stride: walk.stride,
					batch: (walk.batch * 2).min(MAX_BATCH),
					sent: 0,
				}
			}
			// Inside what the last reply carried, as when the reply cap cut a chain short, or
			// a second miss in the same chain: the walk goes on from here at the same batch.
			(Some(walk), measured)
				if walk.stride > 0
					&& (here == walk.last
						|| measured.is_some_and(|(steps, forward)| {
							forward == walk.forward && steps <= walk.stride * walk.sent
						})) =>
			{
				Walk {
					last: here,
					sent: 0,
					..walk
				}
			}
			// A short step from the last miss sets the stride of a new walk.
			(_, Some((steps, forward))) if (1..=MAX_STRIDE).contains(&steps) => Walk {
				last: here,
				forward,
				stride: steps,
				batch: FIRST_BATCH,
				sent: 0,
			},
			_ => Walk {
				last: here,
				forward: true,
				stride: 0,
				batch: 0,
				sent: 0,
			},
		};
		let mut walk = walk;
		if walk.stride > 0 {
			let chains = self.ahead(here, walk.forward, walk.stride, walk.batch, scan);
			walk.sent = chains.len();
			self.pending.extend(chains);
			self.pending_table = Some(table);
		}
		if !self.walks.contains_key(&table) {
			self.walk_order.push_back(table);
		}
		self.walks.insert(table, walk);
		while self.walk_order.len() > TABLES {
			if let Some(old) = self.walk_order.pop_front() {
				self.walks.remove(&old);
			}
		}
	}

	/// The chains `count` strides ahead of `from`, one stride apart.
	fn ahead(
		&self,
		from: (Pgno, usize),
		forward: bool,
		stride: usize,
		count: usize,
		scan: &ScanDetector,
	) -> Vec<OverflowRef> {
		let mut chains = Vec::new();
		let mut steps = 0;
		for overflow in self.cells(from, forward, scan).take(stride * count) {
			steps += 1;
			if steps % stride == 0 {
				chains.push(overflow);
			}
		}
		chains
	}

	/// How many chains `to` lies after `from` in the walk's order, if within [`MEASURE`].
	fn distance(
		&self,
		from: (Pgno, usize),
		to: (Pgno, usize),
		forward: bool,
		scan: &ScanDetector,
	) -> Option<usize> {
		let (to_leaf, to_cell) = to;
		let target = self.refs.get(&to_leaf)?.get(to_cell)?.head;
		self.cells(from, forward, scan)
			.take(MEASURE)
			.position(|overflow| overflow.head == target)
			.map(|index| index + 1)
	}

	/// The chains after `from` in leaf order: the rest of its leaf's cells, then each
	/// sibling leaf's, as far as the remembered leaves and the shape cache reach.
	fn cells<'a>(
		&'a self,
		from: (Pgno, usize),
		forward: bool,
		scan: &'a ScanDetector,
	) -> impl Iterator<Item = OverflowRef> + 'a {
		let (leaf, cell) = from;
		let step: i64 = match forward {
			true => 1,
			false => -1,
		};
		let mut current = Some((leaf, cell as i64 + step));
		std::iter::from_fn(move || {
			loop {
				let (leaf, cell) = current?;
				let refs = self.refs.get(&leaf)?;
				if cell >= 0 && (cell as usize) < refs.len() {
					current = Some((leaf, cell + step));
					return Some(refs[cell as usize]);
				}
				let next = scan.following(leaf, step, 1).first().copied()?;
				let next_refs = self.refs.get(&next)?;
				let start = match forward {
					true => 0,
					false => next_refs.len() as i64 - 1,
				};
				current = Some((next, start));
			}
		})
	}
}
