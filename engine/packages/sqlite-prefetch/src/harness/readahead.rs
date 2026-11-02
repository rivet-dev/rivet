//! Client-side read-ahead rules the library is compared against: plain Linux read-ahead, the
//! v2 VFS predictor on main, and the adaptive v3 VFS read-ahead the library replaced.
//!
//! Each rule sees every read SQLite makes, hits included, as the VFS it models did, and on a
//! miss names the pages to fetch with the missed one. The client holds the pages it fetched
//! until the database reopens, so a rule never fetches a page the client already holds.
use std::collections::BTreeMap;

use crate::page::Pgno;

/// A client-side read-ahead rule.
pub trait ReadAhead: Send {
	/// Records a read the client served from its own pages. Returns pages to fetch at once
	/// without SQLite waiting, as the kernel's asynchronous read-ahead does.
	fn hit(&mut self, pgno: Pgno, bytes: &[u8], size_pages: u32) -> Vec<Pgno>;

	/// Records a miss and returns the pages to fetch with it, in order.
	fn miss(&mut self, pgno: Pgno, size_pages: u32) -> Vec<Pgno>;
}

/// The page class the v2 and v3 VFS kept separate trackers for: B-tree pages by their type
/// byte, anything else as an overflow page. A missed page has no bytes yet and counts as a
/// B-tree page, as both VFS versions did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
	Btree,
	Overflow,
}

fn classify(pgno: Pgno, bytes: &[u8]) -> Class {
	let header = match pgno {
		1 => 100,
		_ => 0,
	};
	match bytes.get(header) {
		Some(0x02 | 0x05 | 0x0a | 0x0d) => Class::Btree,
		_ => Class::Overflow,
	}
}

/// Linux on-demand read-ahead for one sequential reader of a file (`mm/readahead.c`), with
/// the default 128 KiB maximum: 32 pages of 4 KiB.
///
/// The kernel keeps one window per open file: where it starts, how many pages it covers,
/// and how many of them at its end are the asynchronous part. A miss on page one, or on the
/// page right after the previous read, opens a window sized from the request: one page
/// rounds up to one and is multiplied by four, so the first window is four pages. The
/// kernel marks the first page of the asynchronous part. When a read reaches the marked
/// page, it fetches the next window without waiting and grows it, four times while the
/// window is under a sixteenth of the maximum and two times after that, up to the maximum:
/// 4, 8, 16, 32 pages. The new window is all asynchronous, so its first page carries the
/// next mark. A miss on the page right after the last window, when the reader outran the
/// asynchronous fetch, grows the window the same way; here its second page carries the mark,
/// where the kernel marks the missed page and starts the next window in the same read. Any
/// other miss is random: the page is
/// read alone. The kernel leaves its window alone on a random read; this model drops it,
/// which differs only when a later miss lands exactly where the old window would have
/// continued.
///
/// The kernel's request counts here as a round trip even when SQLite does not wait on it.
#[derive(Debug, Default)]
pub struct Kernel {
	window: Option<KernelWindow>,
	previous: Option<Pgno>,
}

#[derive(Debug, Clone, Copy)]
struct KernelWindow {
	start: Pgno,
	size: u32,
	async_size: u32,
}

impl KernelWindow {
	fn marker(&self) -> Pgno {
		self.start + self.size - self.async_size
	}
}

/// The largest window: 128 KiB of 4 KiB pages.
const KERNEL_MAX_PAGES: u32 = 32;

/// The first window for a request of `pages` pages.
fn kernel_initial_size(pages: u32) -> u32 {
	let size = pages.next_power_of_two();
	if size <= KERNEL_MAX_PAGES / 32 {
		size * 4
	} else if size <= KERNEL_MAX_PAGES / 4 {
		size * 2
	} else {
		KERNEL_MAX_PAGES
	}
}

