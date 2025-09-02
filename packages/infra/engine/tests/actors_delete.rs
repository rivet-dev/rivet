mod common;

use std::time::Duration;

// MARK: Basic
#[test]
fn delete_existing_actor_with_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		common::destroy_actor(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		common::wait_for_eventual_consistency().await;

		common::assert_actor_is_destroyed(
			&actor_id,
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;
	});
}

#[test]
fn delete_existing_actor_without_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		let response =
			common::destroy_actor_without_namespace(&actor_id, ctx.leader_dc().guard_port()).await;
		common::assert_success_response(&response);

		common::wait_for_eventual_consistency().await;

		common::assert_actor_is_destroyed(
			&actor_id,
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;
	});
}

#[test]
fn delete_actor_current_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		common::destroy_actor(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		common::wait_for_eventual_consistency().await;

		common::assert_actor_is_destroyed(
			&actor_id,
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;
	});
}

#[test]
fn delete_actor_remote_datacenter() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		common::wait_for_actor_propagation(&actor_id, 1).await;

		common::destroy_actor(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		common::wait_for_eventual_consistency().await;

		common::assert_actor_is_destroyed(&actor_id, Some(&namespace), ctx.get_dc(2).guard_port())
			.await;
	});
}

// MARK: Error cases

#[test]
fn delete_non_existent_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (_namespace, _, runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let fake_actor_id = format!("00000000-0000-0000-0000-{:012x}", rand::random::<u64>());
		let response =
			common::destroy_actor_without_namespace(&fake_actor_id, ctx.leader_dc().guard_port())
				.await;

		assert_eq!(
			response.status(),
			400,
			"Should return 400 for non-existent actor"
		);
	});
}

#[test]
fn delete_actor_wrong_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace1, _, runner1) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let (namespace2, _, runner2) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace1, ctx.leader_dc().guard_port()).await;

		let client = reqwest::Client::new();
		let response = client
			.delete(&format!(
				"http://127.0.0.1:{}/actors/{}?namespace={}",
				ctx.leader_dc().guard_port(),
				actor_id,
				namespace2
			))
			.send()
			.await
			.expect("Failed to send delete request");

		assert!(
			!response.status().is_success(),
			"Should fail to delete actor with wrong namespace"
		);

		common::assert_actor_exists(&actor_id, &namespace1, ctx.leader_dc().guard_port()).await;
	});
}

#[test]
fn delete_with_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		let client = reqwest::Client::new();
		let response = client
			.delete(&format!(
				"http://127.0.0.1:{}/actors/{}?namespace=non-existent-namespace",
				ctx.leader_dc().guard_port(),
				actor_id
			))
			.send()
			.await
			.expect("Failed to send delete request");

		assert!(
			!response.status().is_success(),
			"Should fail with non-existent namespace"
		);
	});
}

#[test]
fn delete_invalid_actor_id_format() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let client = reqwest::Client::new();
		let response = client
			.delete(&format!(
				"http://127.0.0.1:{}/actors/invalid-uuid?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.send()
			.await
			.expect("Failed to send delete request");

		assert_eq!(
			response.status(),
			400,
			"Should return 400 for invalid actor ID format"
		);
	});
}

// MARK: Cross-datacenter tests

#[test]
fn delete_remote_actor_verify_propagation() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				datacenter: Some("dc-2".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		common::wait_for_actor_propagation(&actor_id, 1).await;

		common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		common::assert_actor_exists(&actor_id, &namespace, ctx.get_dc(2).guard_port()).await;

		common::destroy_actor(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		tokio::time::sleep(Duration::from_millis(500)).await;

		common::assert_actor_is_destroyed(
			&actor_id,
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_actor_is_destroyed(&actor_id, Some(&namespace), ctx.get_dc(2).guard_port())
			.await;
	});
}
