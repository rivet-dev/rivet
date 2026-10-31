//! Reclaim heartbeat versions only after store timestamps prove they exceed retention.
//! Retention exceeds the lease/takeover window; a matching ETag proves no renewal or replacement
//! changed the observation. This remains safe if a delete completes after coordinator leadership changes.
//!
//! A delayed create can recreate a removed record, but cannot refresh its sender's original lease
//! deadline, even through read-back. Discovery can see it until its new Last-Modified expires;
//! same-ID takeover still waits for an unchanged record. Each further delayed write can delay
//! discovery expiry again, but cannot grant serving rights. Released gets the same retention,
//! preserving immediate clean replacement while the marker exists. No heartbeat payload GET is
//! needed, and local wall time never decides eligibility.
//!
//! The collector works in passes. One pass lists each role's heartbeat prefix once, in batches
//! bounded by `batch_size` entries and `batch_timeout_ms`, and conditionally deletes every
//! record whose `Last-Modified` is older than the retention cutoff. Passes repeat on
//! `pass_interval_ms`; see `Collector::run` for why a periodic pass is the only option here.
//!
//! Conditional delete needs the store's support: S3 honors `If-Match` on DELETE and GCS a
//! generation precondition (with generation ETags). A store without it fails every delete as
//! `unsupported`, which the collector counts and warns about instead of deleting blindly. A
//! listing that carries no ETag (GCS listings carry no generation) costs one HEAD per old
//! record, never per listed record, and the delete is conditioned on that HEAD's ETag after
//! rechecking its `Last-Modified`.

use crate::{
	config::LeaseTiming,
	heartbeat::StoreClock,
	keys::s3::{WorkerHeartbeatKey, WorkerHeartbeatPrefix},
	worker::WorkerRole,
};
use anyhow::{Context, Result, ensure};
use futures_util::{
	StreamExt,
	stream::{self, BoxStream},
};
use object_store::{ObjectMeta, ObjectStore, ObjectStoreExt, path::Path};
use rivet_keys::keys::KeyAddress;
use std::sync::Arc;

/// Separate from lease expiry: keep clean-shutdown markers available for quick restarts.
#[derive(Clone, Debug)]
pub struct Config {
	pub retention_ms: u64,
	/// Maximum LIST entries examined per batch, including recent and malformed entries. Also
	/// the number of listed candidates the collector holds before it stops listing and
	/// deletes first.
	pub batch_size: usize,
	pub delete_concurrency: usize,
	/// Budget for one batch's listing and deletes. A batch over budget is abandoned: the
	/// listing and the candidates not yet sent carry over to the next batch, while the
	/// deletes in flight are dropped and their candidates left to the next pass.
	pub batch_timeout_ms: u64,
	/// Pause between the batches of one pass, spreading LIST and DELETE load over time. Also
	/// the retry delay while topology has no store clock evidence yet.
	pub batch_interval_ms: u64,
	/// Time between the starts of consecutive passes. A pass that outlasts it starts the
	/// next pass immediately after finishing instead of catching up.
	pub pass_interval_ms: u64,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			retention_ms: 86_400_000,
			batch_size: 128,
			delete_concurrency: 4,
			batch_timeout_ms: 2_000,
			batch_interval_ms: 10_000,
			pass_interval_ms: 3_600_000,
		}
	}
}

impl Config {
	pub fn validate(&self, lease: &LeaseTiming) -> Result<()> {
		ensure!(
			self.retention_ms
				> lease
					.ttl_ms
					.saturating_add(lease.zombie_grace_ms)
					.saturating_add(lease.store_timestamp_uncertainty_ms.saturating_mul(2)),
			"heartbeat retention must exceed lease expiry and store uncertainty"
		);
		ensure!(
			(1..=1000).contains(&self.batch_size),
			"heartbeat GC batch size must be 1..=1000"
		);
		ensure!(
			(1..=self.batch_size).contains(&self.delete_concurrency),
			"heartbeat GC concurrency must be 1..=batch_size"
		);
		ensure!(
			self.batch_timeout_ms > 0 && self.batch_interval_ms > 0 && self.pass_interval_ms > 0,
			"heartbeat GC intervals must be positive"
		);

		Ok(())
	}
}

