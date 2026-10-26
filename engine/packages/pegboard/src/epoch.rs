//! Ring comparisons for actor epochs.
//!
//! An actor's epoch is a `u32` that only ever advances with `wrapping_add`, so it may wrap.
//! Two epochs compare by their distance around the ring: `a` is after `b` when going forward
//! from `b` reaches `a` in less than half the ring. Equality is exact. Every one-sided epoch
//! comparison in the engine must use these, never the integer operators, or an actor that
//! wrapped would have its newest epochs judged oldest.

/// Whether `a` is a later epoch than `b`.
pub fn wrapping_gt(a: u32, b: u32) -> bool {
	a != b && a.wrapping_sub(b) < u32::MAX / 2
}

/// Whether `a` is the same epoch as `b` or a later one.
pub fn wrapping_ge(a: u32, b: u32) -> bool {
	a == b || wrapping_gt(a, b)
}

/// Whether `a` is an earlier epoch than `b`.
pub fn wrapping_lt(a: u32, b: u32) -> bool {
	wrapping_gt(b, a)
}

/// Whether `a` is the same epoch as `b` or an earlier one.
pub fn wrapping_le(a: u32, b: u32) -> bool {
	a == b || wrapping_gt(b, a)
}

/// The later of two epochs.
pub fn wrapping_max(a: u32, b: u32) -> u32 {
	if wrapping_gt(b, a) { b } else { a }
}


