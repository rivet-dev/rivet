//! One open database's prediction state, driven by events from the page protocol server.
//!
//! The server calls [`Prefetcher::plan`] for each page request, reads the pages the plan
//! names, hands their bytes to [`Prefetcher::fetched`], and replies with the pages it keeps. It
//! plans each commit's KV ops with [`Prefetcher::plan_commit`] and reports the outcome. Nothing
//! here reads, writes, sleeps or spawns: every call returns what the caller does next.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::overflow::commit::{Apply, CommitBuilder, KvOp, Sidecar};
use crate::overflow::{Lookup, LookupResult, OverflowMap, RowStatus};
use crate::page::{self, Pgno};
use crate::payloads::LeafPayloads;
use crate::recency::{self, Recency, RecencyCarried};
use crate::scan::ScanDetector;
use crate::window::Window;

/// Where a planned page came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
	Requested,
	Chain,
	Scan,
	Early,
	Frequent,
	Window,
	/// The overflow pages a served leaf's cells name, sent with the leaf for a table whose
	/// payloads SQLite reads.
	Leaf,
}

impl Source {
	pub const ALL: [Source; 7] = [
		Source::Requested,
		Source::Chain,
		Source::Scan,
		Source::Early,
		Source::Frequent,
		Source::Window,
		Source::Leaf,
	];

	pub fn as_str(self) -> &'static str {
		match self {
			Self::Requested => "requested",
			Self::Chain => "chain",
			Self::Scan => "scan",
			Self::Early => "early",
			Self::Frequent => "frequent",
			Self::Window => "window",
			Self::Leaf => "leaf",
		}
	}
}

/// Fixed limits. The defaults are the decided values; tests may lower them.
#[derive(Debug, Clone, Copy)]
pub struct Config {
	/// Pages one reply carries at most, requested pages aside: 4 MiB.
	pub reply_pages: usize,
	/// Prefetched bytes in flight per actor.
	pub actor_prefetch_bytes: usize,
	/// Early pages fetched with the opening read: 2 MiB.
	pub opening_pages: usize,
	/// Upper bound on remembered client pages. Repeated demand invalidates this knowledge
	/// sooner because native whole-page reads consume their VFS buffer entry.
	pub sent_ttl_ms: u64,
	/// Planner parts a harness run switches off to measure what each is worth.
	#[cfg(feature = "harness")]
	pub ablation: Ablation,
}

/// Planner parts a harness run may switch off, one at a time. Production runs every part.
#[cfg(feature = "harness")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ablation {
	pub window: bool,
	/// Scan prefetch only; the shapes it learns still guide the leaf payload walk.
	pub scan: bool,
	/// The early pages of an opening and the frequent pages.
	pub recency: bool,
	/// Chain pages from the overflow map and every lookup of its rows.
	pub overflow_map: bool,
	pub leaf_payloads: bool,
}

/// Which planner parts run.
#[derive(Debug, Clone, Copy)]
struct Parts {
	window: bool,
	scan: bool,
	recency: bool,
	overflow_map: bool,
	leaf_payloads: bool,
}

impl Config {
	#[cfg(feature = "harness")]
	fn parts(&self) -> Parts {
		Parts {
			window: !self.ablation.window,
			scan: !self.ablation.scan,
			recency: !self.ablation.recency,
			overflow_map: !self.ablation.overflow_map,
			leaf_payloads: !self.ablation.leaf_payloads,
		}
	}

	#[cfg(not(feature = "harness"))]
	fn parts(&self) -> Parts {
		Parts {
			window: true,
			scan: true,
			recency: true,
			overflow_map: true,
			leaf_payloads: true,
		}
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			reply_pages: 1_024,
			actor_prefetch_bytes: 4 << 20,
			opening_pages: recency::EARLY_PAGES,
			sent_ttl_ms: 30_000,
			#[cfg(feature = "harness")]
			ablation: Ablation::default(),
		}
	}
}

/// Pages remembered as sent, at most this many.
const SENT_PAGES: usize = 32_768;
/// The page size the page protocol uses.
const PAGE_BYTES: usize = 4096;

/// The pages a request reads, requested pages first.
#[derive(Debug, Clone, Default)]
pub struct Plan {
	pub pages: Vec<(Pgno, Source)>,
	checks: Vec<ChainCheck>,
}