/// The window after one of `size` pages.
fn kernel_next_size(size: u32) -> u32 {
	if size < KERNEL_MAX_PAGES / 16 {
		size * 4
	} else if size <= KERNEL_MAX_PAGES / 2 {
		size * 2
	} else {
		KERNEL_MAX_PAGES
	}
}

fn span(first: Pgno, count: u32, size_pages: u32) -> Vec<Pgno> {
	(first..first.saturating_add(count))
		.filter(|pgno| *pgno <= size_pages)
		.collect()
}

impl ReadAhead for Kernel {
	fn hit(&mut self, pgno: Pgno, _bytes: &[u8], size_pages: u32) -> Vec<Pgno> {
		self.previous = Some(pgno);
		let Some(window) = self.window.filter(|window| window.marker() == pgno) else {
			return Vec::new();
		};
		let size = kernel_next_size(window.size);
		let next = KernelWindow {
			start: window.start + window.size,
			size,
			async_size: size,
		};
		self.window = Some(next);
		span(next.start, next.size, size_pages)
	}

	fn miss(&mut self, pgno: Pgno, size_pages: u32) -> Vec<Pgno> {
		let previous = self.previous.replace(pgno);
		let expected = self.window.map(|window| window.start + window.size);
		let size = if expected == Some(pgno) {
			Some(kernel_next_size(
				self.window.map_or(0, |window| window.size),
			))
		} else if pgno == 1 || previous.is_some_and(|previous| previous + 1 == pgno) {
			Some(kernel_initial_size(1))
		} else {
			None
		};
		// A miss reads its window at once; the pages after the missed one are the
		// asynchronous part, so the next page carries the mark.
		let window = size.map(|size| KernelWindow {
			start: pgno,
			size,
			async_size: size - 1,
		});
		self.window = window;
		match window {
			Some(window) => span(pgno + 1, window.size - 1, size_pages),
			None => Vec::new(),
		}
	}
}

/// Pages the v2 VFS on main predicted per miss outside a forward scan, and its byte cap.
const V2_DEPTH: usize = 64;
const V2_MAX_BYTES: usize = 256 * 1024;
/// The same inside a forward scan.
const V2_SCAN_DEPTH: usize = 256;
const V2_SCAN_MAX_BYTES: usize = 1024 * 1024;
/// A forward-scan score at or above this reads with the scan depth.
const SCAN_SCORE_THRESHOLD: i32 = 6;
const SCAN_SCORE_MAX: i32 = 12;
/// Pages a read may skip and still continue a forward walk.
const SCAN_GAP_TOLERANCE: u32 = 8;
const PAGE_BYTES: usize = 4096;

/// The v2 VFS on main (`depot-client/src/vfs.rs`, the default adaptive read-ahead mode).
///
/// Every read, hit or miss, trains two predictors, one per page class. Each repeats the last
/// step while it held twice in a row, and otherwise follows the step that most often came
/// after the last one. A forward-scan score per class rises by two for each read up to eight
/// pages past the walk's tip and falls on other reads. On a miss the predictor of the missed
/// page's class extrapolates from it, 64 pages deep within 256 KiB, or 128 to 256 pages deep
/// within 1 MiB while the score shows a forward scan. Main's engine also sent the overflow
/// chains of every B-tree page it served; the harness server adds those. Main breaks ties between equally
/// frequent steps in hash order; this model takes the largest step, so runs repeat.
#[derive(Debug, Default)]
pub struct MainV2 {
	btree: V2Predictor,
	overflow: V2Predictor,
	btree_scan: V2Score,
	overflow_scan: V2Score,
}

#[derive(Debug, Default)]
struct V2Predictor {
	last_pgno: Option<Pgno>,
	last_delta: Option<i64>,
	stride_run: usize,
	transitions: BTreeMap<i64, BTreeMap<i64, u32>>,
}

