//! The one sequence of reads around a [`Prefetcher`], shared by the page protocol server and
//! the testing VFS so both drive the library the same way. The [`Store`] does the reads; this
//! module only orders them.
use std::collections::BTreeSet;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;

use crate::overflow::commit::MAX_FLOOR_PAGES;
use crate::overflow::commit::{CommitBuilder, Sidecar};
use crate::overflow::{Lookup, LookupResult};
use crate::page::Pgno;
use crate::prefetcher::{CommitPlan, Prefetcher, Source};

/// Lookup rounds a commit may need: a floor lookup, the head rows it points at, and the
/// free-list trunks it follows. Past this the commit purges the rows instead.
const COMMIT_LOOKUP_ROUNDS: usize = 256;

/// Reads at the database's current head.
#[async_trait]
pub trait Store: Send + Sync {
	/// Reads pages; a page past the end of the file reads as `None`.
	async fn read_pages(&self, pgnos: &[Pgno]) -> Result<Vec<Option<Bytes>>>;

	async fn kv_get(&self, sidecar: Sidecar, key: &[u8]) -> Result<Option<Vec<u8>>>;

	/// Rows with keys in `start..=end`, in key order, at most `limit`.
	async fn kv_scan(
		&self,
		sidecar: Sidecar,
		start: &[u8],
		end: &[u8],
		limit: usize,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

	/// Whether `error` refused a read before it ran, so the commit that needed it should be
	/// refused the same way and sent again, rather than purge the overflow rows.
	fn refused(&self, _error: &anyhow::Error) -> bool {
		false
	}
}

/// A reply to a page request.
#[derive(Debug, Default)]
pub struct Reply {
	/// The requested pages in request order, then the prefetched pages.
	pub pages: Vec<(Pgno, Option<Bytes>)>,
	/// The source of each prefetched page, in reply order after the requested ones.
	pub sources: Vec<Source>,
	/// Lookups to read once the reply is out, overlapping the client's next round trip.
	pub after_reply: Vec<Lookup>,
}

/// Serves one page request. `opening` marks a VFS's first read of an opening, which carries
/// the early pages. The requested pages always come back; a failed lookup only costs
/// prediction.
pub async fn read<S: Store>(
	prefetcher: &mut Prefetcher,
	store: &S,
	request_id: u64,
	requested: &[Pgno],
	now_ms: u64,
	opening: bool,
) -> Result<Reply> {
	let plan = match opening {
		true => prefetcher.opening_plan(requested, now_ms),
		false => prefetcher.plan(requested, now_ms),
	};
	let pgnos = plan.pgnos();
	let planned_count = plan.pages.len() - plan.count(Source::Requested);
	let bytes = match store.read_pages(&pgnos).await {
		Ok(bytes) => bytes,
		Err(error) => {
			// A refused or failed read sends no reply, so its prefetch is not in flight.
			prefetcher.plan_abandoned(planned_count);
			return Err(error);
		}
	};
	if bytes.len() != pgnos.len() {
		prefetcher.plan_abandoned(planned_count);
		anyhow::bail!("page read returned the wrong count");
	}
	let slices: Vec<Option<&[u8]>> = bytes.iter().map(|page| page.as_deref()).collect();
	let fetched = prefetcher.fetched(&plan, &slices);

	let mut kept: Vec<(Pgno, Source, Option<Bytes>)> = plan
		.pages
		.iter()
		.zip(bytes)
		.zip(&fetched.keep)
		.filter(|(_, keep)| **keep)
		.map(|(((pgno, source), bytes), _)| (*pgno, *source, bytes))
		.collect();
	let mut planned = plan.pages.len() - plan.count(Source::Requested);

	// A missed overflow page the map could not place: read its rows and send the rest of its
	// chain in this reply.
	if !fetched.followups.is_empty() {
		let ok = run_lookups(prefetcher, store, fetched.followups.clone(), "read").await;
		if let Err(error) = ok {
			tracing::debug!(?error, "sqlite prefetch followup lookup failed");
		} else {
			let already: BTreeSet<Pgno> = kept.iter().map(|(pgno, _, _)| *pgno).collect();
			let more = prefetcher.followup(&fetched, &already);
			if !more.pages.is_empty() {
				planned += more.pages.len();
				let pgnos = more.pgnos();
				match store.read_pages(&pgnos).await {
					Ok(bytes) if bytes.len() == pgnos.len() => {
						let slices: Vec<Option<&[u8]>> =
							bytes.iter().map(|page| page.as_deref()).collect();
						let checked = prefetcher.fetched(&more, &slices);
						kept.extend(
							more.pages
								.iter()
								.zip(bytes)
								.zip(&checked.keep)
								.filter(|(_, keep)| **keep)
								.map(|(((pgno, source), bytes), _)| (*pgno, *source, bytes)),
						);
					}
					Ok(_) => {}
					Err(error) => {
						tracing::debug!(?error, "sqlite prefetch followup read failed");
					}
				}
			}
		}
	}

	// The overflow pages of the leaves this reply carries, for tables that read payloads.
	let lookups = prefetcher.leaf_lookups();
	if !lookups.is_empty() {
		if let Err(error) = run_lookups(prefetcher, store, lookups, "read").await {
			tracing::debug!(?error, "sqlite prefetch leaf payload lookup failed");
		}
	}
	let already: BTreeSet<Pgno> = kept.iter().map(|(pgno, _, _)| *pgno).collect();
	let leaf = prefetcher.leaf_payloads(&already);
	if !leaf.pages.is_empty() {
		planned += leaf.pages.len();
		let pgnos = leaf.pgnos();
		match store.read_pages(&pgnos).await {
			Ok(bytes) if bytes.len() == pgnos.len() => {
				let slices: Vec<Option<&[u8]>> = bytes.iter().map(|page| page.as_deref()).collect();
				let checked = prefetcher.fetched(&leaf, &slices);
				kept.extend(
					leaf.pages
						.iter()
						.zip(bytes)
						.zip(&checked.keep)
						.filter(|(_, keep)| **keep)
						.map(|(((pgno, source), bytes), _)| (*pgno, *source, bytes)),
				);
			}
			Ok(_) => {}
			Err(error) => {
				tracing::debug!(?error, "sqlite prefetch leaf payload read failed");
			}
		}
	}

	// Requested pages in request order, duplicates included; prefetched pages after them.
	let mut reply = Reply::default();
	for &pgno in requested {
		let bytes = kept
			.iter()
			.find(|(page, source, _)| *page == pgno && *source == Source::Requested)
			.and_then(|(_, _, bytes)| bytes.clone());
		reply.pages.push((pgno, bytes));
	}
	for (pgno, source, bytes) in kept {
		if source == Source::Requested {
			continue;
		}
		let Some(bytes) = bytes else {
			continue;
		};
		reply.pages.push((pgno, Some(bytes)));
		reply.sources.push(source);
	}
	prefetcher.sent(
		reply
			.pages
			.iter()
			.filter_map(|(pgno, bytes)| bytes.as_ref().map(|_| *pgno)),
		planned,
	);
	reply.after_reply = prefetcher.take_lookups();

	record(request_id, requested, &reply);
	Ok(reply)
}

fn record(request_id: u64, requested: &[Pgno], reply: &Reply) {
	let count = |source: Source| reply.sources.iter().filter(|s| **s == source).count();
	crate::metrics::PAGES_TOTAL
		.with_label_values(&[Source::Requested.as_str()])
		.inc_by(requested.len() as u64);
	for source in Source::ALL {
		if source == Source::Requested {
			continue;
		}
		let pages = count(source);
		if pages > 0 {
			crate::metrics::PAGES_TOTAL
				.with_label_values(&[source.as_str()])
				.inc_by(pages as u64);
		}
	}
	crate::metrics::REPLY_PAGES.observe(reply.pages.len() as f64);
	tracing::trace!(
		target: "sqlite_prefetch::trace",
		event = "plan",
		request_id,
		requested = requested.len(),
		chain = count(Source::Chain),
		scan = count(Source::Scan),
		early = count(Source::Early),
		frequent = count(Source::Frequent),
		window = count(Source::Window),
		leaf = count(Source::Leaf),
		reply_pages = reply.pages.len(),
		reply_bytes = reply.pages.iter().filter_map(|(_, bytes)| bytes.as_ref()).map(Bytes::len).sum::<usize>(),
		"plan"
	);
	if tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE) {
		for (ordinal, (pgno, bytes)) in reply.pages.iter().enumerate() {
			let source = if ordinal < requested.len() {
				Source::Requested
			} else {
				reply.sources[ordinal - requested.len()]
			};
			// This is a candidate, before the server's final ownership/head check. Only
			// the native receipt event proves delivery; absent pages carry zero bytes.
			tracing::trace!(target: "sqlite_prefetch::trace", event = "page_candidate",
                request_id, ordinal, pgno, source = source.as_str(), present = bytes.is_some(),
                bytes = bytes.as_ref().map_or(0, Bytes::len));
		}
	}
}

