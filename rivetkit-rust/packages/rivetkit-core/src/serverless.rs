use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use http::StatusCode;
use reqwest::Url;
use rivet_envoy_client::config::EnvoyConfig;
use rivet_envoy_client::envoy::start_envoy;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use rivetkit_shared_types::serverless_metadata::{
	ActorName, ServerlessMetadataEnvoy, ServerlessMetadataPayload,
};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{Mutex as TokioMutex, mpsc};
use tokio_util::sync::CancellationToken;

use crate::actor::factory::ActorFactory;
use crate::engine_process::EngineProcessManager;
use crate::registry::{RegistryCallbacks, RegistryDispatcher, ServeConfig};

const DEFAULT_BASE_PATH: &str = "/api/rivet";
const SSE_PING_INTERVAL: Duration = Duration::from_secs(1);
/// Bound on `handle.shutdown_and_wait` inside teardown paths. If envoy cannot
/// reach the engine (reconnect loop stuck), we fall back to immediate `Stop`
/// rather than hanging indefinitely. Must stay below the outer TS grace ceiling.
const SHUTDOWN_DRAIN_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub struct CoreServerlessRuntime {
	settings: Arc<ServerlessSettings>,
	dispatcher: Arc<RegistryDispatcher>,
	envoy: Arc<TokioMutex<Option<EnvoyHandle>>>,
	_engine_process: Arc<TokioMutex<Option<EngineProcessManager>>>,
	shutting_down: Arc<AtomicBool>,
}

#[derive(Clone, Debug)]
struct ServerlessSettings {
	version: u32,
	configured_endpoint: String,
	configured_token: Option<String>,
	configured_namespace: String,
	base_path: String,
	package_version: String,
	client_endpoint: Option<String>,
	client_namespace: Option<String>,
	client_token: Option<String>,
	validate_endpoint: bool,
	max_start_payload_bytes: usize,
}

#[derive(Debug)]
pub struct ServerlessRequest {
	pub method: String,
	pub url: String,
	pub headers: HashMap<String, String>,
	pub body: Vec<u8>,
	pub cancel_token: CancellationToken,
}

#[derive(Debug)]
pub struct ServerlessResponse {
	pub status: u16,
	pub headers: HashMap<String, String>,
	pub body: mpsc::Receiver<Result<Vec<u8>, ServerlessStreamError>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServerlessStreamError {
	pub group: String,
	pub code: String,
	pub message: String,
}

#[derive(Debug)]
struct StartHeaders {
	endpoint: String,
	token: Option<String>,
	pool_name: String,
	namespace: String,
}

#[derive(Debug, Serialize)]
struct ServerlessErrorBody<'a> {
	group: &'a str,
	code: &'a str,
	message: String,
	metadata: serde_json::Value,
}

#[derive(rivet_error::RivetError, Serialize)]
#[error("request", "invalid", "Invalid request.", "Invalid request: {reason}")]
struct InvalidRequest {
	reason: String,
}

#[derive(rivet_error::RivetError, Serialize)]
#[error(
	"config",
	"endpoint_mismatch",
	"Endpoint mismatch.",
	"Endpoint mismatch: expected \"{expected}\", received \"{received}\""
)]
struct EndpointMismatch {
	expected: String,
	received: String,
}

#[derive(rivet_error::RivetError, Serialize)]
#[error(
	"config",
	"namespace_mismatch",
	"Namespace mismatch.",
	"Namespace mismatch: expected \"{expected}\", received \"{received}\""
)]
struct NamespaceMismatch {
	expected: String,
	received: String,
}

#[derive(rivet_error::RivetError, Serialize)]
#[error(
	"message",
	"incoming_too_long",
	"Incoming message too long.",
	"Incoming message too long. Received {size} bytes, limit is {limit} bytes."
)]
struct IncomingMessageTooLong {
	size: usize,
	limit: usize,
}

#[derive(rivet_error::RivetError, Serialize)]
#[error(
	"registry",
	"shut_down",
	"Registry is shut down.",
	"Registry is shut down; no new requests can be accepted."
)]
struct RuntimeShutDown;