impl V2Predictor {
	fn record(&mut self, pgno: Pgno) {
		if let Some(last_pgno) = self.last_pgno {
			let delta = i64::from(pgno) - i64::from(last_pgno);
			match self.last_delta {
				Some(last_delta) => {
					*self
						.transitions
						.entry(last_delta)
						.or_default()
						.entry(delta)
						.or_default() += 1;
					self.stride_run = match delta == last_delta {
						true => self.stride_run + 1,
						false => 1,
					};
				}
				None => self.stride_run = 1,
			}
			self.last_delta = Some(delta);
		}
		self.last_pgno = Some(pgno);
	}

	fn predict(&self, from: Pgno, depth: usize, size_pages: u32) -> Vec<Pgno> {
		let mut predicted = Vec::new();
		let Some(delta) = self.last_delta else {
			return predicted;
		};
		let inside = |pgno: i64| (1..=i64::from(size_pages)).contains(&pgno);
		if self.stride_run >= 2 && delta > 0 {
			let mut current = i64::from(from);
			while predicted.len() < depth {
				current += delta;
				if !inside(current) {
					break;
				}
				predicted.push(current as Pgno);
			}
			if predicted.len() >= depth {
				return predicted;
			}
		}
		let mut current_delta = delta;
		let mut current = i64::from(from);
		for _ in predicted.len()..depth {
			let Some(next_delta) = self
				.transitions
				.get(&current_delta)
				.and_then(|counts| counts.iter().max_by_key(|(_, count)| **count))
				.map(|(delta, _)| *delta)
			else {
				break;
			};
			current += next_delta;
			if !inside(current) {
				break;
			}
			let pgno = current as Pgno;
			if !predicted.contains(&pgno) {
				predicted.push(pgno);
			}
			current_delta = next_delta;
		}
		predicted
	}
}

/// The v2 forward-scan score of one page class.
#[derive(Debug, Default)]
struct V2Score {
	last_pgno: Option<Pgno>,
	tip: Option<Pgno>,
	score: i32,
}

impl V2Score {
	/// Records a read; returns whether it continues a forward walk.
	fn record(&mut self, pgno: Pgno) -> bool {
		let near = |from: Option<Pgno>| {
			from.and_then(|from| pgno.checked_sub(from))
				.is_some_and(|delta| (1..=SCAN_GAP_TOLERANCE).contains(&delta))
		};
		let forward = near(self.last_pgno) || near(self.tip);
		if forward {
			self.score = (self.score + 2).min(SCAN_SCORE_MAX);
			self.tip = Some(pgno);
		} else if self.last_pgno != Some(pgno) {
			if self.score >= SCAN_SCORE_THRESHOLD && self.tip.is_some() {
				self.score = (self.score - 1).max(0);
			} else {
				self.score = (self.score - 4).max(0);
				self.tip = Some(pgno);
			}
		}
		self.last_pgno = Some(pgno);
		forward
	}
}

impl MainV2 {
	fn record(&mut self, pgno: Pgno, class: Class) -> bool {
		match class {
			Class::Btree => {
				self.btree.record(pgno);
				self.btree_scan.record(pgno)
			}
			Class::Overflow => {
				self.overflow.record(pgno);
				self.overflow_scan.record(pgno)
			}
		}
	}
}

impl ReadAhead for MainV2 {
	fn hit(&mut self, pgno: Pgno, bytes: &[u8], _size_pages: u32) -> Vec<Pgno> {
		self.record(pgno, classify(pgno, bytes));
		Vec::new()
	}

	fn miss(&mut self, pgno: Pgno, size_pages: u32) -> Vec<Pgno> {
		let forward = self.record(pgno, Class::Btree);
		let score = self.btree_scan.score;
		let (depth, max_bytes) = if forward && score >= SCAN_SCORE_THRESHOLD {
			let depth = match score >= SCAN_SCORE_THRESHOLD + 4 {
				true => V2_SCAN_DEPTH,
				false => V2_SCAN_DEPTH.min(V2_DEPTH * 2),
			};
			(depth, V2_SCAN_MAX_BYTES)
		} else {
			(V2_DEPTH, V2_MAX_BYTES)
		};
		let budget = (max_bytes / PAGE_BYTES).saturating_sub(1);
		self.btree
			.predict(pgno, depth.min(budget), size_pages.max(pgno))
			.into_iter()
			.filter(|predicted| *predicted != pgno)
			.collect()
	}
}

