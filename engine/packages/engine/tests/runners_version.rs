mod common;

#[test]
fn runner_version_upgrade() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc().guard_port()).await;
		let _runner1 = common::setup_runner(ctx.leader_dc(), &namespace, "key-1", 1, 1).await;
		let runner2 = common::setup_runner(ctx.leader_dc(), &namespace, "key-2", 2, 2).await;

		// Create actor to fill a single slot. This forces the second allocation to either be in the other
		// runner, or in the same runner IF runner versions are implemented correctly.
		let actor_id1 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: "actor1".to_string(),
				key: None,
				datacenter: None,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id1.is_empty(), "Actor ID should not be empty");

		common::assert_actor_exists(&actor_id1, &namespace, ctx.leader_dc().guard_port()).await;

		common::assert_actor_in_runner(ctx.leader_dc(), &actor_id1, &runner2.runner_id.to_string())
			.await;

		let actor_id2 = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: "actor2".to_string(),
				key: None,
				datacenter: None,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id2.is_empty(), "Actor ID should not be empty");

		common::assert_actor_exists(&actor_id2, &namespace, ctx.leader_dc().guard_port()).await;

		common::assert_actor_in_runner(ctx.leader_dc(), &actor_id2, &runner2.runner_id.to_string())
			.await;
	});
}