impl Plan {
	pub fn pgnos(&self) -> Vec<Pgno> {
		self.pages.iter().map(|(pgno, _)| *pgno).collect()
	}

	pub fn count(&self, source: Source) -> usize {
		self.pages.iter().filter(|(_, s)| *s == source).count()
	}
}

/// Chain pages a plan names after a missed chain page, verified by their next pointers once
/// read.
#[derive(Debug, Clone)]
struct ChainCheck {
	head: Pgno,
	/// The missed page and the planned pages after it, in chain order.
	pages: Vec<Pgno>,
	/// Whether the last page is the chain's last.
	ends_chain: bool,
}

/// What reading a plan's pages decided.
#[derive(Debug, Default)]
pub struct Fetched {
	/// Per planned page, whether the reply carries it.
	pub keep: Vec<bool>,
	/// Lookups to read before replying, after which [`Prefetcher::followup`] plans more.
	pub followups: Vec<Lookup>,
	/// The requested pages the followups are for.
	followup_pages: Vec<Pgno>,
}

/// The persisted rows an opening starts from.
#[derive(Debug, Default, Clone)]
pub struct OpenRows {
	pub early: Option<Vec<u8>>,
	pub frequent: Option<Vec<u8>>,
	/// The overflow inode holds no rows: this opening created it.
	pub overflow_empty: bool,
}

/// A commit's KV ops and what applying it changes.
#[derive(Debug)]
pub struct CommitPlan {
	pub ops: Vec<KvOp>,
	overflow: Apply,
	recency: RecencyCarried,
	dirty: Vec<Pgno>,
	scan_shapes: Option<Vec<(Pgno, Vec<Pgno>)>>,
	new_size: u32,
	pub overflow_puts: usize,
	pub overflow_deletes: usize,
}

#[derive(Default)]
struct SentPages {
	at: BTreeMap<Pgno, (u64, u64)>,
	order: VecDeque<(u64, u64, Pgno)>,
	generation: u64,
}

impl SentPages {
	fn has(&self, pgno: Pgno, now_ms: u64, ttl_ms: u64) -> bool {
		self.at.get(&pgno).is_some_and(|(at, generation)| {
			*generation == self.generation && now_ms.saturating_sub(*at) < ttl_ms
		})
	}

	fn insert(&mut self, pgno: Pgno, now_ms: u64) {
		self.at.insert(pgno, (now_ms, self.generation));
		self.order.push_back((now_ms, self.generation, pgno));
		while self.order.len() > SENT_PAGES {
			let Some((at, generation, pgno)) = self.order.pop_front() else {
				break;
			};
			if self.at.get(&pgno) == Some(&(at, generation)) {
				self.at.remove(&pgno);
			}
		}
	}

	fn invalidate(&mut self) {
		// Do not free up to 32K map nodes on a demand read. Stale generations are
		// invisible immediately and their bounded entries leave through normal eviction.
		if let Some(next) = self.generation.checked_add(1) {
			self.generation = next;
		} else {
			*self = Self::default();
		}
	}

	fn remove(&mut self, pgno: Pgno) {
		self.at.remove(&pgno);
	}
}

pub struct Prefetcher {
	config: Config,
	size_pages: u32,
	map: OverflowMap,
	window: Window,
	scan: ScanDetector,
	recency: Recency,
	sent: SentPages,
	in_flight_bytes: usize,
	now_ms: u64,
	leaves: LeafPayloads,
}

/// Collects a plan's pages within the prefetch budget.
struct PlanBuilder {
	plan: Plan,
	seen: BTreeSet<Pgno>,
	slots: usize,
	size_pages: u32,
	/// The prefetcher's sent pages, taken for the plan's duration and put back by
	/// [`Prefetcher::finish`].
	sent: SentPages,
	now_ms: u64,
	ttl_ms: u64,
}

impl PlanBuilder {
	fn requested(&mut self, pgno: Pgno) {
		if self.seen.insert(pgno) {
			self.plan.pages.push((pgno, Source::Requested));
		}
	}

