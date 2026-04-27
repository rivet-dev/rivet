use super::*;

mod moved_tests {
	use std::collections::HashMap;

	use tokio_util::sync::CancellationToken;

	use super::{
		CoreServerlessRuntime, ServerlessRequest, endpoints_match, normalize_endpoint_url,
	};
	use crate::registry::ServeConfig;

	#[test]
	fn normalizes_loopback_addresses() {
		assert_eq!(
			normalize_endpoint_url("http://127.0.0.1:6420/").as_deref(),
			Some("http://localhost:6420/")
		);
		assert!(endpoints_match(
			"http://0.0.0.0:6420/api/",
			"http://localhost:6420/api"
		));
	}

	#[test]
	fn normalizes_rivet_regional_hosts() {
		assert!(endpoints_match(
			"https://api-us-west-1.rivet.dev",
			"https://api.rivet.dev/"
		));
		assert!(endpoints_match(
			"https://api-lax.staging.rivet.dev",
			"https://api.staging.rivet.dev/"
		));
		assert!(!endpoints_match(
			"https://api-us-west-1.example.com",
			"https://api.example.com"
		));
	}

	#[test]
	fn invalid_urls_fall_back_to_string_comparison() {
		assert!(endpoints_match("not a url", "not a url"));
		assert!(!endpoints_match("not a url", "also not a url"));
	}

	#[tokio::test]
	async fn handles_basic_routes() {
		let runtime = test_runtime().await;

		let health = runtime
			.handle_request(test_request("GET", "/api/rivet/health"))
			.await;
		assert_eq!(health.status, 200);
		let health_body = read_body(health).await;
		assert_eq!(health_body["status"], "ok");
		assert_eq!(health_body["runtime"], "rivetkit");
		assert_eq!(health_body["version"], "test-version");

		let metadata = runtime
			.handle_request(test_request("GET", "/api/rivet/metadata"))
			.await;
		assert_eq!(metadata.status, 200);
		let metadata_body = read_body(metadata).await;
		assert_eq!(metadata_body["runtime"], "rivetkit");
		assert_eq!(metadata_body["version"], "test-version");
		assert_eq!(
			metadata_body["envoy"]["kind"]["serverless"],
			serde_json::json!({})
		);
		assert_eq!(metadata_body["clientEndpoint"], "http://client.example");
		assert_eq!(metadata_body["clientNamespace"], "default");
		assert_eq!(metadata_body["clientToken"], "client-token");

		let root = runtime
			.handle_request(test_request("GET", "/api/rivet"))
			.await;
		assert_eq!(root.status, 200);
		let root_body = read_text(root).await;
		assert_eq!(
			root_body,
			"This is a RivetKit server.\n\nLearn more at https://rivet.dev"
		);
	}

	#[tokio::test]
	async fn start_requires_serverless_headers() {
		let runtime = test_runtime().await;
		let response = runtime
			.handle_request(test_request("POST", "/api/rivet/start"))
			.await;
		assert_eq!(response.status, 400);
		let body = read_body(response).await;
		assert_eq!(body["group"], "request");
		assert_eq!(body["code"], "invalid");
	}

	async fn test_runtime() -> CoreServerlessRuntime {
		CoreServerlessRuntime::new(
			HashMap::new(),
			ServeConfig {
				version: 1,
				endpoint: "http://127.0.0.1:6420".to_owned(),
				token: Some("dev".to_owned()),
				namespace: "default".to_owned(),
				pool_name: "default".to_owned(),
				engine_binary_path: None,
				handle_inspector_http_in_runtime: true,
				serverless_base_path: Some("/api/rivet".to_owned()),
				serverless_package_version: "test-version".to_owned(),
				serverless_client_endpoint: Some("http://client.example".to_owned()),
				serverless_client_namespace: Some("default".to_owned()),
				serverless_client_token: Some("client-token".to_owned()),
				serverless_validate_endpoint: true,
				serverless_max_start_payload_bytes: 1_048_576,
			},
		)
		.await
		.expect("runtime should build")
	}

	fn test_request(method: &str, path: &str) -> ServerlessRequest {
		ServerlessRequest {
			method: method.to_owned(),
			url: format!("http://localhost{path}"),
			headers: HashMap::new(),
			body: Vec::new(),
			cancel_token: CancellationToken::new(),
		}
	}

	async fn read_body(response: super::ServerlessResponse) -> serde_json::Value {
		let text = read_text(response).await;
		serde_json::from_str(&text).expect("response should be json")
	}

	async fn read_text(mut response: super::ServerlessResponse) -> String {
		let mut body = Vec::new();
		while let Some(chunk) = response.body.recv().await {
			body.extend(chunk.expect("stream should not error"));
		}
		String::from_utf8(body).expect("response should be utf-8")
	}
}
