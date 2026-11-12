//! Calls to V2's public `/migration/v1/` routes. The migrator uses only these routes.
//!
//! Every call is one request with one reply or an error; retries belong to the migrator state
//! machine, which reissues the same idempotent request.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use bytes::Bytes;
use futures_util::{StreamExt, stream::BoxStream};
use migrate_from_v2_protocol as proto;
use rivet_env::capabilities::http::{ExternalHttpClient, HttpBody, Method, Request};
use rivet_util::Id;

/// The most bytes a non-export reply may carry.
const MAX_REPLY_BYTES: usize = 64 * 1024 * 1024;

/// A non-success reply from V2. `group` and `code` are V2's `RivetError` identity when the body
/// carries one, so callers can tell a definite refusal from a transient failure.
#[derive(Debug)]
pub struct V2ApiError {
	pub url: String,
	pub status: u16,
	pub group: Option<String>,
	pub code: Option<String>,
	pub body: String,
}

impl V2ApiError {
	pub fn is(&self, group: &str, code: &str) -> bool {
		self.group.as_deref() == Some(group) && self.code.as_deref() == Some(code)
	}
}

impl std::fmt::Display for V2ApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"V2 migration request {} returned {}: {}",
			self.url, self.status, self.body
		)
	}
}

impl std::error::Error for V2ApiError {}

#[derive(serde::Deserialize)]
struct ErrorBody {
	group: String,
	code: String,
}

#[derive(Clone)]
pub struct V2Client {
	http: Arc<dyn ExternalHttpClient>,
	/// V2 public API base URL per datacenter label.
	api_urls: BTreeMap<u16, String>,
	token: Option<String>,
	namespace: String,
	/// The namespace in this cluster the actors land in.
	target_namespace: String,
}

impl V2Client {
	pub fn new(
		http: Arc<dyn ExternalHttpClient>,
		settings: &rivet_config::config::MigrateFromV2,
	) -> Result<Self> {
		let mut api_urls = BTreeMap::new();
		for (label, url) in &settings.api_urls {
			let label = label.parse::<u16>().with_context(|| {
				format!("migrate_from_v2.api_urls key {label} is not a datacenter label")
			})?;
			api_urls.insert(label, url.trim_end_matches('/').to_string());
		}
		ensure!(!api_urls.is_empty(), "migrate_from_v2.api_urls is empty");

		Ok(Self {
			http,
			api_urls,
			token: settings.token.clone(),
			namespace: settings.namespace.clone(),
			target_namespace: settings.target_namespace().to_owned(),
		})
	}

	pub fn target_namespace(&self) -> &str {
		&self.target_namespace
	}

	pub fn datacenter_labels(&self) -> Vec<u16> {
		self.api_urls.keys().copied().collect()
	}

	fn url(&self, datacenter_label: u16, path: &str) -> Result<String> {
		let base = self
			.api_urls
			.get(&datacenter_label)
			.with_context(|| format!("no V2 api url for datacenter {datacenter_label}"))?;
		Ok(format!("{base}/migration/v1/{path}"))
	}

	fn request(&self, method: Method, url: &str, body: Vec<u8>) -> Result<Request<HttpBody>> {
		let mut builder = Request::builder()
			.method(method)
			.uri(url)
			.header("content-type", proto::BARE_CONTENT_TYPE);
		if let Some(token) = &self.token {
			builder = builder.header("authorization", format!("Bearer {token}"));
		}
		let body: HttpBody =
			futures_util::stream::once(async move { Ok(Bytes::from(body)) }).boxed();
		builder.body(body).context("build V2 migration request")
	}

	/// Sends one request and returns the whole reply body, failing on a non-success status with
	/// V2's error body in the message.
	async fn call(&self, method: Method, url: String, body: Vec<u8>) -> Result<Vec<u8>> {
		let response = self
			.http
			.execute(self.request(method, &url, body)?)
			.await
			.with_context(|| format!("V2 migration request to {url}"))?;
		let status = response.status();
		let body = collect(response.into_body(), MAX_REPLY_BYTES).await?;
		if !status.is_success() {
			let identity = serde_json::from_slice::<ErrorBody>(&body).ok();
			return Err(V2ApiError {
				url,
				status: status.as_u16(),
				group: identity.as_ref().map(|identity| identity.group.clone()),
				code: identity.map(|identity| identity.code),
				body: String::from_utf8_lossy(&body).into_owned(),
			}
			.into());
		}
		Ok(body)
	}

