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
) -> (String, rivet_util::Id, super::test_runner::TestRunner) {
	let (namespace_name, namespace_id) = setup_test_namespace(dc).await;

	let runner = setup_runner(dc, &namespace_name, |builder| {
		builder.with_actor_behavior("test-actor", |_config| {
			Box::new(super::test_runner::EchoActor::new())
		})
	})
	.await;

	(namespace_name, namespace_id, runner)
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

	// TODO: make this fetch as well
	assert_eq!(
		actual_dc_label, expected_dc_label,
		"Actor should be in datacenter {} but is in {}",
		expected_dc_label, actual_dc_label
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

// Timing helpers for eventual consistency
pub async fn wait_for_eventual_consistency() {
	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

pub async fn wait_for_actor_propagation(actor_id: &str, _generation: u32) {
	// Wait for actor state to propagate through all systems
	tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
}

// Test runner helper functions

/// Build a test runner with specified configuration
///
/// Defaults to 20 total slots, but can be overridden in the builder closure.
///
/// Example:
/// ```
/// // Default 20 slots
/// let runner = setup_runner(ctx.leader_dc(), &namespace, |builder| {
///     builder
///         .with_actor_behavior("test-actor", |_| Box::new(EchoActor::new()))
///         .with_actor_behavior("crash-actor", |_| Box::new(CrashOnStartActor::new(1)))
/// }).await;
///
/// // Override slots
/// let runner = setup_runner(ctx.leader_dc(), &namespace, |builder| {
///     builder
///         .with_total_slots(2)
///         .with_actor_behavior("test-actor", |_| Box::new(EchoActor::new()))
/// }).await;
/// ```
pub async fn setup_runner<F>(
	dc: &super::TestDatacenter,
	namespace: &str,
	configure: F,
) -> super::test_runner::TestRunner
where
	F: FnOnce(super::test_runner::TestRunnerBuilder) -> super::test_runner::TestRunnerBuilder,
{
	let builder = super::test_runner::TestRunnerBuilder::new(namespace)
		.with_runner_key(&format!("key-{:012x}", rand::random::<u64>()))
		.with_version(1)
		.with_total_slots(20);

	let builder = configure(builder);

	let runner = builder
		.build(dc)
		.await
		.expect("failed to build test runner");

	runner.start().await.expect("failed to start runner");
	runner.wait_ready().await;

	runner
}

pub fn convert_strings_to_ids(actor_ids: Vec<String>) -> Vec<rivet_util::Id> {
	actor_ids
		.iter()
		.map(|x| rivet_util::Id::from_str(&x).expect("failed to convert actor ids to string"))
		.collect::<Vec<_>>()
}

pub async fn create_actor(
	port: u16,
	namespace: &str,
	name: &str,
	runner_name: &str,
	crash_policy: rivet_types::actors::CrashPolicy,
) -> super::api_types::actors::create::CreateResponse {
	super::api::public::actors_create(
		port,
		super::api_types::actors::create::CreateQuery {
			namespace: namespace.to_string(),
		},
		super::api_types::actors::create::CreateRequest {
			datacenter: None,
			name: name.to_string(),
			key: None,
			input: None,
			runner_name_selector: runner_name.to_string(),
			crash_policy,
		},
	)
	.await
	.expect("failed to create actor")
}

pub fn generate_dummy_rivet_id(dc: &super::TestDatacenter) -> rivet_util::Id {
	rivet_util::Id::new_v1(dc.config.dc_label())
}
