use prometheus::core::{Atomic, AtomicF64, AtomicI64, GenericGauge, Number};

/// Guard returned by `IntGauge` and `IntGaugeVec`.
pub type IntGaugeGuard = GaugeGuard<AtomicI64>;

/// Guard returned by `Gauge` and `GaugeVec`.
pub type FloatGaugeGuard = GaugeGuard<AtomicF64>;

/// Holds an increment on a gauge and releases it when dropped. Use it for gauges that count work
/// currently in progress so the decrement cannot be lost to an early return, a `?`, a cancelled
/// future, or a panic.
///
/// ```ignore
/// let _guard = REQUEST_PENDING.with_label_values(&[router]).inc_guard();
/// ```
#[must_use = "the gauge is decremented as soon as the guard is dropped"]
pub struct GaugeGuard<P: Atomic> {
	gauge: GenericGauge<P>,
	amount: P::T,
}

impl<P: Atomic> std::fmt::Debug for GaugeGuard<P>
where
	P::T: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GaugeGuard")
			.field("amount", &self.amount)
			.finish()
	}
}

impl<P: Atomic> GaugeGuard<P> {
	/// Releases the increment now instead of at the end of the scope.
	pub fn release(self) {}
}

impl<P: Atomic> Drop for GaugeGuard<P> {
	fn drop(&mut self) {
		self.gauge.sub(self.amount);
	}
}

/// Adds RAII increments to every gauge type, including the gauges returned by
/// `GenericGaugeVec::with_label_values`.
pub trait GaugeGuardExt<P: Atomic> {
	/// Increments the gauge by one and returns a guard that decrements it when dropped.
	fn inc_guard(&self) -> GaugeGuard<P>;

	/// Increments the gauge by `amount` and returns a guard that subtracts the same amount when
	/// dropped.
	fn add_guard(&self, amount: P::T) -> GaugeGuard<P>;
}

impl<P: Atomic> GaugeGuardExt<P> for GenericGauge<P> {
	fn inc_guard(&self) -> GaugeGuard<P> {
		self.add_guard(P::T::from_i64(1))
	}

	fn add_guard(&self, amount: P::T) -> GaugeGuard<P> {
		self.add(amount);

		GaugeGuard {
			gauge: self.clone(),
			amount,
		}
	}
}
