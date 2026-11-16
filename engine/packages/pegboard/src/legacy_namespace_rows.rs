//! Legacy: the pass that moves every legacy namespace row of the catalog into its namespace
//! volume. The legacy behavior kept a namespace created by 3.0.0 as a record row of the catalog,
//! moved into its own volume only when something read it; new behavior also moves every row no
//! request reads, so 3.1 can delete the rows, the legacy listing and the row handlers. Taken once
//! every storage worker reads namespace volumes, by the one storage worker elected below.
//! Removed in 3.1 with the backfill.
//!
//! The pass walks the record rows in pages through the legacy list command and moves each row
//! through the same move the read path uses, so a row the read path moved first is simply gone
//! from the next page. It keeps its cursor in memory: running it again from the start is cheap
//! and idempotent, because a moved row is deleted with its index write. It ends when a full walk
//! finds no row. Each page and each move is bounded by the page deadline. A refused move does not
//! stop the walk: the walk goes on to the rows after it and ends refused after its last page, so
//! one row that keeps refusing never starves the rest. A refused page ends the walk, since the
//! rows after it cannot be reached. A refused walk, or one that finds only rows the walk before
//! it found too, is sent again after a retry wait that doubles, never polled. A row or page found
//! by `LEGACY_REFUSAL_WARN_EVERY` walks in a row warns.
//!
//! The pass starts only after two checks one backfill start delay apart both find the fleet
//! reading namespace volumes, as the actor backfill does, so a storage worker of the previous
//! release that is briefly unobserved does not start it. Between checks it waits for a topology
//! change rather than polling.
//!
//! It runs on the storage worker with the lowest worker id that storage discovery lists. The
//! catalog volume's opening has the wrong lifetime for it: it closes after a minute idle, well
//! inside the start delay. A process service has the right lifetime, and the election keeps the
//! walk on one worker; two workers that briefly both think they are elected move the same rows,
//! and a repeated move changes nothing.

use crate::wire;
use anyhow::{Context, Result};
use rivet_engine_context::EngineContext;
use rivet_env::Clock;
use std::{collections::BTreeMap, future::Future, time::Duration};

/// Legacy: the rows one legacy list command answers per page. The legacy list caps a page at
/// this many. Removed in 3.1 with the backfill.
const LEGACY_PAGE_LIMIT: u32 = 100;

/// Legacy: the consecutive walks that find one row or refuse one page after which the pass
/// warns, and every this many after that. Removed in 3.1 with the backfill.
const LEGACY_REFUSAL_WARN_EVERY: u32 = 10;

/// Legacy: the waits and deadlines of the pass, from the backfill and namespace job
/// configuration. Removed in 3.1 with the backfill.
#[derive(Clone, Debug)]
pub(crate) struct LegacyRowPassPolicy {
	/// The wait between the two fleet checks that start the pass.
	pub start_delay_ms: u64,
	/// The deadline of one page list and of each move.
	pub page_timeout_ms: u64,
	/// The first wait after a refusal, and the wait before the walk that confirms a walk that
	/// moved rows; refusals double it up to `retry_max_ms`.
	pub retry_ms: u64,
	pub retry_max_ms: u64,
}

impl LegacyRowPassPolicy {
	pub(crate) fn from_config(ctx: &EngineContext) -> Self {
		let pegboard = ctx.config().pegboard();
		Self {
			start_delay_ms: pegboard.backfill.start_delay_ms(),
			page_timeout_ms: pegboard.backfill.page_timeout_ms(),
			retry_ms: pegboard.namespace_fork_retry_ms(),
			retry_max_ms: pegboard.namespace_job_retry_max_ms(),
		}
	}
}

/// Legacy: one page of legacy catalog rows and the cursor of the next, `None` at the end.
/// Removed in 3.1 with the backfill.
pub(crate) struct LegacyRowPage {
	pub rows: Vec<(String, wire::NamespaceRecord)>,
	pub cursor: Option<String>,
}