/// The first adaptive window, its largest size, and the byte cap of one read-ahead.
const V3_INITIAL_WINDOW: usize = 16;
const V3_MAX_WINDOW: usize = 256;
const V3_MAX_BYTES: usize = 1024 * 1024;
/// Pages a read may move the walk's front by and still count toward opening a window.
const NEAR_SEQUENTIAL_SPAN: u32 = 4;
/// Equal steps in a row that make a walk strided, and the largest strided step.
const STRIDE_RUN: u8 = 6;
const MAX_STRIDE_PAGES: u32 = 64;

/// The adaptive v3 client read-ahead the library replaced, as it stood in `sqlite-embedded`
/// before the client VFS became a pass-through: kernel-like contiguous windows per page class
/// and one more over the whole read stream, which also follows strided walks.
///
/// A window opens after two adjacent reads, or three reads that each move the walk's front
/// forward by at most four pages. It starts at 16 pages and doubles on each miss that
/// reaches its far page, up to 256 pages and 1 MiB. Reads within eight pages still continue
/// the walk; two misses in a row that continue none close it. A walk two reads down opens a
/// backward window. Six reads in a row with the same step of 2 to 64 pages open a strided
/// window on the stream tracker. Hits move the walk but never the window. On a miss, the
/// missed page's class tracker plans the read-ahead when it sees a walk, and the stream
/// tracker when it does not.
#[derive(Debug)]
pub struct V3Adaptive {
	btree: V3Tracker,
	overflow: V3Tracker,
	stream: V3Tracker,
}

