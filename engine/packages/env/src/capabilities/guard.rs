//! Wraps `std::ops::Fn` predicates as authority checks for guarded runtime mutations.

/// Rechecks current authority immediately before a side effect, including after preparatory I/O.
/// Implementations read live authority; a successful check does not extend a lease or prove completion.
pub trait MutationGuard: Send + Sync {
	/// Allows dispatch only while the operation and its owner's authority remain current.
	fn may_send(&self) -> bool;
}

impl<F: Fn() -> bool + Send + Sync> MutationGuard for F {
	fn may_send(&self) -> bool {
		self()
	}
}
