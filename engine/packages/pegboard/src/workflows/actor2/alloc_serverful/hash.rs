use std::{
	ops::{Bound, RangeBounds},
	sync::atomic::{AtomicU32, Ordering},
};

use futures_util::TryStreamExt;
use gas::prelude::*;
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use universaldb::{prelude::*, utils::end_of_key_range};
use xxhash_rust::xxh3::xxh3_128_with_seed;

use super::{AllocStats, read_isolation};
use crate::{envoy_expire_scheduler, keys, metrics, workflows::actor2::HashAllocatorReadStats};

/// Power-of-K-choices allocator over the EnvoyHashIdxKey ring.
///
/// Behavior by `samples`:
/// - `samples == 1` — **uniform random pick (short-circuit).** Draws one random
///   ring pivot, returns the first fresh envoy. Skips the `SlotsKey` read
///   entirely (no comparison to do with a single candidate). The dedupe,
///   tiebreak, and slot-read paths below are all unreachable. This is
///   semantically equivalent to a "random pick" strategy with zero slot
///   awareness. Per-allocation cost: **3 snapshot reads** (1 highest-version
///   range read + 1 hash-index range read + 1 `LastPingTsKey` lookup). The
///   legacy `RandomPingTimestamp` strategy pays **2 reads** (the LB index
///   key embeds `last_ping_ts`); this variant pays the extra lookup as the
///   cost of decoupling membership from ping freshness. Pick this for pools
///   with uniform actor cost, short actor lifetimes, or when measurements
///   don't yet justify load-aware reads.
/// - `samples >= 2` — power-of-K choices. K independent random pivots →
///   K fresh envoys (deduped) → K `SlotsKey` reads → min with `slot_jitter`
///   randomization → random tiebreak on ties.
///
/// `slot_jitter` defends against concurrent-allocator herd. See `slot_jitter`
/// block below for the sizing argument.
pub(super) async fn allocate(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	stats: &mut AllocStats,
	now: i64,
	envoy_eligible_threshold: i64,
	samples: u8,
	max_scan: u32,
	slot_jitter: u8,
	use_snapshot_read: bool,
) -> Result<Option<String>> {
	let mut rng = ThreadHashAllocatorRng;
	allocate_with_rng(
		namespace_id,
		pool_name,
		tx,
		pools,
		stats,
		now,
		envoy_eligible_threshold,
		samples,
		max_scan,
		slot_jitter,
		use_snapshot_read,
		&mut rng,
		None,
	)
	.await
}