/// Reads lookups and applies their rows, following up until the chains they found are whole.
pub async fn run_lookups<S: Store>(
	prefetcher: &mut Prefetcher,
	store: &S,
	lookups: Vec<Lookup>,
	phase: &'static str,
) -> Result<()> {
	let mut pending = lookups;
	let mut rounds = 0;
	while !pending.is_empty() && rounds < 4 {
		rounds += 1;
		let mut next = Vec::new();
		for lookup in dedupe(pending) {
			let result = execute(prefetcher, store, &lookup, phase).await?;
			if let Some(result) = result {
				next.extend(prefetcher.apply(result));
			}
		}
		pending = next;
	}
	Ok(())
}

fn dedupe(lookups: Vec<Lookup>) -> Vec<Lookup> {
	let mut seen = BTreeSet::new();
	lookups
		.into_iter()
		.filter(|lookup| seen.insert(lookup.clone()))
		.collect()
}

/// Reads one lookup. A trunk read is applied here, since it needs the page, and returns
/// `None`.
async fn execute<S: Store>(
	prefetcher: &mut Prefetcher,
	store: &S,
	lookup: &Lookup,
	phase: &'static str,
) -> Result<Option<LookupResult>> {
	let kind = match lookup {
		Lookup::Head(_) => "head",
		Lookup::Floor { .. } => "floor",
		Lookup::Trunk(_) => "trunk",
	};
	crate::metrics::LOOKUPS_TOTAL
		.with_label_values(&[kind, phase])
		.inc();
	match lookup {
		Lookup::Head(head) => {
			let value = store.kv_get(Sidecar::Overflow, &head.to_be_bytes()).await?;
			Ok(Some(LookupResult::Head { head: *head, value }))
		}
		Lookup::Floor { first, last } => {
			let span = last.checked_sub(*first).and_then(|n| n.checked_add(1));
			anyhow::ensure!(
				span.is_some_and(|n| n <= MAX_FLOOR_PAGES),
				"overflow lookup span exceeds the bounded scan"
			);
			let limit = span.unwrap() as usize;
			let rows = store
				.kv_scan(
					Sidecar::Overflow,
					&first.to_be_bytes(),
					&last.to_be_bytes(),
					limit,
				)
				.await?;
			Ok(Some(LookupResult::Floor {
				first: *first,
				last: *last,
				rows,
			}))
		}
		Lookup::Trunk(pgno) => {
			let pages = store.read_pages(&[*pgno]).await?;
			if let Some(Some(bytes)) = pages.first() {
				prefetcher.observe_trunk(*pgno, bytes);
			}
			Ok(None)
		}
	}
}

