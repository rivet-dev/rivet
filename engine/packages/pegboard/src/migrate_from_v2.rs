//! Placement of keyed creates while a V2 namespace migrates in, and the V2 public API calls it
//! and routing need.
//!
//! A keyed create settles which cluster owns its key once, in the actor's own state machine, so
//! every later create of the key follows that decision whatever the configured share says by
//! then. A create whose [`key_share`] is at or past `create_ratio` is delegated to V2 at once;
//! any other create starts here, and its reservation asks V2 whether it already holds the key.

use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use futures_util::StreamExt;
use rivet_config::config::MigrateFromV2;
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::http::{Method, Request, StatusCode};
use std::time::Duration;

/// How long a V2 public API call may take.
const V2_API_TIMEOUT: Duration = Duration::from_secs(15);

/// Largest V2 public API response read.
const V2_API_RESPONSE_LIMIT_BYTES: usize = 1024 * 1024;

/// The migration that places a create in `namespace`, when that is the namespace migrating in.
pub fn applies<'a>(
	config: Option<&'a MigrateFromV2>,
	namespace: &str,
) -> Option<&'a MigrateFromV2> {
	config.filter(|config| config.target_namespace() == namespace)
}

/// The key's place in `[0, 1)` for `create_ratio`: the leading 53 bits of its placement hash,
/// so every process and every create of the key sees the same value.
pub fn key_share(namespace: &str, name: &str, key: &str) -> f64 {
	let placement = crate::keys::partition::key_placement(namespace, name, key);
	let mut leading = [0u8; 8];
	leading.copy_from_slice(&placement[..8]);
	(u64::from_be_bytes(leading) >> 11) as f64 / (1u64 << 53) as f64
}

/// A V2 public API response, read in full.
pub struct ApiResponse {
	pub status: StatusCode,
	pub headers: http::HeaderMap,
	pub body: Bytes,
}

/// The V2 public API of the lowest datacenter label. Any V2 datacenter resolves keys and
/// creates globally, so one stable choice is enough.
pub fn api_url(config: &MigrateFromV2) -> Result<&str> {
	config
		.api_urls
		.iter()
		.filter_map(|(label, url)| Some((label.parse::<u16>().ok()?, url)))
		.min_by_key(|(label, _)| *label)
		.map(|(_, url)| url.trim_end_matches('/'))
		.context("migrate_from_v2.api_urls names no V2 public API")
}

/// Calls V2's public API with the migration's token. `path_and_query` starts with `/`.
pub async fn api(
	ctx: &EngineContext,
	config: &MigrateFromV2,
	method: Method,
	path_and_query: &str,
	body: Option<Bytes>,
) -> Result<ApiResponse> {
	let uri: http::Uri = format!("{}{path_and_query}", api_url(config)?)
		.parse()
		.context("invalid V2 public API URI")?;
	let has_body = body.is_some();
	let body = futures_util::stream::iter(body.map(Ok)).boxed();
	let mut builder = Request::builder().method(method).uri(uri);
	if has_body {
		builder = builder.header(http::header::CONTENT_TYPE, "application/json");
	}
	if let Some(token) = &config.token {
		builder = builder.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
	}
	let request = builder.body(body)?;

	let clock = ctx.env().clock();
	let deadline = rivet_env::deadline_after(clock.as_ref(), V2_API_TIMEOUT)?;
	let read = async {
		let response = ctx.external_http().execute(request).await?;
		let (parts, mut body) = response.into_parts();
		let mut bytes = Vec::new();
		while let Some(chunk) = body.next().await {
			let chunk = chunk?;
			ensure!(
				bytes.len().saturating_add(chunk.len()) <= V2_API_RESPONSE_LIMIT_BYTES,
				"V2 public API response is larger than {V2_API_RESPONSE_LIMIT_BYTES} bytes"
			);
			bytes.extend_from_slice(&chunk);
		}
		anyhow::Ok(ApiResponse {
			status: parts.status,
			headers: parts.headers,
			body: Bytes::from(bytes),
		})
	};
	rivet_env::timeout_at(clock.as_ref(), deadline, read)
		.await
		.context("V2 public API did not answer before the deadline")?
}

/// Whether V2 holds a live actor for `name` and `key`, in the slash form V2's actor API reads.
pub async fn key_exists(
	ctx: &EngineContext,
	config: &MigrateFromV2,
	name: &str,
	key: &str,
) -> Result<bool> {
	let query = serde_html_form::to_string([
		("namespace", config.namespace.as_str()),
		("name", name),
		("key", key),
	])?;
	let response = api(ctx, config, Method::GET, &format!("/actors?{query}"), None).await?;
	ensure!(
		response.status.is_success(),
		"V2 actor lookup by key failed with status {}",
		response.status
	);
	let listed: ListedActors =
		serde_json::from_slice(&response.body).context("invalid V2 actor listing")?;
	Ok(listed.actors.iter().any(|actor| {
		actor
			.get("destroy_ts")
			.is_none_or(serde_json::Value::is_null)
	}))
}

#[derive(serde::Deserialize)]
struct ListedActors {
	actors: Vec<serde_json::Value>,
}