/// Legacy: what the pass reads and sends. [`LegacyCatalogRows`] is the production one, over the
/// topology view and Depot commands; the catalog unit tests drive the pass over the catalog's
/// own handlers. Removed in 3.1 with the backfill.
pub(crate) trait LegacyRowPassOps: Send + Sync {
	/// Whether the pass may walk now: every storage worker reads namespace volumes and this
	/// worker is the one elected to walk.
	fn ready(&self) -> bool;

	/// Resolves once [`LegacyRowPassOps::ready`] answers `ready`, waiting for topology changes
	/// between checks.
	fn wait_until(&self, ready: bool) -> impl Future<Output = Result<()>> + Send;

	/// The rows after `after`, through the legacy list command.
	fn list_rows(
		&self,
		after: Option<String>,
	) -> impl Future<Output = Result<LegacyRowPage>> + Send;

	/// Moves `name` from its row into its namespace volume, as the read path does.
	fn move_row(
		&self,
		name: String,
		record: wire::NamespaceRecord,
	) -> impl Future<Output = Result<()>> + Send;
}

/// Legacy: how one walk over the rows ended. Removed in 3.1 with the backfill.
#[derive(Debug, PartialEq, Eq)]
enum LegacyWalkEnd {
	/// The walk found no row: every namespace lives on its volume.
	Clean,
	/// The walk moved every row it found, and found at least one row the walk before it did not;
	/// a walk after the retry wait confirms none is left, once the moves' index writes deleted
	/// the rows.
	Moved,
	/// A page or a move failed or passed its deadline, or every row the walk found was found by
	/// the walk before it too, so its moves are not deleting it. Sent again after a retry wait
	/// that doubles.
	Refused,
	/// The fleet stopped reading namespace volumes, or another worker was elected.
	NotReady,
}

/// Legacy: what one find counts toward the warning: a row by name, or a page by the cursor it
/// was listed after. Removed in 3.1 with the backfill.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum LegacyFind {
	Row(String),
	Page(Option<String>),
}

/// Legacy: how many walks in a row found each row or refused each page, for the warning.
/// `previous` holds the finds of the last finished walk and `current` those of the walk in
/// progress, so a row moved for good drops out after one walk. Both are bounded by the legacy
/// rows the catalog holds, which only 3.0.0 binaries write. Removed in 3.1 with the backfill.
#[derive(Default)]
struct LegacyFinds {
	previous: BTreeMap<LegacyFind, u32>,
	current: BTreeMap<LegacyFind, u32>,
}

impl LegacyFinds {
	/// Counts a find of `find` by the walk in progress, with the refusal of its move or list if
	/// it was refused, warns once `LEGACY_REFUSAL_WARN_EVERY` walks in a row found it and every as
	/// many again, and answers whether the walk before found it too.
	fn found(&mut self, find: LegacyFind, refusal: Option<&anyhow::Error>) -> bool {
		let count = self
			.previous
			.get(&find)
			.copied()
			.unwrap_or(0)
			.saturating_add(1);
		if count % LEGACY_REFUSAL_WARN_EVERY == 0 {
			match (&find, refusal) {
				(LegacyFind::Row(name), Some(error)) => tracing::warn!(
					namespace = %name,
					walks = count,
					?error,
					"a legacy namespace row keeps refusing its move into its namespace volume; check the namespace volume's host"
				),
				(LegacyFind::Row(name), None) => tracing::warn!(
					namespace = %name,
					walks = count,
					"a legacy namespace row is still found after its moves; check its index write on the catalog"
				),
				(LegacyFind::Page(after), refusal) => tracing::warn!(
					?after,
					walks = count,
					error = ?refusal,
					"a legacy namespace row page keeps refusing; check the catalog volume's host and its rows"
				),
			}
		}
		self.current.insert(find, count);
		count > 1
	}

	/// Ends the walk in progress: its finds become the ones the next walk counts on.
	fn end_walk(&mut self) {
		self.previous = std::mem::take(&mut self.current);
	}
}

