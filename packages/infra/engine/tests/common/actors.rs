use serde_json::json;

#[derive(Clone)]
pub struct CreateActorOptions {
	pub namespace: String,
	pub name: String,
	pub key: Option<String>,
	pub input: Option<String>,
	pub runner_name_selector: Option<String>,
	pub durable: bool,
}

impl Default for CreateActorOptions {
	fn default() -> Self {
		Self {
			namespace: "test".to_string(),
			name: "test-actor".to_string(),
			key: Some(rand::random::<u16>().to_string()),
			input: Some(base64::Engine::encode(
				&base64::engine::general_purpose::STANDARD,
				"hello",
			)),
			runner_name_selector: Some("test-runner".to_string()),
			durable: false,
		}
	}
}

pub async fn create_actor_with_options(options: CreateActorOptions, guard_port: u16) -> String {
	tracing::info!(?options.namespace, ?options.name, "creating actor");

	let mut body = json!({
		"name": options.name,
		"key": options.key,
		"crash_policy": if options.durable {
			"restart"
		} else {
			"destroy"
		},
	});

	if let Some(input) = options.input {
		body["input"] = json!(input);
	}
	if let Some(runner_name_selector) = options.runner_name_selector {
		body["runner_name_selector"] = json!(runner_name_selector);
	}
	let url = format!(
		"http://127.0.0.1:{}/actors?namespace={}",
		guard_port, options.namespace
	);

	let client = reqwest::Client::new();
	let response = client
		.post(url)
		.json(&body)
		.send()
		.await
		.expect("Failed to send actor creation request");

	if !response.status().is_success() {
		let text = response.text().await.expect("Failed to read response text");
		panic!("Failed to create actor: {}", text);
	}

	let body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse JSON response");
	let actor_id = body["actor"]["actor_id"]
		.as_str()
		.expect("Missing actor_id in response");

	tracing::info!(?actor_id, "actor created");

	actor_id.to_string()
}

pub async fn create_actor(namespace_name: &str, guard_port: u16) -> String {
	create_actor_with_options(
		CreateActorOptions {
			namespace: namespace_name.to_string(),
			..Default::default()
		},
		guard_port,
	)
	.await
}


/// Pings actor via Guard.
pub async fn ping_actor_via_guard(guard_port: u16, actor_id: &str) -> serde_json::Value {
	tracing::info!(?guard_port, ?actor_id, "sending request to actor via guard");

	let client = reqwest::Client::new();
	let response = client
		.get(format!("http://127.0.0.1:{}/ping", guard_port))
		.header("X-Rivet-Target", "actor")
		.header("X-Rivet-Actor", actor_id)
		.send()
		.await
		.expect("Failed to send ping request through guard");

	if !response.status().is_success() {
		let text = response.text().await.expect("Failed to read response text");
		panic!("Failed to ping actor through guard: {}", text);
	}

	let response = response
		.json()
		.await
		.expect("Failed to parse JSON response");

	tracing::info!(?response, "received response from actor");

	response
}

/// Pings actor directly on the runner's server.
pub async fn ping_actor_via_runner(actor_id: &str, runner_port: u16) -> serde_json::Value {
	tracing::info!(
		?actor_id,
		?runner_port,
		"sending request to actor via runner"
	);

	let client = reqwest::Client::new();
	let response = client
		.get(format!("http://127.0.0.1:{}/ping", runner_port))
		.header("X-Rivet-Actor", actor_id)
		.send()
		.await
		.expect("Failed to send ping request");

	if !response.status().is_success() {
		let text = response.text().await.expect("Failed to read response text");
		panic!("Failed to ping actor: {}", text);
	}

	let response = response
		.json()
		.await
		.expect("Failed to parse JSON response");

	tracing::info!(?response, "received response from actor");

	response
}

pub async fn destroy_actor(actor_id: &str, namespace_name: &str, guard_port: u16) {
	let client = reqwest::Client::new();
	let url = format!(
		"http://127.0.0.1:{}/actors/{}?namespace={}",
		guard_port, actor_id, namespace_name
	);

	tracing::info!(?url, "sending delete request");

	let response = client
		.delete(&url)
		.send()
		.await
		.expect("Failed to send delete request");

	let status = response.status();
	let headers = response.headers().clone();
	let text = response.text().await.expect("Failed to read response text");

	tracing::info!(?status, ?headers, ?text, "received response");

	if !status.is_success() {
		panic!("Failed to destroy actor: {}", text);
	}
}

pub async fn destroy_actor_without_namespace(actor_id: &str, guard_port: u16) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!("http://127.0.0.1:{}/actors/{}", guard_port, actor_id);

	tracing::info!(?url, "sending delete request without namespace");

	client
		.delete(&url)
		.send()
		.await
		.expect("Failed to send delete request")
}