	pub async fn candidates(
		&self,
		datacenter_label: u16,
		after: Option<&[u8]>,
		limit: usize,
	) -> Result<proto::CandidatesResponse> {
		let mut path = format!(
			"namespaces/{}/candidates?limit={limit}",
			urlencoding::encode(&self.namespace)
		);
		if let Some(after) = after {
			path.push_str("&after=");
			path.push_str(&base64_url(after));
		}
		let body = self
			.call(Method::GET, self.url(datacenter_label, &path)?, Vec::new())
			.await?;
		proto::decode(&body)
	}

	/// The candidate facts of one actor, or `None` when V2 holds no such actor in the namespace.
	pub async fn candidate(&self, v2_actor_id: Id) -> Result<Option<proto::Candidate>> {
		let reply = self
			.call(
				Method::GET,
				self.actor_url(v2_actor_id, "candidate")?,
				Vec::new(),
			)
			.await;
		match reply {
			Ok(body) => Ok(Some(proto::decode(&body)?)),
			Err(error)
				if error
					.downcast_ref::<V2ApiError>()
					.is_some_and(|error| error.is("migration", "actor_not_found")) =>
			{
				Ok(None)
			}
			Err(error) => Err(error),
		}
	}

	/// Opens the export stream. The caller reads frames until `ExportEnd` or `ExportAbort`.
	pub async fn export(
		&self,
		v2_actor_id: Id,
		resume: Option<&[u8]>,
	) -> Result<BoxStream<'static, Result<Bytes>>> {
		let mut path = format!(
			"actors/{v2_actor_id}/export?namespace={}",
			urlencoding::encode(&self.namespace)
		);
		if let Some(resume) = resume {
			path.push_str("&resume=");
			path.push_str(&base64_url(resume));
		}
		let url = self.url(v2_actor_id.label(), &path)?;
		let response = self
			.http
			.execute(self.request(Method::GET, &url, Vec::new())?)
			.await
			.with_context(|| format!("V2 export request to {url}"))?;
		let status = response.status();
		if !status.is_success() {
			let body = collect(response.into_body(), MAX_REPLY_BYTES).await?;
			bail!(
				"V2 export {url} returned {status}: {}",
				String::from_utf8_lossy(&body)
			);
		}
		Ok(response.into_body())
	}

	pub async fn cutover(
		&self,
		v2_actor_id: Id,
		request: &proto::CutoverRequest,
	) -> Result<proto::CutoverResponse> {
		let body = self
			.call(
				Method::POST,
				self.actor_url(v2_actor_id, "cutover")?,
				proto::encode(request),
			)
			.await?;
		proto::decode(&body)
	}

	pub async fn confirm(
		&self,
		v2_actor_id: Id,
		request: &proto::ConfirmRequest,
	) -> Result<proto::ConfirmResponse> {
		let body = self
			.call(
				Method::POST,
				self.actor_url(v2_actor_id, "confirm")?,
				proto::encode(request),
			)
			.await?;
		proto::decode(&body)
	}

	pub async fn restore(
		&self,
		v2_actor_id: Id,
		request: &proto::RestoreRequest,
	) -> Result<proto::RestoreResponse> {
		let body = self
			.call(
				Method::POST,
				self.actor_url(v2_actor_id, "restore")?,
				proto::encode(request),
			)
			.await?;
		proto::decode(&body)
	}

	fn actor_url(&self, v2_actor_id: Id, route: &str) -> Result<String> {
		self.url(
			v2_actor_id.label(),
			&format!(
				"actors/{v2_actor_id}/{route}?namespace={}",
				urlencoding::encode(&self.namespace)
			),
		)
	}
}

/// A fixed timeout for one V2 call that is not an export.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(30);

async fn collect(mut body: HttpBody, max_bytes: usize) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	while let Some(chunk) = body.next().await {
		let chunk = chunk?;
		ensure!(
			out.len() + chunk.len() <= max_bytes,
			"V2 reply exceeds {max_bytes} bytes"
		);
		out.extend_from_slice(&chunk);
	}
	Ok(out)
}

fn base64_url(bytes: &[u8]) -> String {
	use base64::Engine;
	base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