impl Default for V3Adaptive {
	fn default() -> Self {
		Self {
			btree: V3Tracker::default(),
			overflow: V3Tracker::default(),
			stream: V3Tracker {
				follows_strides: true,
				..V3Tracker::default()
			},
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
	Forward,
	Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
	None,
	Window,
	Stride,
}

#[derive(Debug, Clone, Copy)]
struct V3Plan {
	mode: Mode,
	backward: bool,
	stride: i64,
	depth: usize,
	seed: Pgno,
}

#[derive(Debug, Default, Clone)]
struct V3Tracker {
	last_pgno: Option<Pgno>,
	tip: Option<Pgno>,
	score: i32,
	window: usize,
	window_edge: Option<Pgno>,
	random_misses: u8,
	adjacent_run: u8,
	advance_run: u8,
	walk_front: Option<Pgno>,
	backward_steps: u8,
	stride: i64,
	stride_run: u8,
	stride_window: usize,
	stride_edge: Option<Pgno>,
	follows_strides: bool,
}

impl V3Tracker {
	fn none(seed: Option<Pgno>) -> V3Plan {
		V3Plan {
			mode: Mode::None,
			backward: false,
			stride: 0,
			depth: 0,
			seed: seed.unwrap_or_default(),
		}
	}

	/// Records reads and plans a miss's read-ahead; a hit passes `missed = false`.
	fn record_and_plan(&mut self, pgnos: &[Pgno], missed: bool) -> V3Plan {
		let mut scan_seed = None;
		for &pgno in pgnos {
			let adjacent = self.last_pgno.is_some_and(|last| last.abs_diff(pgno) == 1);
			self.adjacent_run = match adjacent {
				true => self.adjacent_run.saturating_add(1),
				false => 0,
			};
			let advanced = self.walk_front.is_some_and(|front| {
				pgno.checked_sub(front)
					.is_some_and(|delta| (1..=NEAR_SEQUENTIAL_SPAN).contains(&delta))
			});
			let at_or_behind_front = self.walk_front.is_some_and(|front| {
				front
					.checked_sub(pgno)
					.is_some_and(|delta| delta <= NEAR_SEQUENTIAL_SPAN)
			});
			if advanced {
				self.advance_run = self.advance_run.saturating_add(1);
				self.walk_front = Some(pgno);
			} else if !at_or_behind_front {
				self.advance_run = 0;
				self.walk_front = Some(pgno);
			}
			self.record_stride(pgno);
			if let Some(direction) = self.record(pgno) {
				let opens = self.adjacent_run >= 2
					|| (self.advance_run >= 3 && direction == Direction::Forward);
				scan_seed = Some((pgno, direction, opens));
			}
		}
		if !missed {
			return Self::none(pgnos.last().copied());
		}
		if let Some(plan) = self.plan_stride(pgnos) {
			return plan;
		}
		let Some((seed, direction, opens)) = scan_seed else {
			self.random_misses = self.random_misses.saturating_add(1);
			if self.random_misses >= 2 {
				self.window = 0;
				self.window_edge = None;
			}
			return Self::none(pgnos.last().copied());
		};
		self.random_misses = 0;
		if self.window == 0 && !opens {
			return Self::none(Some(seed));
		}
		let reached_edge = self.window_edge.is_some_and(|edge| match direction {
			Direction::Forward => seed.saturating_add(SCAN_GAP_TOLERANCE) >= edge,
			Direction::Backward => seed <= edge.saturating_add(SCAN_GAP_TOLERANCE),
		});
		self.window = if self.window == 0 {
			V3_INITIAL_WINDOW
		} else if reached_edge {
			self.window.saturating_mul(2)
		} else {
			self.window
		}
		.min(V3_MAX_WINDOW)
		.max(1);
		let span = self.window as u32;
		self.window_edge = Some(match direction {
			Direction::Forward => seed.saturating_add(span),
			Direction::Backward => seed.saturating_sub(span).max(1),
		});
		V3Plan {
			mode: Mode::Window,
			backward: direction == Direction::Backward,
			stride: 0,
			depth: self.window,
			seed,
		}
	}

	fn record_stride(&mut self, pgno: Pgno) {
		let Some(last_pgno) = self.last_pgno else {
			return;
		};
		let step = i64::from(pgno) - i64::from(last_pgno);
		if step == self.stride {
			self.stride_run = self.stride_run.saturating_add(1);
		} else {
			self.stride = step;
			self.stride_run = 1;
			self.stride_window = 0;
			self.stride_edge = None;
		}
	}

	fn plan_stride(&mut self, pgnos: &[Pgno]) -> Option<V3Plan> {
		let seed = *pgnos.last()?;
		let step = self.stride.unsigned_abs();
		if !self.follows_strides
			|| self.stride_run < STRIDE_RUN
			|| !(2..=u64::from(MAX_STRIDE_PAGES)).contains(&step)
		{
			return None;
		}
		self.random_misses = 0;
		let reached_edge = self.stride_edge.is_some_and(|edge| match self.stride > 0 {
			true => seed >= edge,
			false => seed <= edge,
		});
		self.stride_window = if self.stride_window == 0 {
			V3_INITIAL_WINDOW
		} else if reached_edge {
			self.stride_window.saturating_mul(2)
		} else {
			self.stride_window
		}
		.min(V3_MAX_WINDOW)
		.max(1);
		let far = i64::from(seed) + self.stride * self.stride_window as i64;
		self.stride_edge = Some(far.clamp(1, i64::from(u32::MAX)) as Pgno);
		Some(V3Plan {
			mode: Mode::Stride,
			backward: self.stride < 0,
			stride: self.stride,
			depth: self.stride_window,
			seed,
		})
	}

	/// Records a read and returns its direction when it continues a walk.
	fn record(&mut self, pgno: Pgno) -> Option<Direction> {
		let near = |from: Option<Pgno>| {
			from.and_then(|from| pgno.checked_sub(from))
				.is_some_and(|delta| (1..=SCAN_GAP_TOLERANCE).contains(&delta))
		};
		let forward = near(self.last_pgno) || near(self.tip);
		if forward {
			self.score = (self.score + 2).min(SCAN_SCORE_MAX);
			self.tip = Some(pgno);
		} else if self.last_pgno != Some(pgno) {
			if self.score >= SCAN_SCORE_THRESHOLD && self.tip.is_some() {
				self.score = (self.score - 1).max(0);
			} else {
				self.score = (self.score - 4).max(0);
				self.tip = Some(pgno);
			}
		}
		let stepped_back = self
			.last_pgno
			.and_then(|last| last.checked_sub(pgno))
			.is_some_and(|delta| (1..=SCAN_GAP_TOLERANCE).contains(&delta));
		self.backward_steps = match stepped_back {
			true => self.backward_steps.saturating_add(1),
			false => 0,
		};
		self.last_pgno = Some(pgno);
		if forward {
			Some(Direction::Forward)
		} else if self.backward_steps >= 2 {
			Some(Direction::Backward)
		} else {
			None
		}
	}
}

impl V3Adaptive {
	/// Feeds one read to the trackers and returns the plan and the tracker that made it.
	fn record(&mut self, pgno: Pgno, class: Class, missed: bool) -> (V3Plan, Option<Class>) {
		let (btree_pages, overflow_pages): (&[Pgno], &[Pgno]) = match class {
			Class::Btree => (&[pgno], &[]),
			Class::Overflow => (&[], &[pgno]),
		};
		let btree = self.btree.record_and_plan(btree_pages, missed);
		let overflow = self.overflow.record_and_plan(overflow_pages, missed);
		let stream = self.stream.record_and_plan(&[pgno], missed);
		let class_plan = match class {
			Class::Btree => btree,
			Class::Overflow => overflow,
		};
		match class_plan.mode == Mode::None && stream.mode != Mode::None {
			true => (stream, None),
			false => (class_plan, Some(class)),
		}
	}
}

impl ReadAhead for V3Adaptive {
	fn hit(&mut self, pgno: Pgno, bytes: &[u8], _size_pages: u32) -> Vec<Pgno> {
		self.record(pgno, classify(pgno, bytes), false);
		Vec::new()
	}

	fn miss(&mut self, pgno: Pgno, size_pages: u32) -> Vec<Pgno> {
		let (plan, tracker) = self.record(pgno, Class::Btree, true);
		let depth = (V3_MAX_BYTES / PAGE_BYTES)
			.saturating_sub(1)
			.min(plan.depth);
		let seed = plan.seed;
		let last = size_pages.max(seed);
		let pages: Vec<Pgno> = match plan.mode {
			Mode::None => Vec::new(),
			Mode::Stride => (1..=depth as i64)
				.map_while(|offset| {
					u32::try_from(i64::from(seed) + offset * plan.stride)
						.ok()
						.filter(|pgno| (1..=last).contains(pgno))
				})
				.collect(),
			Mode::Window if plan.backward => (1..=depth as u32)
				.map_while(|offset| seed.checked_sub(offset).filter(|pgno| *pgno >= 1))
				.collect(),
			Mode::Window => (1..=depth as u32)
				.map_while(|offset| seed.checked_add(offset).filter(|pgno| *pgno <= last))
				.collect(),
		};
		let far = match plan.backward {
			true => pages.iter().copied().min(),
			false => pages.iter().copied().max(),
		};
		if let Some(far) = far {
			let tracker = match tracker {
				Some(Class::Btree) => &mut self.btree,
				Some(Class::Overflow) => &mut self.overflow,
				None => &mut self.stream,
			};
			match plan.mode {
				Mode::Window => tracker.window_edge = Some(far),
				Mode::Stride => tracker.stride_edge = Some(far),
				Mode::None => {}
			}
		}
		pages
	}
}
