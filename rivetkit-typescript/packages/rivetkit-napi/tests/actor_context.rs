use super::*;

mod moved_tests {
	use super::*;

	#[test]
	fn reset_runtime_state_clears_end_reason_without_touching_core_lifecycle_flags() {
		let shared = ActorContextShared::default();
		let ctx = CoreActorContext::new("actor-test", "actor", Vec::new(), "local");

		ctx.set_started(true);
		shared.set_end_reason(EndReason::Sleep);
		assert!(shared.has_end_reason());
		assert!(ctx.started());

		shared.reset_runtime_state();

		assert!(!shared.has_end_reason());
		assert!(ctx.started());
	}
}
