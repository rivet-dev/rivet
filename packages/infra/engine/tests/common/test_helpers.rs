use std::time::Duration;

use serde_json::json;

// Namespace helpers
pub async fn setup_test_namespace(guard_port: u16) -> (String, rivet_util::Id) {
	let random_suffix = rand::random::<u16>();
	let namespace_name = format!("test-{random_suffix}");
	let namespace_id = super::create_namespace(&namespace_name, guard_port).await;
	(namespace_name, namespace_id)
}

// Setup namespace with runner
pub async fn setup_test_namespace_with_runner(
	dc: &super::TestDatacenter,
) -> (String, rivet_util::Id, super::runner::TestRunner) {
	let (namespace_name, namespace_id) = setup_test_namespace(dc.guard_port()).await;

	let runner = super::runner::TestRunner::new(
		dc.guard_port(),
		&namespace_name,
		&format!("key-{:012x}", rand::random::<u64>()),
		1,
		20,
	)
	.await;

	(namespace_name, namespace_id, runner)
}

pub async fn setup_runner(
	dc: &super::TestDatacenter,
	namespace_name: &str,
	key: &str,
	version: u32,
	total_slots: u32,
) -> super::runner::TestRunner {
	super::runner::TestRunner::new(dc.guard_port(), &namespace_name, key, version, total_slots)
		.await
}

pub async fn cleanup_test_namespace(namespace_id: rivet_util::Id, _guard_port: u16) {
	// TODO: implement namespace deletion when available
	tracing::info!(?namespace_id, "namespace cleanup (not implemented)");
}

// Data generation helpers
pub fn generate_test_input_data() -> String {
	base64::Engine::encode(
		&base64::engine::general_purpose::STANDARD,
		json!({
			"test": true,
			"timestamp": chrono::Utc::now().timestamp_millis(),
			"data": "test input data"
		})
		.to_string(),
	)
}

pub fn generate_large_input_data(size_mb: usize) -> String {
	let size_bytes = size_mb * 1024 * 1024;
	let data = "x".repeat(size_bytes);
	base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data)
}

pub fn generate_unicode_string() -> String {
	"test-🦀-🚀-✨-你好-世界".to_string()
}

pub fn generate_special_chars_string() -> String {
	"test-!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string()
}

// Wait helpers
pub async fn wait_for_actor_propagation(actor_id: &str, timeout_secs: u64) {
	tracing::info!(?actor_id, ?timeout_secs, "waiting for actor propagation");
	tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
}

pub async fn wait_for_eventual_consistency() {
	tracing::info!("waiting for eventual consistency");
	tokio::time::sleep(Duration::from_millis(500)).await;
}

// Actor verification helpers
pub async fn assert_actor_exists(
	actor_id: &str,
	namespace: &str,
	guard_port: u16,
) -> serde_json::Value {
	let response = super::get_actor(actor_id, Some(namespace), guard_port).await;
	assert!(
		response.status().is_success(),
		"Actor {} should exist in namespace {}",
		actor_id,
		namespace
	);
	response
		.json()
		.await
		.expect("Failed to parse actor response")
}

pub async fn assert_actor_not_exists(actor_id: &str, guard_port: u16) {
	let response = super::get_actor(actor_id, None, guard_port).await;
	assert_eq!(
		response.status(),
		400,
		"Actor {} should not exist (expecting 400 for Actor::NotFound)",
		actor_id
	);
}

pub async fn assert_actor_returns_bad_request(actor_id: &str, guard_port: u16) {
	let response = super::get_actor(actor_id, None, guard_port).await;
	assert_eq!(
		response.status(),
		400,
		"Actor {} should return 400 BAD_REQUEST",
		actor_id
	);
}

pub async fn assert_actor_is_destroyed(actor_id: &str, namespace: Option<&str>, guard_port: u16) {
	let response = super::get_actor(actor_id, namespace, guard_port).await;
	assert!(
		response.status().is_success(),
		"Actor {} should still be retrievable after destroy",
		actor_id
	);

	let body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse actor response");

	tracing::info!(?body, ?actor_id, "assert_actor_is_destroyed response body");

	assert!(
		body["actor"]["destroy_ts"].as_i64().is_some(),
		"Actor {} should have destroy_ts set. Response: {:?}",
		actor_id,
		body
	);
}

pub async fn assert_actor_in_dc(actor_id_str: &str, expected_dc_label: u16) {
	// Parse the actor ID to get the datacenter label
	let actor_id: rivet_util::Id = actor_id_str.parse().expect("Failed to parse actor ID");
	let actual_dc_label = actor_id.label();

	assert_eq!(
		actual_dc_label, expected_dc_label,
		"Actor should be in datacenter {} but is in {}",
		expected_dc_label, actual_dc_label
	);
}

pub async fn assert_actor_in_runner(
	dc: &super::TestDatacenter,
	actor_id_str: &str,
	expected_runner_id: &str,
) {
	let actor_id: rivet_util::Id = actor_id_str.parse().expect("Failed to parse actor ID");

	let actors_res = dc
		.workflow_ctx
		.op(pegboard::ops::actor::get_runner::Input {
			actor_ids: vec![actor_id],
		})
		.await
		.unwrap();
	let runner_id = actors_res.actors.first().map(|x| x.runner_id.to_string());

	assert_eq!(
		runner_id,
		Some(expected_runner_id.to_string()),
		"Actor {actor_id} should be in runner {expected_runner_id} (actually in runner {runner_id:?})",
	);
}

pub fn assert_actors_equal(actor1: &serde_json::Value, actor2: &serde_json::Value) {
	assert_eq!(
		actor1["actor"]["actor_id"], actor2["actor"]["actor_id"],
		"Actor IDs should match"
	);
	assert_eq!(
		actor1["actor"]["namespace_id"], actor2["actor"]["namespace_id"],
		"Namespace IDs should match"
	);
	assert_eq!(
		actor1["actor"]["name"], actor2["actor"]["name"],
		"Actor names should match"
	);
}

// Response assertion helpers
pub async fn assert_created_response(response: &serde_json::Value, expected_created: bool) {
	let created = response["created"]
		.as_bool()
		.expect("Missing created field in response");
	assert_eq!(
		created, expected_created,
		"Expected created to be {}",
		expected_created
	);
}

pub fn assert_pagination_response(response: &serde_json::Value) {
	assert!(
		response.get("actors").is_some() || response.get("names").is_some(),
		"Response should have actors or names array"
	);
	// cursor is optional in pagination responses
}

// Datacenter helpers
pub fn get_test_datacenter_names(_ctx: &super::TestCtx) -> Vec<String> {
	vec!["dc-1".to_string(), "dc-2".to_string()] // Adjust based on actual DC names
}

pub async fn setup_multi_datacenter_test() -> super::TestCtx {
	super::TestCtx::new_multi(2)
		.await
		.expect("Failed to setup multi-datacenter test")
}