impl CoreServerlessRuntime {
	pub(crate) async fn new(
		factories: HashMap<String, Arc<ActorFactory>>,
		config: ServeConfig,
	) -> Result<Self> {
		let engine_process = match config.engine_binary_path.as_ref() {
			Some(binary_path) => {
				Some(EngineProcessManager::start(binary_path, &config.endpoint).await?)
			}
			None => None,
		};

		let dispatcher = Arc::new(RegistryDispatcher::new(
			factories,
			config.handle_inspector_http_in_runtime,
		));
		let base_path = normalize_base_path(config.serverless_base_path.as_deref());

		Ok(Self {
			settings: Arc::new(ServerlessSettings {
				version: config.version,
				configured_endpoint: config.endpoint,
				configured_token: config.token,
				configured_namespace: config.namespace,
				base_path,
				package_version: config.serverless_package_version,
				client_endpoint: config.serverless_client_endpoint,
				client_namespace: config.serverless_client_namespace,
				client_token: config.serverless_client_token,
				validate_endpoint: config.serverless_validate_endpoint,
				max_start_payload_bytes: config.serverless_max_start_payload_bytes,
			}),
			dispatcher,
			envoy: Arc::new(TokioMutex::new(None)),
			_engine_process: Arc::new(TokioMutex::new(engine_process)),
			shutting_down: Arc::new(AtomicBool::new(false)),
		})
	}

	/// Tear down the cached envoy handle. Idempotent.
	///
	/// Sets `shutting_down` so concurrent `ensure_envoy` callers short-circuit
	/// instead of starting a fresh envoy after teardown, and waits (with a
	/// bounded timeout) for `envoy_loop` to exit. If the drain exceeds the
	/// timeout (e.g. engine unreachable), falls back to an immediate `Stop`.
	pub async fn shutdown(&self) {
		self.shutting_down.store(true, Ordering::Release);
		let handle = { self.envoy.lock().await.take() };
		let Some(handle) = handle else { return };
		match tokio::time::timeout(SHUTDOWN_DRAIN_TIMEOUT, handle.shutdown_and_wait(false)).await {
			Ok(()) => {}
			Err(_) => {
				tracing::warn!(
					"serverless runtime envoy drain exceeded timeout; forcing immediate stop"
				);
				handle.shutdown(true);
				handle.wait_stopped().await;
			}
		}
	}

	pub async fn handle_request(&self, req: ServerlessRequest) -> ServerlessResponse {
		let cors = cors_headers(&req);
		match self.handle_request_inner(req).await {
			Ok(mut response) => {
				apply_cors(&mut response.headers, cors);
				response
			}
			Err(error) => {
				let mut response = error_response(error);
				apply_cors(&mut response.headers, cors);
				response
			}
		}
	}

	async fn handle_request_inner(&self, req: ServerlessRequest) -> Result<ServerlessResponse> {
		let path = route_path(&self.settings.base_path, &req.url)?;
		match (req.method.as_str(), path.as_str()) {
			("GET", "") | ("GET", "/") => Ok(text_response(
				StatusCode::OK,
				"text/plain; charset=utf-8",
				"This is a RivetKit server.\n\nLearn more at https://rivet.dev",
			)),
			("GET", "/health") => Ok(json_response(
				StatusCode::OK,
				json!({
					"status": "ok",
					"runtime": "rivetkit",
					"version": self.settings.package_version,
				}),
			)),
			("GET", "/metadata") => Ok(self.metadata_response()),
			("POST", "/start") => self.start_response(req).await,
			("OPTIONS", _) => Ok(bytes_response(
				StatusCode::NO_CONTENT,
				HashMap::new(),
				Vec::new(),
			)),
			_ => Ok(text_response(
				StatusCode::NOT_FOUND,
				"text/plain; charset=utf-8",
				"Not Found (RivetKit)",
			)),
		}
	}

	async fn start_response(&self, req: ServerlessRequest) -> Result<ServerlessResponse> {
		let headers = parse_start_headers(&req.headers)?;
		self.validate_start_headers(&headers)?;
		if req.body.len() > self.settings.max_start_payload_bytes {
			return Err(IncomingMessageTooLong {
				size: req.body.len(),
				limit: self.settings.max_start_payload_bytes,
			}
			.build());
		}
		let handle = self.ensure_envoy(&headers).await?;
		let payload = req.body;
		let cancel_token = req.cancel_token;
		let (tx, rx) = mpsc::channel(16);

		tokio::spawn(async move {
			let result = tokio::select! {
				_ = cancel_token.cancelled() => {
					return;
				}
				result = handle.start_serverless_actor(&payload) => result,
			};
			if let Err(error) = result {
				let error = stream_error(error);
				let _ = tx.send(Err(error)).await;
				return;
			}

			loop {
				tokio::select! {
					_ = cancel_token.cancelled() => {
						break;
					}
					_ = tokio::time::sleep(SSE_PING_INTERVAL) => {
						if tx.send(Ok(b"event: ping\ndata:\n\n".to_vec())).await.is_err() {
							break;
						}
					}
				}
			}
		});

		Ok(ServerlessResponse {
			status: StatusCode::OK.as_u16(),
			headers: HashMap::from([
				("content-type".to_owned(), "text/event-stream".to_owned()),
				("cache-control".to_owned(), "no-cache".to_owned()),
				("connection".to_owned(), "keep-alive".to_owned()),
			]),
			body: rx,
		})
	}

