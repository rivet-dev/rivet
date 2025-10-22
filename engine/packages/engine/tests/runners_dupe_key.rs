mod common;

#[test]
fn runner_dupe_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc().guard_port()).await;

		let runner1 = common::setup_runner(ctx.leader_dc(), &namespace, "key-1", 1, 1).await;
		let runner2 = common::setup_runner(ctx.leader_dc(), &namespace, "key-1", 1, 1).await;

		let res = ctx
			.leader_dc()
			.workflow_ctx
			.op(pegboard::ops::runner::get::Input {
				runner_ids: vec![runner1.runner_id, runner2.runner_id],
			})
			.await
			.unwrap();
		let mut runners = res.runners.into_iter();

		let runner1 = runners.next().unwrap();
		let runner2 = runners.next().unwrap();

		assert!(runner1.drain_ts.is_some(), "runner1 not draining");
		assert!(runner2.drain_ts.is_none(), "runner2 is draining");
	});
}