/// What one pass, or one batch of a pass, found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PassCounts {
	/// LIST entries examined, including recent and malformed ones.
	pub scanned: u64,
	/// Old entries whose listing carried no ETag and that were read with a HEAD instead.
	pub inspected: u64,
	/// Entries older than the retention cutoff with an ETag to condition the delete on.
	pub candidates: u64,
	/// Candidates whose conditional delete succeeded. Stores may report a delete of an
	/// already-removed key as success, so this can include records another collector removed.
	pub deleted: u64,
	/// Candidates that changed or disappeared between the listing and the delete.
	pub changed_or_missing: u64,
	/// Candidates whose delete failed or was unsupported; the next pass revisits them.
	pub failed: u64,
	/// Candidates whose delete was still in flight when its batch hit the time budget; the
	/// next pass revisits them.
	pub abandoned: u64,
	/// Listings that failed before reaching the end of their prefix.
	pub listing_errors: u64,
	/// Batches abandoned at `batch_timeout_ms`; their partial counts are not included.
	pub timed_out_batches: u64,
}

impl PassCounts {
	fn add(&mut self, other: PassCounts) {
		self.scanned += other.scanned;
		self.inspected += other.inspected;
		self.candidates += other.candidates;
		self.deleted += other.deleted;
		self.changed_or_missing += other.changed_or_missing;
		self.failed += other.failed;
		self.abandoned += other.abandoned;
		self.listing_errors += other.listing_errors;
		self.timed_out_batches += other.timed_out_batches;
	}
}

/// The listing of one role's prefix in progress during a pass. It retains the S3 continuation
/// token and at most one provider page (1,000 entries) across batches, including batches
/// abandoned at their time budget. Never restart a successful listing at a batch boundary or
/// infer a cursor from unordered keys.
pub(crate) struct Listing {
	pub(crate) role: WorkerRole,
	pub(crate) entries: BoxStream<'static, object_store::Result<ObjectMeta>>,
}

/// A listed record older than the cutoff, with the ETag its delete is conditioned on.
#[derive(Clone, Debug)]
pub(crate) struct Candidate {
	role: WorkerRole,
	location: Path,
	e_tag: String,
}

pub(crate) struct Collector {
	store: Arc<dyn ObjectStore>,
	config: Config,
	pub(crate) listing: Option<Listing>,
	/// Candidates listed but not yet deleted, at most `2 * batch_size`: a batch lists only
	/// while fewer than `batch_size` are queued. A candidate stays until its delete returns
	/// or its batch is abandoned with the delete in flight.
	pub(crate) pending: Vec<Candidate>,
	/// How many candidates at the front of `pending` have deletes in flight. A batch
	/// abandoned at its deadline drops those deletes, so the pass removes the candidates
	/// too; otherwise a store that never answers one delete would pin the pass.
	in_flight: usize,
	capped: bool,
	delete_capped: bool,
	delete_failed: bool,
	timed_out: bool,
	clock_unknown: bool,
	no_etag_warned: bool,
}

impl Collector {
	pub(crate) fn new(store: Arc<dyn ObjectStore>, config: Config) -> Self {
		Self {
			store,
			config,
			listing: None,
			pending: Vec::new(),
			in_flight: 0,
			capped: false,
			delete_capped: false,
			delete_failed: false,
			timed_out: false,
			clock_unknown: false,
			no_etag_warned: false,
		}
	}

	pub(crate) async fn run(
		mut self,
		env: Arc<dyn rivet_env::Env>,
		topology: Arc<crate::Topology>,
	) -> Result<()> {
		let config = self.config.clone();
		config.validate(&topology.config().lease)?;

		for (resource, value) in [
			("batch_size", i64::try_from(config.batch_size)?),
			(
				"delete_concurrency",
				i64::try_from(config.delete_concurrency)?,
			),
			("batch_timeout_ms", i64::try_from(config.batch_timeout_ms)?),
			(
				"batch_interval_ms",
				i64::try_from(config.batch_interval_ms)?,
			),
			("pass_interval_ms", i64::try_from(config.pass_interval_ms)?),
		] {
			crate::metrics::HEARTBEAT_GC_CAPACITY
				.with_label_values(&[resource])
				.set(value);
		}

		// A periodic pass is the only option here: a worker that dies without releasing its
		// heartbeat sends nothing, and object storage emits no event when a lease deadline
		// passes, so the only evidence that a record is reclaimable is its Last-Modified seen
		// in a listing. Each pass is bounded and paginated, and the interval skips missed
		// ticks instead of catching up.
		let clock = env.clock();
		let mut passes = rivet_env::interval(clock.clone(), config.pass_interval_ms);
		loop {
			passes.tick().await?;
			// Topology may not have store clock evidence right after this tenure started.
			// Retry on the batch interval rather than losing a whole pass interval.
			while self.pass(clock.as_ref(), &topology).await?.is_none() {
				let retry_at = clock
					.observe()?
					.boot_ms
					.checked_add(config.batch_interval_ms)
					.context("heartbeat GC interval overflow")?;
				clock.sleep_until(retry_at).await?;
			}
		}
	}