	fn metadata_response(&self) -> ServerlessResponse {
		let actor_names = self
			.dispatcher
			.factories
			.iter()
			.map(|(actor_name, factory): (&String, &Arc<ActorFactory>)| {
				let config = factory.config();
				let mut metadata = serde_json::Map::new();
				if let Some(icon) = &config.icon {
					metadata.insert("icon".to_owned(), json!(icon));
				}
				if let Some(name) = &config.name {
					metadata.insert("name".to_owned(), json!(name));
				}
				metadata.insert(
					"preload".to_owned(),
					json!({
						"keys": [
							[1],
							[3],
							[5, 1, 1],
						],
						"prefixes": [
							{
								"prefix": [6, 1],
								"maxBytes": config.preload_max_workflow_bytes.unwrap_or(131_072),
								"partial": false,
							},
							{
								"prefix": [2],
								"maxBytes": config.preload_max_connections_bytes.unwrap_or(65_536),
								"partial": false,
							},
							{
								"prefix": [5, 1, 2],
								"maxBytes": 65_536,
								"partial": false,
							},
						],
					}),
				);
				(
					actor_name.clone(),
					ActorName {
						metadata: Some(serde_json::Value::Object(metadata)),
					},
				)
			})
			.collect::<HashMap<_, _>>();

		let payload = ServerlessMetadataPayload {
			runtime: "rivetkit".to_owned(),
			version: self.settings.package_version.clone(),
			envoy_protocol_version: Some(protocol::PROTOCOL_VERSION),
			actor_names,
			envoy: Some(ServerlessMetadataEnvoy {
				version: Some(self.settings.version),
			}),
			runner: None,
		};

		let mut response = serde_json::to_value(payload).unwrap_or_else(|_| json!({}));

		if let serde_json::Value::Object(object) = &mut response {
			if object.get("runner").is_some_and(serde_json::Value::is_null) {
				object.remove("runner");
			}
			if let Some(envoy) = object
				.get_mut("envoy")
				.and_then(|value| value.as_object_mut())
			{
				envoy.insert("kind".to_owned(), json!({ "serverless": {} }));
			}
			if let Some(client_endpoint) = &self.settings.client_endpoint {
				object.insert("clientEndpoint".to_owned(), json!(client_endpoint));
			}
			if let Some(client_namespace) = &self.settings.client_namespace {
				object.insert("clientNamespace".to_owned(), json!(client_namespace));
			}
			if let Some(client_token) = &self.settings.client_token {
				object.insert("clientToken".to_owned(), json!(client_token));
			}
		}

		json_response(StatusCode::OK, response)
	}

	fn validate_start_headers(&self, headers: &StartHeaders) -> Result<()> {
		// TODO: pegboard-outbound does not currently auth the /start endpoint,
		// so the incoming `x-rivet-token` does not match `config.token`
		// (which is the user's API token, not a shared pool secret). Re-enable
		// once the envoy-era serverless pool carries a dedicated shared secret
		// in its configured headers.
		// if let Some(expected_token) = &self.settings.configured_token {
		// 	let Some(received_token) = &headers.token else {
		// 		return Err(Forbidden.build());
		// 	};
		// 	if !constant_time_eq(expected_token, received_token) {
		// 		return Err(Forbidden.build());
		// 	}
		// }

		if self.settings.validate_endpoint {
			if !endpoints_match(&headers.endpoint, &self.settings.configured_endpoint) {
				tracing::warn!(
					configured_endpoint = %self.settings.configured_endpoint,
					received_endpoint = %headers.endpoint,
					"serverless start rejected: endpoint mismatch",
				);
				return Err(EndpointMismatch {
					expected: self.settings.configured_endpoint.clone(),
					received: headers.endpoint.clone(),
				}
				.build());
			}

			if headers.namespace != self.settings.configured_namespace {
				tracing::warn!(
					configured_namespace = %self.settings.configured_namespace,
					received_namespace = %headers.namespace,
					"serverless start rejected: namespace mismatch",
				);
				return Err(NamespaceMismatch {
					expected: self.settings.configured_namespace.clone(),
					received: headers.namespace.clone(),
				}
				.build());
			}
		}

		Ok(())
	}

