use super::*;

mod moved_tests {
	use super::*;

	#[test]
	fn reset_runtime_state_clears_end_reason_without_touching_core_lifecycle_flags() {
		let shared = ActorContextShared::default();
		let ctx = rivetkit_core::testing::actor_context("actor-test", "actor", Vec::new(), "local");

		ctx.set_started(true);
		shared.set_end_reason(EndReason::Sleep);
		assert!(shared.has_end_reason());
		assert!(ctx.started());

		shared.reset_runtime_state();

		assert!(!shared.has_end_reason());
		assert!(ctx.started());
	}

	#[test]
	fn generations_of_the_same_actor_do_not_share_runtime_state() {
		let harness = rivetkit_core::testing::ActorContextHarness::new();
		let context_for = |generation| {
			ActorContext::new(harness.context_with_generation(
				"actor-generations",
				"actor",
				Vec::new(),
				"local",
				rivetkit_core::ActorConfig::default(),
				generation,
			))
		};

		let old = context_for(1);
		let old_again = context_for(1);
		let new = context_for(2);

		assert!(Arc::ptr_eq(&old.shared, &old_again.shared));
		assert!(!Arc::ptr_eq(&old.shared, &new.shared));

		// An older generation that finishes shutting down after a newer generation started must
		// not reset the newer generation's runtime state.
		new.shared.set_end_reason(EndReason::Sleep);
		old.shared.reset_runtime_state();
		assert!(new.shared.has_end_reason());
	}
}