	/// Adds a prefetched page. Returns false once the budget is spent.
	fn prefetch(&mut self, pgno: Pgno, source: Source) -> bool {
		if self.slots == 0 {
			return false;
		}
		if pgno == 0 || pgno > self.size_pages || self.seen.contains(&pgno) {
			return true;
		}
		if self.sent.has(pgno, self.now_ms, self.ttl_ms) {
			return true;
		}
		self.seen.insert(pgno);
		self.plan.pages.push((pgno, source));
		self.slots -= 1;
		true
	}
}

impl Prefetcher {
	pub fn open(rows: OpenRows, size_pages: u32, now_ms: u64, config: Config) -> Self {
		let mut recency = Recency::open(rows.early.as_deref(), rows.frequent.as_deref(), now_ms);
		recency.truncate(size_pages);
		Self {
			config,
			size_pages,
			map: OverflowMap::new(rows.overflow_empty),
			window: Window::default(),
			scan: ScanDetector::default(),
			recency,
			sent: SentPages::default(),
			in_flight_bytes: 0,
			now_ms,
			leaves: LeafPayloads::default(),
		}
	}

	pub fn size_pages(&self) -> u32 {
		self.size_pages
	}

	pub fn map(&self) -> &OverflowMap {
		&self.map
	}

	/// Memory this database's prediction state holds, for the process cache budget.
	pub fn memory_bytes(&self) -> usize {
		self.map.memory_bytes()
			+ self.leaves.memory_bytes()
			+ self.scan.memory_bytes()
			+ self.recency.memory_bytes()
			+ self.sent.at.len() * 64
			+ self.sent.order.capacity() * std::mem::size_of::<(u64, u64, Pgno)>()
	}

	/// Reports the file's size at the current head before each request. A size this opening
	/// did not commit means another writer changed the file, so everything learned is dropped.
	pub fn head(&mut self, size_pages: u32) {
		if size_pages != self.size_pages {
			self.forget();
			self.size_pages = size_pages;
		}
	}

	fn forget(&mut self) {
		self.map.reset();
		self.scan.reset();
		self.sent = SentPages::default();
		self.leaves.reset();
	}

	fn builder(&mut self, requested: usize) -> PlanBuilder {
		let in_flight_slots = self
			.config
			.actor_prefetch_bytes
			.saturating_sub(self.in_flight_bytes)
			/ PAGE_BYTES;
		PlanBuilder {
			plan: Plan::default(),
			seen: BTreeSet::new(),
			slots: self
				.config
				.reply_pages
				.saturating_sub(requested)
				.min(in_flight_slots),
			size_pages: self.size_pages,
			sent: std::mem::take(&mut self.sent),
			now_ms: self.now_ms,
			ttl_ms: self.config.sent_ttl_ms,
		}
	}

	/// Plans an opening's first read: the requested pages and the last opening's early pages.
	pub fn opening_plan(&mut self, requested: &[Pgno], now_ms: u64) -> Plan {
		self.now_ms = now_ms;
		// A new opening starts with an empty client buffer: nothing sent or written before it
		// is still there, so none of it may be skipped as already held.
		self.sent = SentPages::default();
		self.leaves.clear();
		for &pgno in requested {
			self.sent.remove(pgno);
			self.recency.miss(pgno);
		}
		let early: Vec<Pgno> = self.recency.opening_pages().to_vec();
		let mut builder = self.builder(requested.len());
		builder.slots = builder.slots.min(self.config.opening_pages);
		for &pgno in requested {
			builder.requested(pgno);
		}
		let early = match self.config.parts().recency {
			true => early,
			false => Vec::new(),
		};
		for pgno in early {
			if !builder.prefetch(pgno, Source::Early) {
				break;
			}
		}
		self.finish(builder)
	}