	async fn ensure_envoy(&self, headers: &StartHeaders) -> Result<EnvoyHandle> {
		if self.shutting_down.load(Ordering::Acquire) {
			return Err(RuntimeShutDown.build());
		}
		let mut guard = self.envoy.lock().await;
		if let Some(handle) = guard.as_ref() {
			if !endpoints_match(handle.endpoint(), &headers.endpoint)
				|| handle.namespace() != headers.namespace
				|| handle.pool_name() != headers.pool_name
			{
				anyhow::bail!("serverless start headers do not match active envoy");
			}
			return Ok(handle.clone());
		}

		let callbacks = Arc::new(RegistryCallbacks {
			dispatcher: self.dispatcher.clone(),
		});
		// not_global: true to avoid caching the handle in the process-wide
		// `GLOBAL_ENVOY` OnceLock. Without this, a shutdown-during-build race
		// (spec §3 step 7) leaves a dead handle cached for the life of the
		// process and any subsequent consumer gets it back.
		let handle = start_envoy(EnvoyConfig {
			version: self.settings.version,
			endpoint: headers.endpoint.clone(),
			token: self
				.settings
				.configured_token
				.clone()
				.or_else(|| headers.token.clone()),
			namespace: headers.namespace.clone(),
			pool_name: headers.pool_name.clone(),
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: true,
			debug_latency_ms: None,
			callbacks,
		})
		.await;
		// Re-check under the lock: shutdown may have run while we were awaiting
		// `start_envoy`. If so, tear down the freshly-built envoy rather than
		// installing it into the cache.
		if self.shutting_down.load(Ordering::Acquire) {
			drop(guard);
			match tokio::time::timeout(SHUTDOWN_DRAIN_TIMEOUT, handle.shutdown_and_wait(false))
				.await
			{
				Ok(()) => {}
				Err(_) => {
					handle.shutdown(true);
					handle.wait_stopped().await;
				}
			}
			return Err(RuntimeShutDown.build());
		}
		*guard = Some(handle.clone());
		Ok(handle)
	}
}

fn route_path(base_path: &str, url: &str) -> Result<String> {
	let parsed = Url::parse(url).with_context(|| format!("parse request URL `{url}`"))?;
	let path = parsed.path();
	if path == base_path {
		return Ok(String::new());
	}
	let prefix = format!("{base_path}/");
	if let Some(rest) = path.strip_prefix(&prefix) {
		return Ok(format!("/{rest}"));
	}
	Ok(path.to_owned())
}

fn parse_start_headers(headers: &HashMap<String, String>) -> Result<StartHeaders> {
	Ok(StartHeaders {
		endpoint: required_header(headers, "x-rivet-endpoint")?,
		token: optional_header(headers, "x-rivet-token"),
		pool_name: required_header(headers, "x-rivet-pool-name")?,
		namespace: required_header(headers, "x-rivet-namespace-name")?,
	})
}

fn required_header(headers: &HashMap<String, String>, name: &str) -> Result<String> {
	headers
		.get(name)
		.filter(|value| !value.is_empty())
		.cloned()
		.ok_or_else(|| {
			InvalidRequest {
				reason: format!("{name} header is required"),
			}
			.build()
		})
}

fn optional_header(headers: &HashMap<String, String>, name: &str) -> Option<String> {
	headers.get(name).filter(|value| !value.is_empty()).cloned()
}

fn cors_headers(req: &ServerlessRequest) -> HashMap<String, String> {
	let origin = req
		.headers
		.get("origin")
		.cloned()
		.unwrap_or_else(|| "*".to_owned());
	let mut headers = HashMap::from([
		("access-control-allow-origin".to_owned(), origin.clone()),
		(
			"access-control-allow-credentials".to_owned(),
			"true".to_owned(),
		),
		("access-control-expose-headers".to_owned(), "*".to_owned()),
	]);
	if origin != "*" {
		headers.insert("vary".to_owned(), "Origin".to_owned());
	}

	if req.method == "OPTIONS" {
		headers.insert(
			"access-control-allow-methods".to_owned(),
			"GET, POST, PUT, DELETE, OPTIONS, PATCH".to_owned(),
		);
		headers.insert(
			"access-control-allow-headers".to_owned(),
			req.headers
				.get("access-control-request-headers")
				.cloned()
				.unwrap_or_else(|| "*".to_owned()),
		);
		headers.insert("access-control-max-age".to_owned(), "86400".to_owned());
	}

	headers
}