	/// Lists both roles once and deletes every record past the retention cutoff. Returns
	/// `None` without listing anything when topology has no store clock evidence yet. Returns
	/// an error only when this worker lost its own lease or the clock failed; listing and
	/// delete faults are counted and retried by the next pass.
	pub(crate) async fn pass(
		&mut self,
		clock: &dyn rivet_env::Clock,
		topology: &crate::Topology,
	) -> Result<Option<PassCounts>> {
		let _duration = crate::metrics::HEARTBEAT_GC_PASS_DURATION.start_timer();
		let mut counts = PassCounts::default();

		for role in [WorkerRole::Storage, WorkerRole::Stateless] {
			self.listing = Some(Listing {
				role,
				entries: self
					.store
					.list(Some(&WorkerHeartbeatPrefix::new(role).pack())),
			});
			while self.listing.is_some() || !self.pending.is_empty() {
				let now = clock.observe()?;
				let cutoff = {
					let snapshot = topology.snapshot()?;
					ensure!(
						snapshot.may_admit(now.boot_ms),
						"heartbeat GC local lease ended"
					);
					cutoff(
						snapshot.store_clock.as_ref(),
						now.boot_ms,
						self.config.retention_ms,
						topology.config().lease.store_timestamp_uncertainty_ms,
					)
				};
				let Some(cutoff) = cutoff else {
					// Without store time evidence nothing can be proven old. Give up this
					// pass rather than extrapolating from a local clock; the candidates
					// already listed were proven old by an earlier cutoff and stay pending.
					self.listing = None;
					crate::metrics::HEARTBEAT_GC_PASSES
						.with_label_values(&["clock_unknown"])
						.inc();
					if !self.clock_unknown {
						tracing::warn!(
							"heartbeat GC pass skipped because topology has no store clock evidence yet"
						);
					}
					self.clock_unknown = true;
					return Ok(None);
				};
				self.clock_unknown = false;

				let deadline = now
					.boot_ms
					.checked_add(self.config.batch_timeout_ms)
					.context("heartbeat GC deadline overflow")?;
				let batch = rivet_env::select_biased!("GC batch deadline wins over more work",
					result = clock.sleep_until(deadline) => { result?; None },
					counts = self.batch(cutoff) => Some(counts),
				);
				match batch {
					Some(batch) => {
						counts.add(batch);
						self.timed_out = false;
					}
					None => {
						// The dropped batch future dropped its in-flight deletes with it.
						let abandoned = self.in_flight.min(self.pending.len());
						self.pending.drain(..abandoned);
						self.in_flight = 0;
						counts.abandoned += abandoned as u64;
						counts.timed_out_batches += 1;
						crate::metrics::HEARTBEAT_GC_CAP_HITS
							.with_label_values(&["batch_timeout_ms"])
							.inc();
						if !self.timed_out {
							tracing::warn!(
								capacity_ms = self.config.batch_timeout_ms,
								abandoned,
								pending = self.pending.len(),
								"heartbeat GC batch time cap reached; the listing and queued deletes continue next batch"
							);
						}
						self.timed_out = true;
					}
				}

				if self.listing.is_some() || !self.pending.is_empty() {
					let next = clock
						.observe()?
						.boot_ms
						.checked_add(self.config.batch_interval_ms)
						.context("heartbeat GC interval overflow")?;
					clock.sleep_until(next).await?;
				}
			}
		}

		crate::metrics::HEARTBEAT_GC_PASSES
			.with_label_values(&["completed"])
			.inc();
		for (result, value) in [
			("scanned", counts.scanned),
			("inspected", counts.inspected),
			("candidates", counts.candidates),
			("deleted", counts.deleted),
			("changed_or_missing", counts.changed_or_missing),
			("failed", counts.failed),
			("abandoned", counts.abandoned),
		] {
			crate::metrics::HEARTBEAT_GC_PASS_ENTRIES
				.with_label_values(&[result])
				.set(i64::try_from(value).unwrap_or(i64::MAX));
		}
		tracing::info!(
			scanned = counts.scanned,
			inspected = counts.inspected,
			candidates = counts.candidates,
			deleted = counts.deleted,
			changed_or_missing = counts.changed_or_missing,
			failed = counts.failed,
			abandoned = counts.abandoned,
			listing_errors = counts.listing_errors,
			timed_out_batches = counts.timed_out_batches,
			"heartbeat GC pass finished"
		);

		Ok(Some(counts))
	}