	/// Plans a read of the pages SQLite missed.
	pub fn plan(&mut self, requested: &[Pgno], now_ms: u64) -> Plan {
		self.now_ms = now_ms;
		self.recency.tick(now_ms);
		// A page we just delivered is missing again: the client consumed or evicted it.
		// We cannot infer which siblings it retained, so discard the cache hint together.
		// This changes only suppression; ordinary predictors and reply budgets still apply.
		if requested
			.iter()
			.any(|pgno| self.sent.has(*pgno, now_ms, self.config.sent_ttl_ms))
		{
			self.sent.invalidate();
		}
		for &pgno in requested {
			self.sent.remove(pgno);
			self.recency.miss(pgno);
		}
		self.leaves.clear();
		let parts = self.config.parts();
		// A miss on a chain head a served leaf named: the query reads this table's payloads.
		for &pgno in requested {
			// A miss inside a known chain counts as a miss on its head, so a walk whose reply
			// the cap cut short goes on from the chain it was in.
			let head = match self.map.status(pgno, self.size_pages) {
				RowStatus::Chain(head) => head,
				RowStatus::None | RowStatus::Unknown => pgno,
			};
			if !parts.leaf_payloads {
				continue;
			}
			if let Some(leaf) = self.leaves.owner(head) {
				let table = self.table_of(leaf);
				self.leaves.miss(head, table, &self.scan);
			}
		}

		let mut chains: Vec<(Pgno, Pgno, u32)> = Vec::new();
		for &pgno in requested.iter().filter(|_| parts.overflow_map) {
			if let RowStatus::Chain(head) = self.map.status(pgno, self.size_pages) {
				if let Some(position) = self.map.chain(head).and_then(|c| c.position(pgno)) {
					chains.push((pgno, head, position));
				}
			}
		}
		for (_, head, _) in &chains {
			self.map.touch(*head);
		}

		let mut frequent = self.recency.take_frequent();
		if !parts.recency {
			frequent.clear();
		}
		let mut builder = self.builder(requested.len());
		for &pgno in requested {
			builder.requested(pgno);
		}

		// Chain pages from the map, from the missed page onward.
		let mut explained: BTreeSet<Pgno> = BTreeSet::new();
		for (pgno, head, position) in chains {
			let Some(chain) = self.map.chain(head) else {
				continue;
			};
			let mut pages = vec![pgno];
			let mut cut = false;
			for next in chain.pages().skip(position as usize + 1) {
				if !builder.prefetch(next, Source::Chain) {
					cut = true;
					break;
				}
				pages.push(next);
			}
			if pages.len() > 1 {
				explained.insert(pgno);
			}
			let ends_chain = !cut;
			builder.plan.checks.push(ChainCheck {
				head,
				pages,
				ends_chain,
			});
		}

		// Children of interior pages a scan is walking.
		for &pgno in requested.iter().filter(|_| parts.scan) {
			let pages = self.scan.miss(pgno, builder.slots);
			if !pages.is_empty() {
				explained.insert(pgno);
			}
			for page in pages {
				if !builder.prefetch(page, Source::Scan) {
					break;
				}
			}
		}

		for pgno in frequent {
			if !builder.prefetch(pgno, Source::Frequent) {
				break;
			}
		}

		// The contiguous window, for misses nothing else explained.
		for &pgno in requested.iter().filter(|_| parts.window) {
			if explained.contains(&pgno) {
				continue;
			}
			let Some(span) = self.window.miss(pgno, self.size_pages) else {
				continue;
			};
			let mut last_sent = None;
			for page in span.pages() {
				if !builder.prefetch(page, Source::Window) {
					break;
				}
				last_sent = Some(page);
			}
			match last_sent {
				Some(last) if span.pages().last() != Some(last) => self.window.sent_through(last),
				Some(_) => {}
				None => self.window.sent_through(pgno),
			}
		}

		self.finish(builder)
	}

	fn finish(&mut self, builder: PlanBuilder) -> Plan {
		self.sent = builder.sent;
		let plan = builder.plan;
		self.charge(&plan);
		plan
	}

	fn charge(&mut self, plan: &Plan) {
		let prefetched = plan
			.pages
			.iter()
			.filter(|(_, source)| *source != Source::Requested)
			.count();
		self.in_flight_bytes += prefetched * PAGE_BYTES;
	}

