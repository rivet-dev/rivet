use std::str::FromStr;

use serde_json::json;

use super::TestDatacenter;

// Namespace helpers
pub async fn setup_test_namespace(leader_dc: &TestDatacenter) -> (String, rivet_util::Id) {
	let random_suffix = rand::random::<u16>();
	let namespace_name = format!("test-{random_suffix}");
	let res = super::api::public::namespaces_create(
		leader_dc.guard_port(),
		rivet_api_peer::namespaces::CreateRequest {
			name: namespace_name,
			display_name: "Test Namespace".to_string(),
		},
	)
	.await
	.expect("failed to setup test namespace");
	(res.namespace.name, res.namespace.namespace_id)
}

// Setup namespace with runner
pub async fn setup_test_namespace_with_runner(
	dc: &super::TestDatacenter,
) -> (String, rivet_util::Id, super::runner::TestRunner) {
	let (namespace_name, namespace_id) = setup_test_namespace(dc).await;

	let runner = setup_runner(
		dc,
		&namespace_name,
		&format!("key-{:012x}", rand::random::<u64>()),
		1,
		20,
		None,
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
	runner_name: Option<String>,
) -> super::runner::TestRunner {
	super::runner::TestRunner::new(
		dc.guard_port(),
		&namespace_name,
		key,
		version,
		total_slots,
		runner_name,
	)
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

// Actor verification helpers
pub async fn assert_actor_exists(
	port: u16,
	actor_id: &str,
	namespace: &str,
) -> rivet_types::actors::Actor {
	let res = super::try_get_actor(port, actor_id, namespace).await;
	let Some(actor) = res.expect("Failed to try_get_actor") else {
		panic!("Actor {} should exist in namespace {}", actor_id, namespace,);
	};
	actor
}

pub async fn assert_actor_not_exists(port: u16, actor_id: &str, namespace: &str) {
	let res = super::try_get_actor(port, actor_id, namespace).await;
	if let Some(actor) = res.expect("Failed to try_get_actor") {
		panic!(
			"Actor {} should not exist in namespace {}",
			actor_id, namespace,
		);
	};
}

pub async fn assert_actor_is_destroyed(port: u16, actor_id: &str, namespace: &str) {
	let actor = assert_actor_exists(port, actor_id, namespace).await;
	assert!(
		actor.destroy_ts.is_some(),
		"Actor {} should have destroy_ts set",
		actor_id
	);
}

pub async fn assert_actor_is_alive(port: u16, actor_id: &str, namespace: &str) {
	let actor = assert_actor_exists(port, actor_id, namespace).await;
	assert!(
		actor.destroy_ts.is_none(),
		"Actor {} should not have destroy_ts set",
		actor_id
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
		.expect("actor::get_runners operation failed");
	let runner_id = actors_res.actors.first().map(|x| x.runner_id.to_string());

	assert_eq!(
		runner_id,
		Some(expected_runner_id.to_string()),
		"Actor {actor_id} should be in runner {expected_runner_id} (actually in runner {runner_id:?})",
	);
}

pub fn assert_actors_equal(
	actor1: &rivet_types::actors::Actor,
	actor2: &rivet_types::actors::Actor,
) {
	assert_eq!(actor1.actor_id, actor2.actor_id, "Actor IDs should match");
	assert_eq!(
		actor1.namespace_id, actor2.namespace_id,
		"Namespace IDs should match"
	);
	assert_eq!(actor1.name, actor2.name, "Actor names should match");
}

// Datacenter helpers
pub fn get_test_datacenter_name(label: u16) -> String {
	format!("dc-{}", label)
}

pub async fn setup_multi_datacenter_test() -> super::TestCtx {
	super::TestCtx::new_multi(2)
		.await
		.expect("Failed to setup multi-datacenter test")
}

pub fn convert_strings_to_ids(actor_ids: Vec<String>) -> Vec<rivet_util::Id> {
	actor_ids
		.iter()
		.map(|x| rivet_util::Id::from_str(&x).expect("failed to convert actor ids to string"))
		.collect::<Vec<_>>()
}
