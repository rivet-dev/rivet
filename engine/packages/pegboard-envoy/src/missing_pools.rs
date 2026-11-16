//! Connect refusals for node pools that do not exist, tracked per pool name. A node connecting
//! before its pool is written is an expected race and logs at debug; refusals of one pool name
//! that persist past [`WARN_AFTER_MS`] take one warning, so a node configured with a pool that
//! never appears does not retry silently forever. A run ends only after [`RUN_ENDS_AFTER_MS`]
//! without a refusal, longer than the warning threshold, because a node's retries spread across
//! every engine process behind the load balancer, so one process can see gaps of over a minute
//! between refusals of a name that is refused the whole time.

use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::sync::atomic::{AtomicBool, Ordering};

/// How long refusals of one pool name must persist before they warn.
pub(crate) const WARN_AFTER_MS: u64 = 60_000;

/// How long without a refusal ends a pool name's run of refusals.
pub(crate) const RUN_ENDS_AFTER_MS: u64 = 5 * 60_000;

static MISSING_POOLS: std::sync::LazyLock<rivet_metrics::prometheus::IntGauge> =
	std::sync::LazyLock::new(|| {
		let gauge = rivet_metrics::prometheus::IntGauge::new(
			"pegboard_node_missing_pools",
			"Pool names this process refused node connects for, tracked for the sustained refusal warning.",
		)
		.unwrap();
		rivet_metrics::REGISTRY
			.register(Box::new(gauge.clone()))
			.unwrap();
		gauge
	});

static MISSING_POOLS_CAPACITY: std::sync::LazyLock<rivet_metrics::prometheus::IntGauge> =
	std::sync::LazyLock::new(|| {
		let gauge = rivet_metrics::prometheus::IntGauge::new(
			"pegboard_node_missing_pools_capacity",
			"Configured cap on the pool names one process tracks for refused node connects.",
		)
		.unwrap();
		rivet_metrics::REGISTRY
			.register(Box::new(gauge.clone()))
			.unwrap();
		gauge
	});

/// The pool names this process refused node connects for, keyed by namespace and pool name,
/// holding at most `capacity` names.
pub(crate) struct MissingPools {
	capacity: usize,
	refusals: scc::HashMap<(String, String), Refusals>,
	/// Whether the at-capacity warning fired since a name was last tracked.
	full_warned: AtomicBool,
}

/// One pool name's current run of refusals.
struct Refusals {
	since_boot_ms: u64,
	last_boot_ms: u64,
	count: u64,
	warned: bool,
	_tracked: IntGaugeGuard,
}

/// How a refused connect is logged.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
	/// The expected race of a node connecting before its pool is written: logged at debug.
	Expected,
	/// The pool name has been refused for at least [`WARN_AFTER_MS`]: warned once per run.
	Sustained { count: u64, refused_for_ms: u64 },
}

impl MissingPools {
	pub(crate) fn new(capacity: usize) -> Self {
		let capacity = capacity.max(1);
		MISSING_POOLS_CAPACITY.set(capacity as i64);
		Self {
			capacity,
			refusals: scc::HashMap::new(),
			full_warned: AtomicBool::new(false),
		}
	}

	/// Records a refused connect for `pool_name` at `now_boot_ms`. Answers `Sustained` once per
	/// run of refusals: on the first refusal at least [`WARN_AFTER_MS`] after the run's first. A
	/// run ends after [`RUN_ENDS_AFTER_MS`] without a refusal, or when the pool admits a connection.
	/// A new name that finds every slot held by a live run is not tracked.
	pub(crate) async fn refused(
		&self,
		namespace: &str,
		pool_name: &str,
		now_boot_ms: u64,
	) -> Refusal {
		let key = (namespace.to_owned(), pool_name.to_owned());
		if self.refusals.len() >= self.capacity && !self.refusals.contains_async(&key).await {
			self.refusals
				.retain_async(|_, refusals| {
					now_boot_ms.saturating_sub(refusals.last_boot_ms) < RUN_ENDS_AFTER_MS
				})
				.await;
			if self.refusals.len() >= self.capacity {
				if !self.full_warned.swap(true, Ordering::AcqRel) {
					tracing::warn!(
						capacity = self.capacity,
						"node connects are refused for more missing pools than this process tracks; raise pegboard.node_missing_pools_capacity, or check the nodes' pool names"
					);
				}
				return Refusal::Expected;
			}
		}

		match self.refusals.entry_async(key).await {
			scc::hash_map::Entry::Occupied(mut entry) => {
				let refusals = entry.get_mut();
				if now_boot_ms.saturating_sub(refusals.last_boot_ms) >= RUN_ENDS_AFTER_MS {
					refusals.since_boot_ms = now_boot_ms;
					refusals.count = 0;
					refusals.warned = false;
				}
				refusals.count += 1;
				refusals.last_boot_ms = now_boot_ms;
				let refused_for_ms = now_boot_ms.saturating_sub(refusals.since_boot_ms);
				if refusals.warned || refused_for_ms < WARN_AFTER_MS {
					return Refusal::Expected;
				}
				refusals.warned = true;
				Refusal::Sustained {
					count: refusals.count,
					refused_for_ms,
				}
			}
			scc::hash_map::Entry::Vacant(entry) => {
				self.full_warned.store(false, Ordering::Release);
				entry.insert_entry(Refusals {
					since_boot_ms: now_boot_ms,
					last_boot_ms: now_boot_ms,
					count: 1,
					warned: false,
					_tracked: MISSING_POOLS.inc_guard(),
				});
				Refusal::Expected
			}
		}
	}

	/// Ends `pool_name`'s run of refusals: the pool admitted a connection.
	pub(crate) async fn admitted(&self, namespace: &str, pool_name: &str) {
		self.refusals
			.remove_async(&(namespace.to_owned(), pool_name.to_owned()))
			.await;
	}
}