	/// Learns from the pages a plan read, in plan order, and decides which the reply keeps:
	/// chain pages are kept only while their next pointers agree with the map.
	pub fn fetched(&mut self, plan: &Plan, pages: &[Option<&[u8]>]) -> Fetched {
		let mut keep = vec![true; plan.pages.len()];
		let bytes_of: BTreeMap<Pgno, &[u8]> = plan
			.pages
			.iter()
			.zip(pages)
			.filter_map(|((pgno, _), bytes)| bytes.map(|bytes| (*pgno, bytes)))
			.collect();

		// Verify chains before observing, so a stale chain is judged on its own rows.
		let mut dropped: BTreeSet<Pgno> = BTreeSet::new();
		for check in &plan.checks {
			let mut broken = false;
			for (index, pgno) in check.pages.iter().enumerate() {
				if broken {
					dropped.insert(*pgno);
					continue;
				}
				// A page the client already holds was left out of the read; its own link
				// cannot be checked, but the links around it still are.
				let Some(bytes) = bytes_of.get(pgno) else {
					continue;
				};
				let next = page::next(bytes).unwrap_or(u32::MAX);
				let expected = match check.pages.get(index + 1) {
					Some(next_page) => Some(*next_page),
					None if check.ends_chain => Some(0),
					None => None,
				};
				if expected.is_some_and(|expected| expected != next) {
					broken = true;
				}
			}
			if broken {
				self.map.poison(check.head);
			}
		}
		for (index, (pgno, source)) in plan.pages.iter().enumerate() {
			if *source == Source::Chain && dropped.contains(pgno) {
				keep[index] = false;
			}
		}

		for ((pgno, source), bytes) in plan.pages.iter().zip(pages) {
			let Some(bytes) = bytes else {
				continue;
			};
			self.map
				.observe(*pgno, bytes, self.size_pages, *source == Source::Requested);
			if self.map.is_free(*pgno) {
				continue;
			}
			if let Some(kind) = page::proves_btree(*pgno, bytes, self.size_pages)
				&& !kind.is_interior()
			{
				// Every leaf is remembered, rows without chains too, so a walk can step over it.
				let refs = page::overflow_refs(*pgno, bytes, self.map.geometry());
				self.leaves.remember(*pgno, &refs);
			}
			if let Some(children) = page::children(*pgno, bytes) {
				self.scan.interior(*pgno, children);
			}
		}

		// A missed page that reads as an overflow page the map cannot place: look its rows up
		// before replying, so the chain's rest rides this reply.
		let mut followups = Vec::new();
		let mut followup_pages = Vec::new();
		for ((pgno, source), bytes) in plan.pages.iter().zip(pages) {
			if *source != Source::Requested || !self.config.parts().overflow_map {
				continue;
			}
			let Some(bytes) = bytes else {
				continue;
			};
			if page::proves_btree(*pgno, bytes, self.size_pages).is_some() {
				continue;
			}
			if self.map.status(*pgno, self.size_pages) != RowStatus::Unknown
				|| self.map.is_trunk(*pgno)
			{
				continue;
			}
			// Only a head a page SQLite read names is known live; any other page gets a
			// floor lookup, which cannot mistake a stale cell for a chain.
			let lookup = match self
				.map
				.known_head(*pgno)
				.filter(|_| self.map.is_trusted_head(*pgno))
			{
				Some(_) => Lookup::Head(*pgno),
				None => Lookup::Floor {
					first: pgno
						.saturating_sub(crate::overflow::chain::MAX_RUN_SPAN - 1)
						.max(1),
					last: pgno.saturating_add(crate::overflow::chain::MAX_RUN_SPAN - 1),
				},
			};
			followups.push(lookup);
			followup_pages.push(*pgno);
		}
		Fetched {
			keep,
			followups,
			followup_pages,
		}
	}

	/// The table a leaf belongs to, as far as the scan detector's shapes reach up from it.
	fn table_of(&self, leaf: Pgno) -> Pgno {
		let mut at = leaf;
		for _ in 0..16 {
			match self.scan.parent_of(at) {
				Some(parent) => at = parent,
				None => break,
			}
		}
		at
	}

	/// Head lookups the leaf payloads of this request need first: chains of more than one page
	/// whose rows this opening has not read. One parallel round of point reads lets the whole
	/// chain ride the reply instead of its head alone.
	pub fn leaf_lookups(&self) -> Vec<Lookup> {
		let mut lookups = Vec::new();
		if !self.config.parts().overflow_map {
			return lookups;
		}
		for overflow in self.leaves.pending() {
			let unknown = self.map.status(overflow.head, self.size_pages) == RowStatus::Unknown;
			if overflow.pages > 1 && unknown {
				lookups.push(Lookup::Head(overflow.head));
			}
		}
		lookups
	}

