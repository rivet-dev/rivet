mod common;

use serde_json::json;

// MARK: Namespace Validation Tests

#[test]
fn all_endpoints_validate_namespace_exists() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let non_existent_ns = "non-existent-namespace";
		let api_port = ctx.leader_dc().guard_port();
		let client = reqwest::Client::new();

		// POST /actors
		let response = client
			.post(&format!(
				"http://127.0.0.1:{}/actors?namespace={}",
				api_port, non_existent_ns
			))
			.json(&json!({
				"name": "test",
				"key": "key",
			}))
			.send()
			.await
			.expect("Failed to send request");
		assert!(
			!response.status().is_success(),
			"POST /actors should fail with non-existent namespace"
		);

		// GET /actors/{id}
		let response = common::get_actor(
			"00000000-0000-0000-0000-000000000000",
			Some(non_existent_ns),
			api_port,
		)
		.await;
		assert!(
			!response.status().is_success(),
			"GET /actors/{{id}} should fail with non-existent namespace"
		);

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

		// GET /actors/by-id
		let response = common::get_actor_by_id(non_existent_ns, "test", "key", api_port).await;
		assert!(
			!response.status().is_success(),
			"GET /actors/by-id should fail with non-existent namespace"
		);

		// PUT /actors
		let response = common::get_or_create_actor(
			non_existent_ns,
			"test",
			Some("key".to_string()),
			false,
			None,
			None,
			api_port,
		)
		.await;
		assert!(
			!response.status().is_success(),
			"PUT /actors should fail with non-existent namespace"
		);

		// PUT /actors/by-id
		let response = common::get_or_create_actor_by_id(
			non_existent_ns,
			"test",
			Some("key".to_string()),
			None,
			api_port,
		)
		.await;
		assert!(
			!response.status().is_success(),
			"PUT /actors/by-id should fail with non-existent namespace"
		);

		// GET /actors (list)
		let response = common::list_actors(
			non_existent_ns,
			Some("test"),
			None,
			None,
			None,
			None,
			None,
			api_port,
		)
		.await;
		assert!(
			!response.status().is_success(),
			"GET /actors (list) should fail with non-existent namespace"
		);

		// GET /actors/names
		let response = common::list_actor_names(non_existent_ns, None, None, api_port).await;
		assert!(
			!response.status().is_success(),
			"GET /actors/names should fail with non-existent namespace"
		);
	});
}

// MARK: Actor ID Validation

#[test]
fn invalid_actor_id_formats() {
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
			// GET /actors/{id}
			let response = common::get_actor(invalid_id, Some(&namespace), api_port).await;
			assert_eq!(
				response.status(),
				400,
				"GET should return 400 for invalid actor ID: {}",
				invalid_id
			);

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

		// Special case: empty actor ID results in different route
		let response = common::get_actor("", Some(&namespace), api_port).await;
		assert_eq!(
			response.status(),
			404,
			"GET should return 404 for empty actor ID (route not found)"
		);

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