/// Plans a commit's KV ops, reading what the commit needs to know first. A read the store
/// refused before running refuses the commit, so the caller can send it again. Any other failed
/// read, or a commit that needs more rounds than [`COMMIT_LOOKUP_ROUNDS`], purges the rows
/// instead, which is always correct.
pub async fn plan_commit<S: Store>(
	prefetcher: &mut Prefetcher,
	store: &S,
	builder: &CommitBuilder,
	new_size: u32,
	txid: u64,
	budget: usize,
) -> Result<CommitPlan> {
	let mut previous: Option<Vec<Lookup>> = None;
	for _ in 0..COMMIT_LOOKUP_ROUNDS {
		let lookups = match prefetcher.plan_commit(builder, new_size, txid, budget) {
			Ok(plan) => return Ok(plan),
			Err(lookups) => lookups,
		};
		// The same lookups twice in a row resolved nothing, so more rounds would not either.
		if previous.as_ref() == Some(&lookups) {
			tracing::debug!(
				"sqlite prefetch commit lookups made no progress; purging the overflow map"
			);
			break;
		}
		previous = Some(lookups.clone());
		if let Err(error) = run_lookups(prefetcher, store, lookups, "commit").await {
			if store.refused(&error) {
				return Err(error);
			}
			tracing::debug!(
				?error,
				"sqlite prefetch commit lookup failed; purging the overflow map"
			);
			break;
		}
	}
	Ok(prefetcher.plan_purge(builder, new_size, txid, budget))
}