	/// Plans the chains the payload walk asks this reply to carry: the whole chain when the
	/// map knows it, else its head, within the reply's remaining budget. `already` holds the
	/// pages the reply carries.
	pub fn leaf_payloads(&mut self, already: &BTreeSet<Pgno>) -> Plan {
		let chains = self.leaves.pending().to_vec();
		let mut builder = self.builder(already.len());
		for pgno in already {
			builder.seen.insert(*pgno);
		}
		let mut delivered = 0;
		'chains: for overflow in chains {
			let known = self
				.map
				.chain(overflow.head)
				.filter(|_| self.config.parts().overflow_map);
			let pages: Vec<Pgno> = match known {
				Some(chain) if chain.page_count() == overflow.pages => chain.pages().collect(),
				Some(_) | None => vec![overflow.head],
			};
			for (index, pgno) in pages.into_iter().enumerate() {
				if !builder.prefetch(pgno, Source::Leaf) {
					if index > 0 {
						delivered += 1;
					}
					break 'chains;
				}
			}
			delivered += 1;
		}
		self.leaves.delivered(delivered);
		self.leaves.clear();
		self.finish(builder)
	}

	/// Plans the chain pages after missed pages whose rows a followup lookup just read.
	pub fn followup(&mut self, fetched: &Fetched, already: &BTreeSet<Pgno>) -> Plan {
		let mut builder = self.builder(already.len());
		for pgno in already {
			builder.seen.insert(*pgno);
		}
		for &pgno in &fetched.followup_pages {
			let RowStatus::Chain(head) = self.map.status(pgno, self.size_pages) else {
				continue;
			};
			let Some(chain) = self.map.chain(head) else {
				continue;
			};
			let Some(position) = chain.position(pgno) else {
				continue;
			};
			let mut pages = vec![pgno];
			let mut cut = false;
			for next in chain.pages().skip(position as usize + 1) {
				if !builder.prefetch(next, Source::Chain) {
					cut = true;
					break;
				}
				pages.push(next);
			}
			builder.plan.checks.push(ChainCheck {
				head,
				pages,
				ends_chain: !cut,
			});
		}
		self.finish(builder)
	}

	/// Records every page with bytes that a reply carried, including demand pages, and
	/// releases the plan's in-flight bytes. Missing requested pages are not delivery evidence.
	pub fn sent(&mut self, delivered: impl IntoIterator<Item = Pgno>, planned: usize) {
		for pgno in delivered {
			self.sent.insert(pgno, self.now_ms);
		}
		self.in_flight_bytes = self.in_flight_bytes.saturating_sub(planned * PAGE_BYTES);
	}

	/// Forgets after-reply lookups that were cancelled before their rows arrived.
	pub fn lookups_cancelled(&mut self) {
		self.map.lookups_cancelled();
	}

	/// Releases the in-flight prefetch bytes of a plan whose reply never went out.
	pub fn plan_abandoned(&mut self, planned: usize) {
		self.sent([], planned);
	}

	/// Lookups to read once the reply is out: the rows of chains whose heads served pages name.
	pub fn take_lookups(&mut self) -> Vec<Lookup> {
		let lookups = self.map.take_lookups();
		match self.config.parts().overflow_map {
			true => lookups,
			false => {
				self.map.lookups_cancelled();
				Vec::new()
			}
		}
	}

	/// Applies rows a lookup read. Returns lookups needed to complete the chains it found.
	pub fn apply(&mut self, result: LookupResult) -> Vec<Lookup> {
		self.map.apply(result, self.size_pages)
	}

	/// Applies a trunk page a commit had to read.
	pub fn observe_trunk(&mut self, pgno: Pgno, bytes: &[u8]) {
		self.map.observe_trunk(pgno, bytes);
	}

	/// A builder for a commit's dirty pages.
	pub fn commit_builder(&self) -> CommitBuilder {
		CommitBuilder::default()
	}

	/// Adds a dirty page to a commit.
	pub fn add_dirty(&self, builder: &mut CommitBuilder, pgno: Pgno, bytes: &[u8]) {
		builder.add(pgno, bytes, self.map.geometry());
	}

	/// Plans a commit's KV ops within `budget` bytes of entry, or returns the lookups it needs
	/// first. The overflow deletes always fit, by purging when nothing else does; the recent
	/// rows ride only when room is left.
	pub fn plan_commit(
		&mut self,
		builder: &CommitBuilder,
		new_size: u32,
		txid: u64,
		budget: usize,
	) -> Result<CommitPlan, Vec<Lookup>> {
		let overflow = self
			.map
			.plan_commit(builder, self.size_pages, new_size, txid, budget)?;
		Ok(self.with_recency(overflow, builder, new_size, budget))
	}

	/// Plans a commit that purges every overflow row, as a fallback that needs no reads.
	pub fn plan_purge(
		&mut self,
		builder: &CommitBuilder,
		new_size: u32,
		txid: u64,
		budget: usize,
	) -> CommitPlan {
		let overflow = self.map.plan_purge(builder, new_size, txid, budget);
		self.with_recency(overflow, builder, new_size, budget)
	}

	fn with_recency(
		&mut self,
		overflow: crate::overflow::commit::OverflowPlan,
		builder: &CommitBuilder,
		new_size: u32,
		budget: usize,
	) -> CommitPlan {
		let mut ops = overflow.ops;
		let mut spent: usize = ops.iter().map(KvOp::cost).sum();
		let rows = self.recency.rows_for_commit(builder.len());
		let mut carried = RecencyCarried::default();
		if let Some(value) = rows.early {
			let op = KvOp::Put {
				sidecar: Sidecar::Recent,
				key: recency::EARLY_KEY.to_vec(),
				value,
			};
			if spent + op.cost() <= budget {
				spent += op.cost();
				ops.push(op);
				carried.early = true;
			}
		}
		if let Some(value) = rows.frequent {
			let op = KvOp::Put {
				sidecar: Sidecar::Recent,
				key: recency::FREQUENT_KEY.to_vec(),
				value,
			};
			if spent + op.cost() <= budget {
				ops.push(op);
				carried.frequent = true;
			}
		}
		CommitPlan {
			ops,
			overflow: overflow.apply,
			recency: carried,
			dirty: builder.pages().collect(),
			scan_shapes: builder.scan_shapes(new_size),
			new_size,
			overflow_puts: overflow.puts,
			overflow_deletes: overflow.deletes,
		}
	}

	/// Applies a durable commit.
	pub fn committed(&mut self, plan: CommitPlan) {
		for op in &plan.ops {
			let label = match op {
				KvOp::Put {
					sidecar: Sidecar::Overflow,
					..
				} => Some("put"),
				KvOp::Delete {
					sidecar: Sidecar::Overflow,
					..
				} => Some("delete"),
				KvOp::DeleteRange {
					sidecar: Sidecar::Overflow,
					..
				} => Some("purge"),
				KvOp::Put {
					sidecar: Sidecar::Recent,
					..
				}
				| KvOp::Delete {
					sidecar: Sidecar::Recent,
					..
				}
				| KvOp::DeleteRange {
					sidecar: Sidecar::Recent,
					..
				} => None,
			};
			if let Some(label) = label {
				crate::metrics::ROWS_WRITTEN_TOTAL
					.with_label_values(&[label])
					.inc();
			}
		}
		if plan.new_size < self.size_pages {
			self.scan.reset();
		}
		for pgno in plan.overflow.freed_pages() {
			self.scan.forget(pgno);
		}
		self.map.committed(plan.overflow);
		self.recency.committed(&plan.recency);
		self.recency.truncate(plan.new_size);
		self.size_pages = plan.new_size;
		for pgno in plan.dirty {
			self.scan.forget(pgno);
			self.sent.insert(pgno, self.now_ms);
		}
		match plan.scan_shapes {
			Some(shapes) => {
				for (pgno, children) in shapes {
					if !self.map.is_free(pgno) {
						self.scan.committed_interior(pgno, children);
					}
				}
			}
			None => self.scan.reset(),
		}
	}

	/// Drops what a commit whose outcome is unknown may have invalidated.
	pub fn commit_failed(&mut self) {
		self.forget();
	}

	/// Records the opening's end.
	pub fn close(&self) {
		crate::metrics::ROWS_PER_DATABASE.observe(self.map.cached_rows() as f64);
	}
}