// Why `slot_jitter` exists and how the default was sized.
//
// The hash allocator reads `SlotsKey` snapshot-isolated and picks the
// minimum-slot envoy. `SlotsKey` is not incremented inside the allocator
// transaction; it is bumped later in `set_connectable`. So during a burst,
// many in-flight allocator transactions all observe the same `SlotsKey`
// value for any given envoy, and all decide the same low-slot envoy is
// the minimum. That envoy then gets a thundering herd of new actors all
// at once, even though the load balancer believed it was picking the
// least-loaded one.
//
// Concretely, the number of allocators racing on the same stale snapshot
// of `SlotsKey` for one (namespace, pool) is approximately
//
//   in_flight = allocation_rate * allocator_tx_duration
//
// where allocation_rate is allocations per second into that pool and
// allocator_tx_duration is the time from FDB transaction open to commit.
// Without jitter, all `in_flight` allocators land on the same envoy.
//
// `slot_jitter` adds an independent random integer in `0..slot_jitter` to
// each candidate's slot count before comparing. With independent jitter
// per allocator, two allocators that read the same slot counts pick
// different minimums. If the K candidates differ by some `slot_gap`,
// jitter dominates the comparison whenever `slot_jitter >= slot_gap`, so
// candidates within `slot_jitter` slots of the true minimum are sampled
// roughly uniformly. The herd of size `in_flight` spreads across roughly
// `near_min_count` envoys instead of one, giving per-envoy excess of
// about `in_flight / near_min_count`.
//
// Default = 4 covers Rivet's expected steady-state and modest-burst load:
//
//   steady_state:   allocation_rate ~= 10/s    tx_duration ~= 5ms
//                   in_flight ~= 0.05    (jitter only breaks ties)
//   modest_burst:   allocation_rate ~= 100/s   tx_duration ~= 5ms
//                   in_flight ~= 0.5
//   heavy_burst:    allocation_rate ~= 1000/s  tx_duration ~= 10ms
//                   in_flight ~= 10   (jitter spreads herd over ~K envoys)
//
// At loads above heavy_burst, jitter alone is insufficient; the right fix
// then is in-transaction slot reservation, not a larger jitter. Operators
// can bump `slot_jitter` to roughly the measured `in_flight` for their
// pool, capped by the steady-state cost: a larger jitter means we more
// often pick a +jitter envoy over the true minimum. Range is 0..=64.
pub(super) async fn allocate_with_rng(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	stats: &mut AllocStats,
	now: i64,
	envoy_eligible_threshold: i64,
	samples: u8,
	max_scan: u32,
	slot_jitter: u8,
	use_snapshot_read: bool,
	rng: &mut impl HashAllocatorRng,
	mut read_stats: Option<&mut HashAllocatorReadStats>,
) -> Result<Option<String>> {
	debug_assert!(samples >= 1, "samples must be >= 1");

	let ping_threshold_ts = now - envoy_eligible_threshold;
	let read_isolation = read_isolation(use_snapshot_read);
	let Some(highest_version) =
		read_highest_version(namespace_id, pool_name, tx, read_isolation).await?
	else {
		return Ok(None);
	};

	// One shared stale-entry budget covers every scan_for_fresh call in this
	// allocation: forward + wrap across all K samples. Per-direction or
	// per-sample budgets let pathological pools burn K * 2 * max_scan
	// LastPingTs reads against the same stale entries. AtomicU32 because all
	// K scans run concurrently and decrement the same counter.
	let remaining_scan = AtomicU32::new(max_scan);

	// Pre-draw all K pivots from the RNG before spawning the parallel scans.
	// next_pivot() takes &mut self, and the futures must not retain a mut
	// borrow on the rng across .await points.
	let pivots: Vec<[u8; 16]> = (0..samples).map(|_| rng.next_pivot()).collect();

	// Run all K scans concurrently. Each scan does its forward range read,
	// then (if the pivot landed in an empty/stale region and budget remains)
	// the wrap range read. UDB allows concurrent reads on the same
	// transaction, see `update_ping` and `expire` for prior art.
	let scan_futures = pivots.iter().map(|pivot| {
		let pivot = *pivot;
		let remaining_scan = &remaining_scan;
		async move {
			let first = scan_for_fresh(
				tx,
				pools,
				namespace_id,
				pool_name,
				highest_version,
				pivot..,
				ping_threshold_ts,
				read_isolation,
				remaining_scan,
			)
			.await?;
			if first.envoy_key.is_some() || remaining_scan.load(Ordering::Relaxed) == 0 {
				return Ok::<_, anyhow::Error>((first, None));
			}
			let second = scan_for_fresh(
				tx,
				pools,
				namespace_id,
				pool_name,
				highest_version,
				..pivot,
				ping_threshold_ts,
				read_isolation,
				remaining_scan,
			)
			.await?;
			Ok((first, Some(second)))
		}
	});
	let scan_results: Vec<(ScanOutcome, Option<ScanOutcome>)> =
		futures_util::future::try_join_all(scan_futures).await?;

	// Merge per-scan stats back into the caller's accumulators, then collect
	// unique envoy keys for the slot-read phase.
	let mut envoy_candidates: Vec<String> = Vec::with_capacity(samples as usize);
	for (first, second) in &scan_results {
		stats.scan_depth = stats.scan_depth.saturating_add(first.scanned);
		if let Some(rs) = read_stats.as_deref_mut() {
			rs.last_ping_ts_reads += first.last_ping_ts_reads as usize;
		}
		if let Some(s) = second {
			stats.wrapped();
			stats.scan_depth = stats.scan_depth.saturating_add(s.scanned);
			if let Some(rs) = read_stats.as_deref_mut() {
				rs.last_ping_ts_reads += s.last_ping_ts_reads as usize;
			}
		}

		let key = first
			.envoy_key
			.clone()
			.or_else(|| second.as_ref().and_then(|s| s.envoy_key.clone()));
		let Some(key) = key else { continue };
		if envoy_candidates.contains(&key) {
			let namespace_id = namespace_id.to_string();
			metrics::ENVOY_LB_SAMPLE_DEDUPE_TOTAL
				.with_label_values(&[namespace_id.as_str(), pool_name])
				.inc();
			continue;
		}
		envoy_candidates.push(key);
	}

	if samples == 1 {
		if let Some(envoy_key) = envoy_candidates.pop() {
			if use_snapshot_read {
				add_chosen_envoy_hash_conflict(
					tx,
					namespace_id,
					pool_name,
					highest_version,
					&envoy_key,
				)?;
			}
			return Ok(Some(envoy_key));
		}
		return Ok(None);
	}

	// K >= 2: parallel SlotsKey reads on the (deduped) candidate set.
	if let Some(rs) = read_stats.as_deref_mut() {
		rs.slots_reads += envoy_candidates.len();
	}
	let slot_futures = envoy_candidates.iter().map(|envoy_key| {
		let envoy_key = envoy_key.clone();
		async move {
			let key = keys::envoy::SlotsKey::new(namespace_id, envoy_key);
			tx.read_opt(&key, read_isolation).await
		}
	});
	let slots: Vec<Option<i64>> = futures_util::future::try_join_all(slot_futures).await?;
	let candidates: Vec<(String, i64)> = envoy_candidates
		.into_iter()
		.zip(slots)
		.map(|(k, s)| (k, s.unwrap_or(0)))
		.collect();

	if samples >= 2 {
		let namespace_id_str = namespace_id.to_string();
		metrics::ENVOY_LB_SAMPLES_EFFECTIVE
			.with_label_values(&[namespace_id_str.as_str(), pool_name])
			.observe(candidates.len() as f64);
	}

	if candidates.is_empty() {
		// No fresh envoy found across K samples. None is the contract for
		// "no envoy available right now": the workflow engine retries the
		// allocation activity at a fresh FDB snapshot on its next tick.
		// Stale entries observed during the scans have been enqueued for
		// BG expire; by the time the workflow retries, they may be cleared
		// from the index. Matches the legacy RandomPingTimestamp behavior
		// for the "all envoys stale" case.
		return Ok(None);
	}

	// Apply slot_jitter to each candidate's slot count before the min
	// comparison. See the `slot_jitter` doc block above for the herd this
	// defends against and how the default was sized.
	let jittered: Vec<(String, i64, i64)> = candidates
		.into_iter()
		.map(|(envoy_key, slots)| {
			let jitter = rng.next_slot_jitter(slot_jitter) as i64;
			let effective = slots.saturating_add(jitter);
			(envoy_key, slots, effective)
		})
		.collect();

	let effective_min = jittered.iter().map(|(_, _, e)| *e).min().unwrap();
	let tied = jittered
		.iter()
		.filter(|(_, _, e)| *e == effective_min)
		.map(|(k, s, _)| (k.clone(), *s))
		.collect::<Vec<_>>();

	if tied.len() > 1 {
		let namespace_id = namespace_id.to_string();
		metrics::ENVOY_LB_TIED_MIN_TOTAL
			.with_label_values(&[namespace_id.as_str(), pool_name])
			.inc();
	}

	let tied_count = tied.len();
	let tied_index = rng.choose_tied_index(tied_count);
	let (envoy_key, _) = tied
		.into_iter()
		.nth(tied_index)
		.context("expected hash allocator tiebreak index in range")?;

	if use_snapshot_read {
		add_chosen_envoy_hash_conflict(tx, namespace_id, pool_name, highest_version, &envoy_key)?;
	}

	Ok(Some(envoy_key))
}