/// Legacy: runs the pass until a full walk finds no legacy row, as the module describes, and
/// answers how many rows it moved. Fails only when the clock or the topology view fails.
/// Removed in 3.1 with the backfill.
pub(crate) async fn legacy_run_pass(
	ops: &impl LegacyRowPassOps,
	clock: &dyn Clock,
	policy: &LegacyRowPassPolicy,
) -> Result<u64> {
	let mut moved = 0u64;
	let mut started = false;
	let mut finds = LegacyFinds::default();
	loop {
		// The fleet reads namespace volumes twice, one start delay apart.
		ops.wait_until(true).await?;
		rivet_env::sleep(clock, Duration::from_millis(policy.start_delay_ms)).await?;
		if !ops.ready() {
			continue;
		}
		if !started {
			tracing::info!("legacy namespace row pass started");
			started = true;
		}

		// Walk until a walk finds no row, waiting after every walk that found one.
		let mut retry_ms = policy.retry_ms;
		loop {
			let end = legacy_walk(ops, clock, policy, &mut finds, &mut moved).await;
			finds.end_walk();
			match end? {
				LegacyWalkEnd::Clean => {
					crate::metrics::LEGACY_NAMESPACE_ROW_PASSES_FINISHED.inc();
					tracing::info!(
						moved,
						"legacy namespace row pass finished; no legacy catalog row remains"
					);
					return Ok(moved);
				}
				LegacyWalkEnd::Moved => {
					retry_ms = policy.retry_ms;
					rivet_env::sleep(clock, Duration::from_millis(policy.retry_ms)).await?;
				}
				LegacyWalkEnd::Refused => {
					rivet_env::sleep(clock, Duration::from_millis(retry_ms)).await?;
					retry_ms = retry_ms.saturating_mul(2).min(policy.retry_max_ms);
				}
				LegacyWalkEnd::NotReady => break,
			}
		}
	}
}

/// Legacy: one walk over every row from the start, moving each, as the module describes.
/// Removed in 3.1 with the backfill.
async fn legacy_walk(
	ops: &impl LegacyRowPassOps,
	clock: &dyn Clock,
	policy: &LegacyRowPassPolicy,
	finds: &mut LegacyFinds,
	moved: &mut u64,
) -> Result<LegacyWalkEnd> {
	let deadline = Duration::from_millis(policy.page_timeout_ms);
	let mut cursor: Option<String> = None;
	let mut found = false;
	let mut found_new = false;
	let mut refused = false;
	loop {
		if !ops.ready() {
			return Ok(LegacyWalkEnd::NotReady);
		}
		// A refused page ends the walk: the rows after it cannot be reached without it.
		let listed = match rivet_env::timeout(clock, deadline, ops.list_rows(cursor.clone())).await
		{
			Ok(Ok(page)) => Ok(page),
			Ok(Err(error)) => Err(error),
			Err(rivet_env::TimeoutError::Elapsed) => {
				Err(anyhow::anyhow!("the page passed the page deadline"))
			}
			Err(rivet_env::TimeoutError::Clock(error)) => {
				return Err(error).context("the legacy namespace row pass lost its clock");
			}
		};
		let page = match listed {
			Ok(page) => page,
			Err(error) => {
				crate::metrics::LEGACY_NAMESPACE_ROW_PASS_REFUSALS.inc();
				tracing::debug!(?error, after = ?cursor, "a legacy namespace row page was refused; sending it again after the retry wait");
				finds.found(LegacyFind::Page(cursor), Some(&error));
				return Ok(LegacyWalkEnd::Refused);
			}
		};

		for (name, record) in page.rows {
			found = true;
			if !ops.ready() {
				return Ok(LegacyWalkEnd::NotReady);
			}
			let error =
				match rivet_env::timeout(clock, deadline, ops.move_row(name.clone(), record)).await
				{
					Ok(Ok(())) => {
						crate::metrics::LEGACY_NAMESPACE_ROW_PASS_MOVES.inc();
						*moved = moved.saturating_add(1);
						if !finds.found(LegacyFind::Row(name), None) {
							found_new = true;
						}
						continue;
					}
					Ok(Err(error)) => error,
					Err(rivet_env::TimeoutError::Elapsed) => {
						anyhow::anyhow!("the move passed the page deadline")
					}
					Err(rivet_env::TimeoutError::Clock(error)) => {
						return Err(error).context("the legacy namespace row pass lost its clock");
					}
				};
			// A refused move skips its row for the rest of this walk; the next walk retries it.
			crate::metrics::LEGACY_NAMESPACE_ROW_PASS_REFUSALS.inc();
			tracing::debug!(namespace = %name, ?error, "a legacy namespace row move was refused; the walk goes on and the next walk sends it again");
			finds.found(LegacyFind::Row(name), Some(&error));
			refused = true;
		}

		if let Some(next) = page.cursor {
			cursor = Some(next);
			continue;
		}
		if refused || (found && !found_new) {
			return Ok(LegacyWalkEnd::Refused);
		}
		if found {
			return Ok(LegacyWalkEnd::Moved);
		}
		return Ok(LegacyWalkEnd::Clean);
	}
}

