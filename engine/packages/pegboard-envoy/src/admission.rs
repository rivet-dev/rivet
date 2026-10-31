//! Admission of Envoy connects. Every connect costs guard a WebSocket upgrade and pegboard-envoy
//! the work of registering an Envoy, so a fleet that lost its engine together must not arrive
//! all at once. A process-wide token bucket admits connects at a configured rate after a burst.
//! A connect it cannot admit is refused before any connection state is built, with a retry hint,
//! and the Envoy, which owns the retry, waits between one and two hints before trying again. A
//! refused connect still costs its TLS handshake and upgrade.
//!
//! The budget is process-wide, so one namespace's reconnect storm can shed another's runners.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Bucket arithmetic runs in thousandths of a token, so a fractional refill per millisecond is
/// exact.
const MILLI: u64 = 1_000;

/// The connect budget's limits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ConnectBudget {
	pub admissions_per_second: u32,
	pub burst: u32,
	pub retry_after_ms: u64,
}

struct Bucket {
	/// Tokens available, in thousandths.
	tokens_milli: u64,
	/// Boot time of the last refill.
	refilled_at_ms: u64,
}

pub(crate) struct ConnectAdmission {
	budget: ConnectBudget,
	/// Guarded by a sync lock because admission is a short, sync step taken before any await.
	bucket: Mutex<Bucket>,
	/// Whether a shedding episode is in progress, so its warning is logged once. An episode ends
	/// once the bucket has refilled to half its burst, not at the first admitted connect, which
	/// under sustained overload would follow every shed connect.
	shedding: AtomicBool,
}

impl ConnectAdmission {
	pub(crate) fn new(budget: ConnectBudget) -> Self {
		Self {
			budget,
			bucket: Mutex::new(Bucket {
				tokens_milli: u64::from(budget.burst) * MILLI,
				refilled_at_ms: 0,
			}),
			shedding: AtomicBool::new(false),
		}
	}

	/// Admits one connect at `now_ms` (boot time), or returns the retry hint in milliseconds
	/// when the budget is spent.
	pub(crate) fn try_admit(&self, now_ms: u64) -> Result<(), u64> {
		let (admitted, recovered) = {
			let mut bucket = self.bucket.lock();
			let elapsed_ms = now_ms.saturating_sub(bucket.refilled_at_ms);
			// `admissions_per_second` tokens per 1000 ms is that many thousandths per millisecond.
			let refill = elapsed_ms.saturating_mul(u64::from(self.budget.admissions_per_second));
			let capacity = u64::from(self.budget.burst) * MILLI;
			bucket.tokens_milli = bucket.tokens_milli.saturating_add(refill).min(capacity);
			bucket.refilled_at_ms = bucket.refilled_at_ms.max(now_ms);
			let admitted = if bucket.tokens_milli >= MILLI {
				bucket.tokens_milli -= MILLI;
				true
			} else {
				false
			};
			(admitted, bucket.tokens_milli * 2 >= capacity)
		};

		if admitted {
			if recovered && self.shedding.swap(false, Ordering::Relaxed) {
				tracing::info!("envoy connects admitted again after shedding");
			}
			Ok(())
		} else {
			if !self.shedding.swap(true, Ordering::Relaxed) {
				tracing::warn!(
					admissions_per_second = self.budget.admissions_per_second,
					burst = self.budget.burst,
					retry_after_ms = self.budget.retry_after_ms,
					"shedding envoy connects over the connect budget; a fleet is likely reconnecting after an outage, or the budget is too low for this process's share of runners"
				);
			}
			Err(self.budget.retry_after_ms)
		}
	}
}



/// Per-connection admission for retained actor lanes. A handle holds its slot until the last
/// reference disappears, including references held by commands that already found the handle.
pub(crate) struct ActorHandleBudget {
	permits: std::sync::Arc<tokio::sync::Semaphore>,
	capacity: usize,
	pressure: std::sync::Arc<crate::queue::QueuePressure>,
}

pub(crate) struct ActorHandlePermit {
	_slot: tokio::sync::OwnedSemaphorePermit,
	_usage: rivet_metrics::IntGaugeGuard,
}

static ACTOR_HANDLES: std::sync::LazyLock<rivet_metrics::prometheus::IntGauge> =
	std::sync::LazyLock::new(|| {
		let gauge = rivet_metrics::prometheus::IntGauge::new(
			"pegboard_node_actor_handles",
			"Retained actor handles across node connections.",
		)
		.unwrap();
		rivet_metrics::REGISTRY
			.register(Box::new(gauge.clone()))
			.unwrap();
		gauge
	});
static ACTOR_HANDLE_CAPACITY: std::sync::LazyLock<rivet_metrics::prometheus::IntGauge> =
	std::sync::LazyLock::new(|| {
		let gauge = rivet_metrics::prometheus::IntGauge::new(
			"pegboard_node_actor_handles_capacity",
			"Configured actor handle cap per node connection.",
		)
		.unwrap();
		rivet_metrics::REGISTRY
			.register(Box::new(gauge.clone()))
			.unwrap();
		gauge
	});

impl ActorHandleBudget {
	pub(crate) fn new(
		capacity: usize,
		pressure: std::sync::Arc<crate::queue::QueuePressure>,
	) -> Self {
		ACTOR_HANDLE_CAPACITY.set(capacity as i64);
		Self {
			permits: std::sync::Arc::new(tokio::sync::Semaphore::new(capacity)),
			capacity,
			pressure,
		}
	}

	pub(crate) fn try_admit(&self, now_boot_ms: u64) -> anyhow::Result<ActorHandlePermit> {
		let slot = self
			.permits
			.clone()
			.try_acquire_owned()
			.map_err(|_| universalpubsub::errors::Overloaded.build())?;
		let in_use = self
			.capacity
			.saturating_sub(self.permits.available_permits());
		if crate::queue::near_capacity(in_use, self.capacity)
			&& self.pressure.claim_warning(now_boot_ms)
		{
			tracing::warn!(
				in_use,
				capacity = self.capacity,
				config_key = "pegboard.node_actor_handles_capacity",
				"envoy actor handles are near capacity"
			);
		}
		Ok(ActorHandlePermit {
			_slot: slot,
			_usage: rivet_metrics::GaugeGuardExt::inc_guard(&*ACTOR_HANDLES),
		})
	}
}