pub(super) trait HashAllocatorRng {
	fn next_pivot(&mut self) -> [u8; 16];

	fn choose_tied_index(&mut self, len: usize) -> usize;

	/// Random integer in `0..max`, used as additive slot jitter. Returns `0`
	/// when `max == 0` so callers can leave the call unconditional.
	fn next_slot_jitter(&mut self, max: u8) -> u8;
}

struct ThreadHashAllocatorRng;

impl HashAllocatorRng for ThreadHashAllocatorRng {
	fn next_pivot(&mut self) -> [u8; 16] {
		rand::random::<u128>().to_be_bytes()
	}

	fn choose_tied_index(&mut self, len: usize) -> usize {
		rand::thread_rng().gen_range(0..len)
	}

	fn next_slot_jitter(&mut self, max: u8) -> u8 {
		if max == 0 {
			0
		} else {
			rand::thread_rng().gen_range(0..max)
		}
	}
}

pub(super) struct SeededHashAllocatorRng {
	pivots: std::collections::VecDeque<[u8; 16]>,
	rng: StdRng,
}

impl SeededHashAllocatorRng {
	pub(super) fn new(pivots: Vec<[u8; 16]>, seed: u64) -> Self {
		SeededHashAllocatorRng {
			pivots: pivots.into(),
			rng: StdRng::seed_from_u64(seed),
		}
	}
}