/// Legacy: the production [`LegacyRowPassOps`]: the fleet gate and election from this process's
/// topology view, and the catalog's legacy list and the read path's move through Depot. Removed
/// in 3.1 with the backfill.
pub(crate) struct LegacyCatalogRows {
	pub ctx: EngineContext,
}

impl LegacyCatalogRows {
	/// Whether this worker is the storage worker with the lowest id storage discovery lists and
	/// its own lease admits work now. A draining worker is not listed, so it hands the pass to
	/// the next; a worker whose lease lapsed stops walking even while its last snapshot still
	/// lists it lowest.
	fn elected(&self) -> bool {
		let topology = self.ctx.topology();
		let Ok(snapshot) = topology.snapshot() else {
			return false;
		};
		let Ok(now_boot_ms) = self.ctx.env().clock().boot_ms() else {
			return false;
		};
		legacy_elected(
			&self.ctx.worker_identity().worker_id,
			snapshot.may_admit(now_boot_ms),
			snapshot.storage_workers.iter().map(|worker| &worker.id),
		)
	}
}

/// Legacy: whether `own` walks: its lease admits work and it is the lowest of `storage_workers`.
/// Removed in 3.1 with the backfill.
fn legacy_elected<'a>(
	own: &rivet_topology::worker::WorkerId,
	admitted: bool,
	storage_workers: impl Iterator<Item = &'a rivet_topology::worker::WorkerId>,
) -> bool {
	admitted && storage_workers.min() == Some(own)
}

impl LegacyRowPassOps for LegacyCatalogRows {
	fn ready(&self) -> bool {
		matches!(crate::catalog::namespace_volumes_open(&self.ctx), Ok(true)) && self.elected()
	}

	async fn wait_until(&self, ready: bool) -> Result<()> {
		// Subscribe before the check, so a change between the two is not missed.
		let mut snapshots = self.ctx.topology().subscribe()?;
		loop {
			if self.ready() == ready {
				return Ok(());
			}
			snapshots
				.changed()
				.await
				.context("the topology view closed while the legacy namespace row pass waited")?;
		}
	}

	async fn list_rows(&self, after: Option<String>) -> Result<LegacyRowPage> {
		crate::catalog::legacy_list_rows(&self.ctx, after, LEGACY_PAGE_LIMIT).await
	}

	async fn move_row(&self, name: String, record: wire::NamespaceRecord) -> Result<()> {
		crate::catalog::legacy_move_row(&self.ctx, &name, &record).await
	}
}

/// Legacy: runs the pass for this process's lifetime: once a pass finished, it runs again when
/// the fleet or the election changes and comes back, since a storage worker of the previous
/// release that joins again can write new rows. Removed in 3.1 with the backfill.
pub(crate) async fn legacy_run(ctx: EngineContext) -> Result<()> {
	let policy = LegacyRowPassPolicy::from_config(&ctx);
	let clock = ctx.env().clock();
	let ops = LegacyCatalogRows { ctx };
	loop {
		legacy_run_pass(&ops, clock.as_ref(), &policy).await?;
		ops.wait_until(false).await?;
	}
}
