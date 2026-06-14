use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tokio::time::sleep;
use url::Url;

use crate::{POOL_NAME, util::encode};

#[derive(Deserialize)]
pub struct TokenInspectResponse {
	pub project: String,
	pub organization: String,
}

#[derive(Deserialize)]
pub struct NamespaceResponse {
	pub namespace: Namespace,
}

#[derive(Deserialize)]
struct NamespacesResponse {
	namespaces: Vec<Namespace>,
	pagination: Option<Pagination>,
}

#[derive(Deserialize)]
struct Pagination {
	cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Namespace {
	pub name: String,
	pub display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedPoolResponse {
	managed_pool: Option<ManagedPool>,
}

#[derive(Deserialize)]
struct ManagedPool {
	status: Option<String>,
	error: Option<ManagedPoolError>,
}

#[derive(Deserialize)]
struct ManagedPoolError {
	message: Option<String>,
}

pub struct CloudClient {
	http: reqwest::Client,
	base: Url,
	token: String,
}

impl CloudClient {
	pub fn new(base: &str, token: String) -> Result<Self> {
		Ok(Self {
			http: reqwest::Client::new(),
			base: Url::parse(base).context("invalid Cloud API endpoint")?,
			token,
		})
	}

	pub async fn request<T: DeserializeOwned>(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
	) -> Result<Option<T>> {
		let url = self.base.join(path.trim_start_matches('/'))?;
		let mut request = self
			.http
			.request(method, url)
			.bearer_auth(&self.token)
			.header("Content-Type", "application/json");
		if let Some(body) = body {
			request = request.json(&body);
		}
		let response = request.send().await.context("Cloud API request failed")?;
		if response.status() == StatusCode::NOT_FOUND {
			return Ok(None);
		}
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		if !status.is_success() {
			bail!("Cloud API error {status}: {text}");
		}
		if text.trim().is_empty() {
			return Ok(None);
		}
		Ok(Some(serde_json::from_str(&text).with_context(|| {
			format!("Cloud API returned invalid JSON for {path}")
		})?))
	}

	pub async fn request_ok<T: DeserializeOwned>(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
	) -> Result<Option<T>> {
		let url = self.base.join(path.trim_start_matches('/'))?;
		let mut request = self
			.http
			.request(method, url)
			.bearer_auth(&self.token)
			.header("Content-Type", "application/json");
		if let Some(body) = body {
			request = request.json(&body);
		}
		let response = request.send().await.context("Cloud API request failed")?;
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		if !status.is_success() {
			bail!("Cloud API error {status}: {text}");
		}
		if text.trim().is_empty() {
			return Ok(None);
		}
		Ok(Some(serde_json::from_str(&text).with_context(|| {
			format!("Cloud API returned invalid JSON for {path}")
		})?))
	}
}

pub async fn ensure_namespace(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
) -> Result<Namespace> {
	let path = format!(
		"/projects/{}/namespaces/{}?org={}",
		encode(project),
		encode(namespace),
		encode(org)
	);
	if let Some(response) = cloud
		.request::<NamespaceResponse>(Method::GET, &path, None)
		.await?
	{
		return Ok(response.namespace);
	}

	let list_path = format!(
		"/projects/{}/namespaces?org={}&limit=100",
		encode(project),
		encode(org)
	);
	if let Some(response) = cloud
		.request::<NamespacesResponse>(Method::GET, &list_path, None)
		.await?
	{
		let _next_cursor = response.pagination.and_then(|p| p.cursor);
		if let Some(found) = response.namespaces.into_iter().find(|ns| {
			ns.name == namespace
				|| ns
					.display_name
					.as_ref()
					.is_some_and(|display| display.eq_ignore_ascii_case(namespace))
		}) {
			return Ok(found);
		}
	}

	tracing::info!(%namespace, "creating namespace");
	let create_path = format!(
		"/projects/{}/namespaces?org={}",
		encode(project),
		encode(org)
	);
	let response: NamespaceResponse = cloud
		.request(
			Method::POST,
			&create_path,
			Some(json!({ "displayName": namespace })),
		)
		.await?
		.context("namespace create returned no body")?;
	Ok(response.namespace)
}

pub async fn create_or_update_pool(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
	body: Value,
) -> Result<()> {
	let path = format!(
		"/projects/{}/namespaces/{}/managed-pools/{}?org={}",
		encode(project),
		encode(namespace),
		POOL_NAME,
		encode(org)
	);
	let _: Option<Value> = cloud.request_ok(Method::PUT, &path, Some(body)).await?;
	Ok(())
}

async fn get_pool(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
) -> Result<Option<ManagedPool>> {
	let path = format!(
		"/projects/{}/namespaces/{}/managed-pools/{}?org={}",
		encode(project),
		encode(namespace),
		POOL_NAME,
		encode(org)
	);
	Ok(cloud
		.request::<ManagedPoolResponse>(Method::GET, &path, None)
		.await?
		.and_then(|r| r.managed_pool))
}

pub async fn wait_for_pool(
	cloud: &CloudClient,
	project: &str,
	org: &str,
	namespace: &str,
	throw_on_error: bool,
) -> Result<()> {
	for _ in 0..180 {
		let pool = get_pool(cloud, project, org, namespace)
			.await?
			.context("managed pool disappeared while polling")?;
		let status = pool.status.unwrap_or_else(|| "unknown".to_string());
		tracing::info!(%status, "pool status");
		match status.as_str() {
			"ready" => return Ok(()),
			"error" if throw_on_error => {
				bail!(
					"managed pool entered error state: {}",
					pool.error
						.and_then(|e| e.message)
						.unwrap_or_else(|| "unknown error".to_string())
				);
			}
			"error" => return Ok(()),
			_ => sleep(Duration::from_secs(2)).await,
		}
	}
	bail!("timed out waiting for managed pool to become ready")
}

pub fn registry_endpoint(cloud_api: &str) -> Result<String> {
	let url = derive_endpoint(cloud_api, "registry")?;
	// Strip the scheme for Docker image references
	Ok(url
		.trim_start_matches("https://")
		.trim_start_matches("http://")
		.to_string())
}

pub fn dashboard_endpoint(cloud_api: &str) -> Result<String> {
	derive_endpoint(cloud_api, "dashboard")
}

fn derive_endpoint(input: &str, subdomain: &str) -> Result<String> {
	let mut url = Url::parse(input)?;
	let host = url.host_str().context("endpoint missing host")?;
	let next_host = if let Some(rest) = host.strip_prefix("cloud-api.") {
		format!("{subdomain}.{rest}")
	} else if let Some(rest) = host.strip_prefix("api.") {
		format!("{subdomain}.{rest}")
	} else {
		format!("{subdomain}.{host}")
	};
	url.set_host(Some(&next_host))?;
	url.set_path("");
	url.set_query(None);
	url.set_fragment(None);
	Ok(url.as_str().trim_end_matches('/').to_string())
}