	/// Examines up to `batch_size` entries of the current listing while fewer than
	/// `batch_size` candidates are queued, then conditionally deletes every queued candidate.
	/// Clears `self.listing` once the prefix is exhausted or the listing failed. Cancellation
	/// drops in-flight deletes; their outcome may be unknown, so correctness relies on ETags,
	/// not cancellation. A candidate leaves `self.pending` once its delete returned, or when
	/// `pass` abandons the batch with that delete in flight; failed deletes are revisited on
	/// the next pass.
	pub(crate) async fn batch(&mut self, cutoff_unix_ms: i64) -> PassCounts {
		let _duration = crate::metrics::HEARTBEAT_GC_BATCH_DURATION.start_timer();
		let mut counts = PassCounts::default();

		// Scan one bounded slice of the listing, unless the queue already holds a batch.
		if self.pending.len() < self.config.batch_size {
			if let Some(listing) = self.listing.as_mut() {
				let role = listing.role;
				let label = role_label(role);
				let mut exhausted = false;
				let mut no_etag = false;
				while counts.scanned < self.config.batch_size as u64 {
					let meta = match listing.entries.next().await {
						Some(Ok(meta)) => meta,
						Some(Err(error)) => {
							counts.listing_errors += 1;
							crate::metrics::HEARTBEAT_GC_SCAN
								.with_label_values(&[label, "error"])
								.inc();
							tracing::warn!(
								%error,
								"heartbeat GC listing failed; retrying next pass"
							);
							exhausted = true;
							break;
						}
						None => {
							exhausted = true;
							break;
						}
					};
					counts.scanned += 1;
					let valid_key = WorkerHeartbeatKey::unpack(&meta.location)
						.is_ok_and(|key| key.role == role);
					let old = meta.last_modified.timestamp_millis() < cutoff_unix_ms;
					let candidate = match (valid_key && old, meta.e_tag) {
						(true, Some(e_tag)) => Some(Candidate {
							role,
							location: meta.location,
							e_tag,
						}),
						(true, None) => {
							// The listing carried no ETag to condition the delete on. Read
							// the record's metadata, and only trust an ETag whose own
							// Last-Modified is still past the cutoff.
							counts.inspected += 1;
							crate::metrics::HEARTBEAT_GC_SCAN
								.with_label_values(&[label, "inspected"])
								.inc();
							match inspect(self.store.as_ref(), role, meta.location, cutoff_unix_ms)
								.await
							{
								Inspection::Candidate(candidate) => Some(candidate),
								Inspection::Retained => None,
								Inspection::NoEtag => {
									no_etag = true;
									None
								}
							}
						}
						(false, _) => None,
					};
					crate::metrics::HEARTBEAT_GC_SCAN
						.with_label_values(&[
							label,
							if candidate.is_some() {
								"candidate"
							} else {
								"retained"
							},
						])
						.inc();
					if let Some(candidate) = candidate {
						counts.candidates += 1;
						self.pending.push(candidate);
					}
				}
				if exhausted {
					self.listing = None;
				}
				if no_etag && !self.no_etag_warned {
					tracing::warn!(
						"heartbeat GC found old records whose metadata carries no ETag; the object store must expose ETags (generation ETags on GCS) for them to be reclaimed"
					);
				}
				self.no_etag_warned = no_etag;

				crate::metrics::HEARTBEAT_GC_BATCH_ENTRIES
					.set(i64::try_from(counts.scanned).unwrap_or(i64::MAX));
				let capped = counts.scanned == self.config.batch_size as u64;
				if capped {
					crate::metrics::HEARTBEAT_GC_CAP_HITS
						.with_label_values(&["batch_size"])
						.inc();
					if !self.capped {
						tracing::warn!(
							capacity = self.config.batch_size,
							"heartbeat GC batch cap reached; continuing next batch"
						);
					}
				}
				self.capped = capped;
			}
		}

		let delete_capped = self.pending.len() > self.config.delete_concurrency;
		if delete_capped {
			crate::metrics::HEARTBEAT_GC_CAP_HITS
				.with_label_values(&["delete_concurrency"])
				.inc();
			if !self.delete_capped {
				tracing::warn!(
					capacity = self.config.delete_concurrency,
					pending = self.pending.len(),
					"heartbeat GC delete concurrency reached; remaining candidates queued"
				);
			}
		}
		self.delete_capped = delete_capped;

		// Delete the queued candidates, `delete_concurrency` at a time, each conditioned on
		// the ETag the listing or inspection returned. A chunk leaves the queue only after
		// every delete in it returned; `in_flight` tells an abandoning pass which ones to drop.
		while !self.pending.is_empty() {
			let chunk_len = self.pending.len().min(self.config.delete_concurrency);
			self.in_flight = chunk_len;
			let store = self.store.clone();
			let outcomes: Vec<DeleteOutcome> = stream::iter(self.pending[..chunk_len].to_vec())
				.map(|candidate| {
					let store = store.clone();
					async move {
						use rivet_metrics::GaugeGuardExt;
						let _pending = crate::metrics::HEARTBEAT_GC_DELETES_IN_FLIGHT.inc_guard();
						let outcome = match store
							.delete_if_match(&candidate.location, &candidate.e_tag)
							.await
						{
							Ok(()) => DeleteOutcome::Deleted,
							Err(
								object_store::Error::NotFound { .. }
								| object_store::Error::Precondition { .. },
							) => DeleteOutcome::ChangedOrMissing,
							Err(
								object_store::Error::NotSupported { .. }
								| object_store::Error::NotImplemented { .. },
							) => DeleteOutcome::Unsupported,
							Err(_) => DeleteOutcome::Error,
						};
						crate::metrics::HEARTBEAT_GC_DELETE
							.with_label_values(&[role_label(candidate.role), outcome.label()])
							.inc();
						outcome
					}
				})
				.buffer_unordered(chunk_len)
				.collect()
				.await;
			self.pending.drain(..chunk_len);
			self.in_flight = 0;
			for outcome in outcomes {
				match outcome {
					DeleteOutcome::Deleted => counts.deleted += 1,
					DeleteOutcome::ChangedOrMissing => counts.changed_or_missing += 1,
					DeleteOutcome::Unsupported | DeleteOutcome::Error => counts.failed += 1,
				}
			}
		}
		let failed = counts.failed > 0;
		if failed && !self.delete_failed {
			tracing::warn!(
				"conditional heartbeat cleanup failed or is unsupported by the object store; retained for later passes"
			);
		}
		self.delete_failed = failed;

		counts
	}
}

