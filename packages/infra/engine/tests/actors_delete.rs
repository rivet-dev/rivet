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

		common::assert_actor_is_destroyed(
			&actor_id,
			Some(&namespace),
			ctx.leader_dc().guard_port(),
		)
		.await;
	});
}

#[test]
fn delete_existing_actor_without_namespace_should_succeed() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		let response =
			common::destroy_actor_without_namespace(&actor_id, ctx.leader_dc().guard_port()).await;
		assert_eq!(
			response.status(),
			200,
			"Should return 200 for unprovided namespace parameter"
		);

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
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
		)
		.await;

		common::wait_for_actor_propagation(&actor_id, 1).await;

		common::destroy_actor(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		common::wait_for_eventual_consistency().await;

		common::assert_actor_is_destroyed(&actor_id, Some(&namespace), ctx.get_dc(2).guard_port())
			.await;
	});
}

// MARK: Namespace validation
#[test]
fn delete_actor_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let non_existent_ns = "non-existent-namespace";
		let api_port = ctx.leader_dc().guard_port();
		let client = reqwest::Client::new();

		// DELETE /actors/{id}
		let response = client
			.delete(&format!(
				"http://127.0.0.1:{}/actors/00000000-0000-0000-0000-000000000000?namespace={}",
				api_port, non_existent_ns
			))
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"DELETE /actors/{{id}} should fail with non-existent namespace"
		);
	});
}

#[test]
fn delete_actor_invalid_id_format() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let api_port = ctx.leader_dc().guard_port();

		// Test various invalid actor ID formats
		let invalid_ids = vec![
			"not-a-uuid",
			"12345",
			"00000000-0000-0000-0000",               // Incomplete UUID
			"00000000-0000-0000-0000-000000000000g", // Invalid character
			"00000000_0000_0000_0000_000000000000",  // Wrong separator
		];

		for invalid_id in invalid_ids {
			// DELETE /actors/{id}
			let client = reqwest::Client::new();
			let response = client
				.delete(&format!(
					"http://127.0.0.1:{}/actors/{}?namespace={}",
					api_port, invalid_id, namespace
				))
				.send()
				.await
				.expect("Failed to send request");
			assert_eq!(
				response.status(),
				400,
				"DELETE should return 400 for invalid actor ID: {}",
				invalid_id
			);
		}

		// DELETE with empty ID also returns 404
		let client = reqwest::Client::new();
		let response = client
			.delete(&format!(
				"http://127.0.0.1:{}/actors/?namespace={}",
				api_port, namespace
			))
			.send()
			.await
			.expect("Failed to send request");
		assert_eq!(
			response.status(),
			404,
			"DELETE should return 404 for empty actor ID (route not found)"
		);
	});
}

// MARK: Error cases

#[test]
fn delete_non_existent_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (_namespace, _, _runner) =
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
		let (namespace1, _, _runner1) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let (namespace2, _, _runner2) =
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


// MARK: Cross-datacenter tests

#[test]
fn delete_remote_actor_verify_propagation() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				..Default::default()
			},
			ctx.get_dc(2).guard_port(),
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
