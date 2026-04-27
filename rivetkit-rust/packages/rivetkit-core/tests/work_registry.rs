use super::*;

mod moved_tests {
	use std::panic::{AssertUnwindSafe, catch_unwind};

	use super::WorkRegistry;

	#[test]
	fn region_guard_drop_decrements_counter() {
		let work = WorkRegistry::new();
		assert_eq!(work.keep_awake.load(), 0);

		{
			let _guard = work.keep_awake_guard();
			assert_eq!(work.keep_awake.load(), 1);
		}

		assert_eq!(work.keep_awake.load(), 0);
	}

	#[test]
	fn region_guard_drop_during_panic_unwind_decrements_counter() {
		let work = WorkRegistry::new();

		let result = catch_unwind(AssertUnwindSafe(|| {
			let _guard = work.keep_awake_guard();
			assert_eq!(work.keep_awake.load(), 1);
			panic!("boom");
		}));

		assert!(
			result.is_err(),
			"panic should propagate through catch_unwind"
		);
		assert_eq!(work.keep_awake.load(), 0);
	}
}