impl HashAllocatorRng for SeededHashAllocatorRng {
	fn next_pivot(&mut self) -> [u8; 16] {
		self.pivots.pop_front().unwrap_or_else(|| {
			let mut pivot = [0; 16];
			self.rng.fill_bytes(&mut pivot);
			pivot
		})
	}

	fn choose_tied_index(&mut self, len: usize) -> usize {
		self.rng.gen_range(0..len)
	}

	fn next_slot_jitter(&mut self, max: u8) -> u8 {
		if max == 0 {
			0
		} else {
			self.rng.gen_range(0..max)
		}
	}
}

/// Walks the (namespace, pool, -version) hash subspace from `range`,
/// returning the first envoy whose LastPingTsKey is at or above
/// `ping_threshold_ts`.
///
/// INVARIANT: this function never observes an envoy whose ExpiredTsKey
/// is set. The expire op deletes the V EnvoyHashIdxKey entries
/// atomically with the ExpiredTsKey write (same FDB tx), so a hash-
/// index entry's existence implies the envoy is not expired. The
/// freshness check on LastPingTsKey is for the "lost-host" window
/// (between last heartbeat and lost_timeout firing); it is NOT a
/// defensive check against the impossible "expired in index" case.
/// Any future code path that writes ExpiredTsKey MUST also delete the
/// V hash entries in the same transaction, or this function will
/// return ghosts.
///
/// Stale-envoy retry: stale candidates are skipped-and-continued
/// inline (we never return a stale envoy to the caller). The
/// observation is enqueued into the per-process EnvoyExpireScheduler
/// for BG cleanup. If the scan exhausts without finding a fresh
/// envoy, returns None and the caller (Hash::allocate) may try
/// another sample (K>=2) or return None to the activity. The
/// workflow engine retries the allocation activity at a fresh FDB
/// snapshot on its next tick; by then the BG expire ops may have
/// cleaned up the stale entries we observed.
/// Per-scan outcome returned from `scan_for_fresh`. The caller merges these
/// into the shared `AllocStats` / `HashAllocatorReadStats` after all parallel
/// scans complete.
#[derive(Default)]
struct ScanOutcome {
	envoy_key: Option<String>,
	/// Total entries observed (fresh + stale).
	scanned: u32,
	/// `LastPingTsKey` reads issued (1 per entry observed).
	last_ping_ts_reads: u32,
}