pub async fn get_actor(
	actor_id: &str,
	namespace: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = if let Some(ns) = namespace {
		format!(
			"http://127.0.0.1:{}/actors/{}?namespace={}",
			guard_port, actor_id, ns
		)
	} else {
		format!("http://127.0.0.1:{}/actors/{}", guard_port, actor_id)
	};

	tracing::info!(?url, "getting actor");

	client
		.get(&url)
		.send()
		.await
		.expect("Failed to send get request")
}

pub async fn get_actor_by_id(
	namespace: &str,
	name: &str,
	key: &str,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!("http://127.0.0.1:{}/actors/by-id", guard_port);

	tracing::info!(?url, ?namespace, ?name, ?key, "getting actor by id");

	client
		.get(&url)
		.query(&[("namespace", namespace), ("name", name), ("key", key)])
		.send()
		.await
		.expect("Failed to send get by id request")
}

pub async fn get_or_create_actor(
	namespace: &str,
	name: &str,
	key: Option<String>,
	durable: bool,
	input: Option<String>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!(
		"http://127.0.0.1:{}/actors?namespace={}",
		guard_port, namespace
	);

	let mut body = json!({
		"name": name,
		"key": key,
		"crash_policy": if durable {
			"restart"
		} else {
			"destroy"
		},
		"runner_name_selector": "test-runner",
	});

	if let Some(input) = input {
		body["input"] = json!(input);
	}

	tracing::info!(?url, ?body, "get or create actor");

	client
		.put(&url)
		.json(&body)
		.send()
		.await
		.expect("Failed to send get or create request")
}

pub async fn get_or_create_actor_by_id(
	namespace: &str,
	name: &str,
	key: Option<String>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!(
		"http://127.0.0.1:{}/actors/by-id?namespace={}",
		guard_port, namespace
	);

	let mut body = json!({
		"name": name,
		"key": key,
		"runner_name_selector": "test-runner",
	});

	tracing::info!(?url, ?body, "get or create actor by id");

	client
		.put(&url)
		.json(&body)
		.send()
		.await
		.expect("Failed to send get or create by id request")
}

pub async fn list_actors(
	namespace: &str,
	name: Option<&str>,
	key: Option<String>,
	actor_ids: Option<Vec<String>>,
	include_destroyed: Option<bool>,
	limit: Option<u32>,
	cursor: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let mut url = format!(
		"http://127.0.0.1:{}/actors?namespace={}",
		guard_port, namespace
	);

	if let Some(name) = name {
		url.push_str(&format!("&name={}", name));
	}
	if let Some(key) = key {
		url.push_str(&format!("&key={}", key));
	}
	if let Some(actor_ids) = actor_ids {
		url.push_str(&format!("&actor_ids={}", actor_ids.join(",")));
	}
	if let Some(include_destroyed) = include_destroyed {
		url.push_str(&format!("&include_destroyed={}", include_destroyed));
	}
	if let Some(limit) = limit {
		url.push_str(&format!("&limit={}", limit));
	}
	if let Some(cursor) = cursor {
		url.push_str(&format!("&cursor={}", cursor));
	}

	tracing::info!(?url, "listing actors");

	client
		.get(&url)
		.send()
		.await
		.expect("Failed to send list request")
}

pub async fn list_actor_names(
	namespace: &str,
	limit: Option<u32>,
	cursor: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let mut url = format!(
		"http://127.0.0.1:{}/actors/names?namespace={}",
		guard_port, namespace
	);

	if let Some(limit) = limit {
		url.push_str(&format!("&limit={}", limit));
	}
	if let Some(cursor) = cursor {
		url.push_str(&format!("&cursor={}", cursor));
	}

	tracing::info!(?url, "listing actor names");

	client
		.get(&url)
		.send()
		.await
		.expect("Failed to send list names request")
}

// Test helper functions
pub fn assert_success_response(response: &reqwest::Response) {
	assert!(
		response.status().is_success(),
		"{} Response not successful: {}",
		response.url(),
		response.status()
	);
}

pub async fn assert_error_response(
	response: reqwest::Response,
	expected_error_code: &str,
) -> serde_json::Value {
	assert!(
		!response.status().is_success(),
		"{} Expected error but got success: {}",
		response.url(),
		response.status()
	);

	let body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse error response");

	let error_code = body["code"]
		.as_str()
		.expect("Missing error code in response");
	assert_eq!(
		error_code, expected_error_code,
		"Expected error code {} but got {}",
		expected_error_code, error_code
	);

	body
}

pub fn generate_unique_key() -> String {
	format!("key-{}", rand::random::<u32>())
}

pub async fn bulk_create_actors(
	namespace: &str,
	prefix: &str,
	count: usize,
	guard_port: u16,
) -> Vec<String> {
	let mut actor_ids = Vec::new();
	for i in 0..count {
		let actor_id = create_actor_with_options(
			CreateActorOptions {
				namespace: namespace.to_string(),
				name: format!("{}-{}", prefix, i),
				key: Some(generate_unique_key()),
				..Default::default()
			},
			guard_port,
		)
		.await;
		actor_ids.push(actor_id);
	}
	actor_ids
}
