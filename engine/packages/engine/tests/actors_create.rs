mod common;

use serde_json::json;

// MARK: Basic
#[test]
fn create_actor_valid_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor(&namespace, ctx.leader_dc().guard_port()).await;

		common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;

		assert!(
			runner.has_actor(&actor_id).await,
			"Runner should have the actor"
		);
	});
}

#[test]
fn create_actor_with_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let key = common::generate_unique_key();
		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				key: Some(key.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");

		// Verify actor exists
		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		assert_eq!(actor["actor"]["key"], json!(key));
	});
}

#[test]
fn create_actor_without_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				key: None,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");

		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		assert_eq!(actor["actor"]["key"], json!(null));
	});
}

#[test]
fn create_actor_with_input() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let input_data = common::generate_test_input_data();
		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				input: Some(input_data.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");
	});
}

#[test]
fn create_actor_without_input() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				input: None,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");
	});
}

#[test]
fn create_durable_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				durable: true,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");

		// Verify actor is durable
		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		assert_eq!(actor["actor"]["crash_policy"], "restart");
	});
}

#[test]
fn create_non_durable_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				durable: false,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");

		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		assert_eq!(actor["actor"]["crash_policy"], "destroy");
	});
}

#[test]
fn create_actor_specific_datacenter() {
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

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");

		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		let actor_id_str = actor["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 2).await;
	});
}

#[test]
fn create_actor_current_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				datacenter: None,
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");

		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		let actor_id_str = actor["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 1).await;
	});
}

// MARK: Error cases
#[test]
#[should_panic(expected = "Failed to create actor")]
fn create_actor_non_existent_namespace() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		common::create_actor("non-existent-namespace", ctx.leader_dc().guard_port()).await;
	});
}

#[test]
#[should_panic(expected = "Failed to create actor")]
fn create_actor_invalid_datacenter() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				datacenter: Some("invalid-dc".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;
	});
}

#[test]
fn create_actor_malformed_input() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				input: Some("not-valid-base64!@#$%".to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		assert!(!actor_id.is_empty(), "Actor ID should not be empty");
	});
}

// MARK: Cross-datacenter tests
#[test]
fn create_actor_remote_datacenter_verify() {
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

		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.get_dc(2).guard_port()).await;
		let actor_id_str = actor["actor"]["actor_id"]
			.as_str()
			.expect("Missing actor_id in actor");
		common::assert_actor_in_dc(&actor_id_str, 2).await;
	});
}

// MARK: Edge cases

#[test]
fn empty_strings_for_required_parameters() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;
		let client = reqwest::Client::new();

		// Empty name
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&json!({
				"name": "",
				"key": "key",
			}))
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"Should fail with empty name"
		);

		// Empty key in array
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&json!({
				"name": "test",
				"key": "",
			}))
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"Should fail with empty key"
		);

		// Empty namespace parameter
		let response = client
			.get(&format!(
				"http://127.0.0.1:{}/actors/by-id?namespace=&name=test&key=key",
				ctx.leader_dc().guard_port()
			))
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"Should fail with empty namespace"
		);
	});
}

#[test]
fn very_long_strings_for_names_and_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create very long name and key (should work up to reasonable limits)
		let long_name = "a".repeat(255); // 255 chars should be acceptable
		let long_key = "k".repeat(255);

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: long_name.clone(),
				key: Some(long_key.clone()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Verify actor was created
		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		assert_eq!(actor["actor"]["name"], long_name);
		assert_eq!(actor["actor"]["key"], long_key);

		// Try extremely long name (should fail)
		let too_long_name = "a".repeat(1000);
		let client = reqwest::Client::new();
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				ctx.leader_dc().guard_port(),
				namespace
			))
			.json(&json!({
				"name": too_long_name,
				"key": "key",
			}))
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"Should fail with extremely long name"
		);
	});
}

#[test]
fn special_characters_in_names_and_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor with special characters
		let special_name = common::generate_special_chars_string();
		let special_key = "key-!@#$%";

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				name: special_name.clone(),
				key: Some(special_key.to_string()),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Verify actor was created
		let actor =
			common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
		assert_eq!(actor["actor"]["name"], special_name);
		assert_eq!(actor["actor"]["key"], special_key);

		// Get actor by ID with special characters
		let response = common::get_actor_by_id(
			&namespace,
			&special_name,
			special_key,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);
		let body: serde_json::Value = response.json().await.expect("Failed to parse response");
		assert_eq!(body["actor_id"], actor_id);
	});
}

#[test]
fn unicode_characters_in_input_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create actor with unicode input data
		let unicode_data = base64::Engine::encode(
			&base64::engine::general_purpose::STANDARD,
			json!({
				"message": common::generate_unicode_string(),
				"emoji": "🦀🚀✨",
				"chinese": "你好世界",
				"arabic": "مرحبا بالعالم",
			})
			.to_string(),
		);

		let actor_id = common::create_actor_with_options(
			common::CreateActorOptions {
				namespace: namespace.clone(),
				input: Some(unicode_data),
				..Default::default()
			},
			ctx.leader_dc().guard_port(),
		)
		.await;

		// Verify actor was created successfully
		common::assert_actor_exists(&actor_id, &namespace, ctx.leader_dc().guard_port()).await;
	});
}

#[test]
fn maximum_limits_32_actor_ids_in_list() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _, _runner) =
			common::setup_test_namespace_with_runner(ctx.leader_dc()).await;

		// Create 33 actors
		let actor_ids =
			common::bulk_create_actors(&namespace, "limit-test", 33, ctx.leader_dc().guard_port())
				.await;

		// List with exactly 32 actor IDs (should work)
		let ids_32: Vec<String> = actor_ids.iter().take(32).cloned().collect();
		let response = common::list_actors(
			&namespace,
			None,
			None,
			Some(ids_32),
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		common::assert_success_response(&response);

		// List with 33 actor IDs (should fail)
		let response = common::list_actors(
			&namespace,
			None,
			None,
			Some(actor_ids.clone()),
			None,
			None,
			None,
			ctx.leader_dc().guard_port(),
		)
		.await;
		assert_eq!(
			response.status(),
			400,
			"Should fail with more than 32 actor IDs"
		);
	});
}