fn apply_cors(headers: &mut HashMap<String, String>, cors: HashMap<String, String>) {
	headers.extend(cors);
}

fn normalize_base_path(base_path: Option<&str>) -> String {
	let base_path = base_path
		.filter(|base_path| !base_path.is_empty())
		.unwrap_or(DEFAULT_BASE_PATH);
	let prefixed = if base_path.starts_with('/') {
		base_path.to_owned()
	} else {
		format!("/{base_path}")
	};
	let trimmed = prefixed.trim_end_matches('/');
	if trimmed.is_empty() {
		"/".to_owned()
	} else {
		trimmed.to_owned()
	}
}

fn text_response(status: StatusCode, content_type: &str, body: &str) -> ServerlessResponse {
	bytes_response(
		status,
		HashMap::from([("content-type".to_owned(), content_type.to_owned())]),
		body.as_bytes().to_vec(),
	)
}

fn json_response(status: StatusCode, body: serde_json::Value) -> ServerlessResponse {
	bytes_response(
		status,
		HashMap::from([("content-type".to_owned(), "application/json".to_owned())]),
		serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec()),
	)
}

fn bytes_response(
	status: StatusCode,
	headers: HashMap<String, String>,
	body: Vec<u8>,
) -> ServerlessResponse {
	let (tx, rx) = mpsc::channel(1);
	tokio::spawn(async move {
		let _ = tx.send(Ok(body)).await;
	});
	ServerlessResponse {
		status: status.as_u16(),
		headers,
		body: rx,
	}
}

fn error_response(error: anyhow::Error) -> ServerlessResponse {
	let extracted = rivet_error::RivetError::extract(&error);
	let status = serverless_error_status(extracted.group(), extracted.code());
	let body = ServerlessErrorBody {
		group: extracted.group(),
		code: extracted.code(),
		message: extracted.message().to_owned(),
		metadata: extracted.metadata().unwrap_or(serde_json::Value::Null),
	};
	bytes_response(
		status,
		HashMap::from([("content-type".to_owned(), "application/json".to_owned())]),
		serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec()),
	)
}

fn serverless_error_status(group: &str, code: &str) -> StatusCode {
	match (group, code) {
		("auth", "forbidden") => StatusCode::FORBIDDEN,
		("message", "incoming_too_long") => StatusCode::PAYLOAD_TOO_LARGE,
		_ => StatusCode::BAD_REQUEST,
	}
}

fn stream_error(error: anyhow::Error) -> ServerlessStreamError {
	let extracted = rivet_error::RivetError::extract(&error);
	ServerlessStreamError {
		group: extracted.group().to_owned(),
		code: extracted.code().to_owned(),
		message: extracted.message().to_owned(),
	}
}

pub fn normalize_endpoint_url(url: &str) -> Option<String> {
	let parsed = Url::parse(url).ok()?;
	let pathname = if parsed.path() == "/" {
		"/".to_owned()
	} else {
		parsed.path().trim_end_matches('/').to_owned()
	};
	let mut hostname = parsed.host_str()?.to_owned();
	if is_loopback_address(&hostname) {
		hostname = "localhost".to_owned();
	}
	hostname = normalize_regional_hostname(&hostname);
	let host = match parsed.port() {
		Some(port) => format!("{hostname}:{port}"),
		None => hostname,
	};
	Some(format!("{}://{}{}", parsed.scheme(), host, pathname))
}

pub fn endpoints_match(a: &str, b: &str) -> bool {
	match (normalize_endpoint_url(a), normalize_endpoint_url(b)) {
		(Some(a), Some(b)) => a == b,
		_ => a == b,
	}
}

fn normalize_regional_hostname(hostname: &str) -> String {
	if !hostname.ends_with(".rivet.dev") || !hostname.starts_with("api-") {
		return hostname.to_owned();
	}
	let without_prefix = &hostname[4..];
	let Some(first_dot_index) = without_prefix.find('.') else {
		return hostname.to_owned();
	};
	let domain = &without_prefix[first_dot_index + 1..];
	format!("api.{domain}")
}

fn is_loopback_address(hostname: &str) -> bool {
	matches!(hostname, "127.0.0.1" | "0.0.0.0" | "::1" | "[::1]")
}

#[cfg(test)]
mod tests {
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