async fn scan_for_fresh<R>(
	tx: &universaldb::Transaction,
	pools: &rivet_pools::PoolsHandle,
	namespace_id: Id,
	pool_name: &str,
	version: u32,
	range: R,
	ping_threshold_ts: i64,
	read_isolation: universaldb::utils::IsolationLevel,
	remaining_scan: &AtomicU32,
) -> Result<ScanOutcome>
where
	R: RangeBounds<[u8; 16]>,
{
	let subspace = keys::subspace().subspace(&keys::ns::EnvoyHashIdxKey::subspace(
		namespace_id,
		pool_name.to_string(),
		version,
	));
	let subspace_range = subspace.range();
	let start_key = match range.start_bound() {
		Bound::Included(hash_pos) => hash_position_key(namespace_id, pool_name, version, *hash_pos),
		Bound::Excluded(hash_pos) => end_of_key_range(&hash_position_key(
			namespace_id,
			pool_name,
			version,
			*hash_pos,
		)),
		Bound::Unbounded => subspace_range.0,
	};
	let end_key = match range.end_bound() {
		Bound::Included(hash_pos) => end_of_key_range(&hash_position_key(
			namespace_id,
			pool_name,
			version,
			*hash_pos,
		)),
		Bound::Excluded(hash_pos) => hash_position_key(namespace_id, pool_name, version, *hash_pos),
		Bound::Unbounded => subspace_range.1,
	};

	let mut stream = tx.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			..(start_key.as_slice(), end_key.as_slice()).into()
		},
		read_isolation,
	);

	let mut outcome = ScanOutcome::default();
	let mut stale_count: u32 = 0;

	while let Some(entry) = stream.try_next().await? {
		outcome.scanned = outcome.scanned.saturating_add(1);
		let (hash_key, _) = tx.read_entry::<keys::ns::EnvoyHashIdxKey>(&entry)?;
		outcome.last_ping_ts_reads = outcome.last_ping_ts_reads.saturating_add(1);
		let last_ping_ts = tx
			.read_opt(
				&keys::envoy::LastPingTsKey::new(namespace_id, hash_key.envoy_key.clone()),
				read_isolation,
			)
			.await?;

		if last_ping_ts.is_some_and(|ts| ts >= ping_threshold_ts) {
			outcome.envoy_key = Some(hash_key.envoy_key);
			return Ok(outcome);
		}

		// Stale envoy. Skip-and-continue: we never return a stale envoy to
		// the allocator caller. Fire-and-forget a BG expire via the
		// read-path scheduler (single-flight per process), then continue
		// the scan. No throw, no error. The scheduler invokes
		// pegboard_envoy_expire { skip_if_fresh: true }, which re-checks
		// LastPingTsKey + ExpiredTsKey (Serializable) inside its own FDB
		// transaction, closing the TOCTOU window if a heartbeat lands
		// between our observation here and the op's commit.
		envoy_expire_scheduler::get(pools).try_enqueue(namespace_id, hash_key.envoy_key.clone());

		stale_count += 1;
		// Consume one unit of the shared per-allocation stale budget via CAS.
		// CAS rather than `fetch_sub` so we never wrap past zero when other
		// parallel scans also race to consume. If `prev` is 0 the budget was
		// already drained by a sibling scan; bail without claiming a unit. If
		// `prev` is 1 we just took the last unit, so we fire the breaker
		// metric and return.
		let mut prev = remaining_scan.load(Ordering::Relaxed);
		while prev > 0 {
			match remaining_scan.compare_exchange_weak(
				prev,
				prev - 1,
				Ordering::Relaxed,
				Ordering::Relaxed,
			) {
				Ok(_) => break,
				Err(actual) => prev = actual,
			}
		}
		if prev <= 1 {
			tracing::warn!(
				?namespace_id,
				%pool_name,
				version,
				stale_count,
				"envoy_lb scan_for_fresh exhausted shared max_scan budget; aborting scan"
			);
			let namespace_id = namespace_id.to_string();
			metrics::ENVOY_LB_SCAN_CIRCUIT_BREAKER_TOTAL
				.with_label_values(&[namespace_id.as_str(), pool_name, "hash"])
				.inc();
			return Ok(outcome);
		}
	}

	Ok(outcome)
}

async fn read_highest_version(
	namespace_id: Id,
	pool_name: &str,
	tx: &universaldb::Transaction,
	read_isolation: universaldb::utils::IsolationLevel,
) -> Result<Option<u32>> {
	let subspace = keys::subspace().subspace(&keys::ns::EnvoyLoadBalancerIdxKey::subspace(
		namespace_id,
		pool_name.to_string(),
	));
	let mut stream = tx.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			limit: Some(1),
			..(&subspace).into()
		},
		read_isolation,
	);

	let Some(entry) = stream.try_next().await? else {
		return Ok(None);
	};
	let (lb_key, _) = tx.read_entry::<keys::ns::EnvoyLoadBalancerIdxKey>(&entry)?;

	Ok(Some(lb_key.version))
}

fn add_chosen_envoy_hash_conflict(
	tx: &universaldb::Transaction,
	namespace_id: Id,
	pool_name: &str,
	version: u32,
	envoy_key: &str,
) -> Result<()> {
	tx.add_conflict_key(
		&keys::ns::EnvoyHashIdxKey::new(
			namespace_id,
			pool_name.to_string(),
			version,
			xxh3_128_with_seed(envoy_key.as_bytes(), 0).to_be_bytes(),
			envoy_key.to_string(),
		),
		ConflictRangeType::Read,
	)
}

fn hash_position_key(
	namespace_id: Id,
	pool_name: &str,
	version: u32,
	hash_pos: [u8; 16],
) -> Vec<u8> {
	keys::subspace().pack(&(
		NAMESPACE,
		ENVOY_HASH_IDX,
		namespace_id,
		pool_name,
		-(version as i32),
		&hash_pos[..],
	))
}
