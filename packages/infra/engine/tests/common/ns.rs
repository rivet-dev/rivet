use std::str::FromStr;

pub async fn create_namespace(name: &str, guard_port: u16) -> rivet_util::Id {
	tracing::info!(?name, ?guard_port, "creating test namespace");

	let client = reqwest::Client::new();
	let response = client
		.post(format!("http://127.0.0.1:{}/namespaces", guard_port))
		.json(&serde_json::json!({
			"name": name,
			"display_name": "Test Namespace",
		}))
		.send()
		.await
		.expect("Failed to send namespace creation request");

	if !response.status().is_success() {
		let text = response.text().await.expect("Failed to read response text");
		panic!("Failed to create namespace: {}", text);
	}

	let body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse JSON response");
	let namespace_id = body["namespace"]["namespace_id"]
		.as_str()
		.expect("Missing namespace_id in response");

	let namespace_id =
		rivet_util::Id::from_str(namespace_id).expect("Failed to parse namespace ID");

	tracing::info!(?namespace_id, ?name, "namespace created");

	namespace_id
}