/// Outcome of reading one old record's metadata because its listing carried no ETag.
enum Inspection {
	Candidate(Candidate),
	/// Not old by the HEAD's own `Last-Modified`, missing, or unreadable right now.
	Retained,
	/// Old, but the HEAD carries no ETag either, so no delete can be conditioned on it.
	NoEtag,
}

/// Reads the record's current metadata for a listing without ETags. Only a HEAD whose own
/// `Last-Modified` is past the cutoff yields a candidate, so the ETag and the age evidence
/// come from the same response. A read failure retains the record for the next pass.
async fn inspect(
	store: &dyn ObjectStore,
	role: WorkerRole,
	location: Path,
	cutoff_unix_ms: i64,
) -> Inspection {
	let head = match store.head(&location).await {
		Ok(head) => head,
		Err(object_store::Error::NotFound { .. }) => return Inspection::Retained,
		Err(error) => {
			tracing::warn!(%error, "heartbeat GC metadata read failed; retrying next pass");
			return Inspection::Retained;
		}
	};
	if head.last_modified.timestamp_millis() >= cutoff_unix_ms {
		return Inspection::Retained;
	}
	match head.e_tag {
		Some(e_tag) => Inspection::Candidate(Candidate {
			role,
			location,
			e_tag,
		}),
		None => Inspection::NoEtag,
	}
}

#[derive(Clone, Copy, Debug)]
enum DeleteOutcome {
	Deleted,
	ChangedOrMissing,
	Unsupported,
	Error,
}

impl DeleteOutcome {
	fn label(self) -> &'static str {
		match self {
			Self::Deleted => "deleted",
			Self::ChangedOrMissing => "changed_or_missing",
			Self::Unsupported => "unsupported",
			Self::Error => "error",
		}
	}
}

fn role_label(role: WorkerRole) -> &'static str {
	match role {
		WorkerRole::Storage => "storage",
		WorkerRole::Stateless => "stateless",
	}
}

/// Use an observed store timestamp without extrapolating it with a local wall clock or a stale
/// monotonic anchor. Subtract uncertainty for both the anchor and candidate Last-Modified.
pub(crate) fn cutoff(
	clock: Option<&StoreClock>,
	now_boot_ms: u64,
	retention_ms: u64,
	uncertainty_ms: u64,
) -> Option<i64> {
	let clock = clock.filter(|clock| clock.at_boot_ms <= now_boot_ms)?;
	let cutoff =
		i128::from(clock.lower_unix_ms) - i128::from(retention_ms) - 2 * i128::from(uncertainty_ms);
	i64::try_from(cutoff).ok()
}


