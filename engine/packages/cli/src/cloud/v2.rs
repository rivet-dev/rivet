use std::path::Path;

use anyhow::{Context, Result, bail};
use reqwest::{Method, StatusCode, header::CONTENT_LENGTH};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use super::CloudClient;
use crate::util::encode;

#[derive(Deserialize)]
struct ApiError {
	message: String,
}

#[derive(Deserialize)]
struct List<T> {
	items: Vec<T>,
}

#[derive(Deserialize)]
pub struct Region {
	pub region: String,
}

#[derive(Deserialize)]
pub struct CreatedBuild {
	pub build_id: String,
	pub upload_url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
	/// Waiting for the tar at the upload URL.
	Uploading,
	/// Completion was requested and the upload is being checked.
	Completing,
	/// The upload is verified and its runners' boot image is being prepared.
	Preparing,
	/// The boot image is ready, so the build can be deployed.
	Ready,
	/// The build can never be deployed, and `failure` says why.
	Failed,
	/// The upload URL expired before the tar arrived.
	Expired,
	Deleting,
}

/// Who is at fault for a build failure.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fault {
	/// The build itself, such as an upload that does not match or an image that
	/// does not load.
	User,
	/// The platform, such as no capacity to prepare the image.
	Platform,
}

impl Fault {
	pub fn as_str(self) -> &'static str {
		match self {
			Fault::User => "user",
			Fault::Platform => "platform",
		}
	}
}

#[derive(Deserialize)]
pub struct BuildFailure {
	pub fault: Fault,
	pub code: String,
	pub message: String,
}

#[derive(Deserialize)]
pub struct Build {
	pub status: BuildStatus,
	/// Set on a failed build, and on a preparing build whose preparation is
	/// being retried after a platform failure.
	#[serde(default)]
	pub failure: Option<BuildFailure>,
}

/// The result of a runner pool upsert.
pub enum PoolUpsert {
	Applied,
	/// The Cloud API refused the change with a conflict, such as a build that is
	/// not ready yet or a cluster that is being deleted.
	Conflict(String),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
	Provisioning,
	Ready,
	Updating,
	Degraded,
	Down,
	Deleting,
}

#[derive(Deserialize)]
pub struct Status {
	pub state: State,
	/// What is wrong or what is changing, as pallet reports it. Null while ready.
	pub message: Option<String>,
	/// The user must act on `message`, such as fixing the deployed build.
	pub action_needed: bool,
	/// The pool's machines. Null on the cluster.
	pub replicas: Option<Replicas>,
}

#[derive(Clone, Copy, Deserialize)]
pub struct Replicas {
	pub desired: u64,
	pub ready: u64,
	pub updated: u64,
}

impl Status {
	/// Whether the pool runs its build on every machine it wants. A pool can
	/// read `ready` before its machines are on the new build, so the state alone
	/// does not mean a rollout finished.
	pub fn rolled_out(&self) -> bool {
		match (self.state, self.replicas) {
			(State::Ready, Some(replicas)) => {
				replicas.updated >= replicas.desired && replicas.ready >= replicas.desired
			}
			(State::Ready, None)
			| (State::Provisioning, _)
			| (State::Updating, _)
			| (State::Degraded, _)
			| (State::Down, _)
			| (State::Deleting, _) => false,
		}
	}
}

#[derive(Deserialize)]
pub struct RunnerPool {
	pub build_id: Option<String>,
	pub cidr_allowlist: Vec<String>,
	pub endpoints: Endpoints,
	pub status: Status,
}

#[derive(Deserialize)]
pub struct Endpoints {
	/// The pool's public URL. Null until the cluster assigns one.
	pub public: Option<String>,
}

async fn send(
	cloud: &CloudClient,
	method: Method,
	path: &str,
	body: Option<Value>,
) -> Result<(StatusCode, String)> {
	let response = cloud.send(method, path, body).await?;
	let status = response.status();
	let text = response.text().await.unwrap_or_default();
	Ok((status, text))
}

fn decode<T: DeserializeOwned>(path: &str, status: StatusCode, text: &str) -> Result<T> {
	if !status.is_success() {
		match serde_json::from_str::<ApiError>(text) {
			Ok(error) => bail!("{}", error.message),
			Err(_) => bail!("Cloud API error {status}: {text}"),
		}
	}
	serde_json::from_str(text)
		.with_context(|| format!("Cloud API returned invalid JSON for {path}"))
}

async fn request<T: DeserializeOwned>(
	cloud: &CloudClient,
	method: Method,
	path: &str,
	body: Option<Value>,
) -> Result<T> {
	let (status, text) = send(cloud, method, path, body).await?;
	decode(path, status, &text)
}

fn build_path(cluster: &str, build_id: &str) -> String {
	format!(
		"/v2/clusters/{}/builds/{}",
		encode(cluster),
		encode(build_id)
	)
}

fn runner_pool_path(cluster: &str, region: &str, pool: &str) -> String {
	format!(
		"/v2/clusters/{}/regions/{}/runner-pools/{}",
		encode(cluster),
		encode(region),
		encode(pool)
	)
}

pub async fn list_regions(cloud: &CloudClient, cluster: &str) -> Result<Vec<Region>> {
	let path = format!("/v2/clusters/{}/regions", encode(cluster));
	let list: List<Region> = request(cloud, Method::GET, &path, None).await?;
	Ok(list.items)
}

pub async fn create_build(
	cloud: &CloudClient,
	cluster: &str,
	image: &str,
	content_length: u64,
	content_md5: &str,
) -> Result<Option<CreatedBuild>> {
	let path = format!("/v2/clusters/{}/builds", encode(cluster));
	let (status, text) = send(
		cloud,
		Method::POST,
		&path,
		Some(json!({
			"image": image,
			"content_length": content_length,
			"content_md5": content_md5,
		})),
	)
	.await?;
	// A new cluster refuses builds until its bucket is ready.
	if status == StatusCode::CONFLICT {
		return Ok(None);
	}
	decode(&path, status, &text).map(Some)
}

/// The upload URL is presigned, so it is sent without the Cloud API token.
pub async fn upload_build(
	cloud: &CloudClient,
	upload_url: &str,
	tar: &Path,
	content_length: u64,
	content_md5: &str,
) -> Result<()> {
	let file = tokio::fs::File::open(tar)
		.await
		.with_context(|| format!("open {}", tar.display()))?;
	let response = cloud
		.http
		.put(upload_url)
		.header(CONTENT_LENGTH, content_length)
		.header("Content-MD5", content_md5)
		.body(reqwest::Body::from(file))
		.send()
		.await
		.context("build upload failed")?;
	let status = response.status();
	if !status.is_success() {
		let text = response.text().await.unwrap_or_default();
		bail!("build upload failed with {status}: {text}");
	}
	Ok(())
}

pub async fn complete_build(cloud: &CloudClient, cluster: &str, build_id: &str) -> Result<Build> {
	let path = format!("{}/complete", build_path(cluster, build_id));
	request(cloud, Method::POST, &path, None).await
}

pub async fn get_build(cloud: &CloudClient, cluster: &str, build_id: &str) -> Result<Build> {
	request(cloud, Method::GET, &build_path(cluster, build_id), None).await
}

/// Returns `None` when the pool does not exist. The caller must have resolved
/// the cluster and region already, since those are also reported as not found.
pub async fn find_runner_pool(
	cloud: &CloudClient,
	cluster: &str,
	region: &str,
	pool: &str,
) -> Result<Option<RunnerPool>> {
	let path = runner_pool_path(cluster, region, pool);
	let (status, text) = send(cloud, Method::GET, &path, None).await?;
	if status == StatusCode::NOT_FOUND {
		return Ok(None);
	}
	decode(&path, status, &text).map(Some)
}

pub async fn put_runner_pool(
	cloud: &CloudClient,
	cluster: &str,
	region: &str,
	pool: &str,
	build_id: &str,
	cidr_allowlist: &[String],
) -> Result<PoolUpsert> {
	let path = runner_pool_path(cluster, region, pool);
	let (status, text) = send(
		cloud,
		Method::PUT,
		&path,
		Some(json!({
			"build_id": build_id,
			"cidr_allowlist": cidr_allowlist,
		})),
	)
	.await?;
	if status == StatusCode::CONFLICT {
		let message = serde_json::from_str::<ApiError>(&text)
			.map(|error| error.message)
			.unwrap_or(text);
		return Ok(PoolUpsert::Conflict(message));
	}
	decode::<RunnerPool>(&path, status, &text)?;
	Ok(PoolUpsert::Applied)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LogLine {
	/// Ordered by time; a cursor for `before` and `after`.
	pub id: String,
	/// ISO 8601 in UTC, in microseconds, so it sorts as text.
	pub timestamp: String,
	pub severity: Option<String>,
	pub message: String,
	pub source: String,
	pub runner_pool: Option<String>,
	pub instance_id: String,
}

#[derive(Deserialize)]
pub struct LogPage {
	/// Oldest first.
	pub items: Vec<LogLine>,
	pub has_more: bool,
}

#[derive(Default)]
pub struct LogQuery<'a> {
	pub source: Option<&'a str>,
	pub runner_pool: Option<&'a str>,
	pub contains: Option<&'a str>,
	pub min_severity: Option<&'a str>,
	pub before: Option<&'a str>,
	pub after: Option<&'a str>,
	pub limit: Option<u32>,
}

pub async fn list_logs(
	cloud: &CloudClient,
	cluster: &str,
	query: &LogQuery<'_>,
) -> Result<LogPage> {
	let mut params = Vec::new();
	for (key, value) in [
		("source", query.source),
		("runner_pool", query.runner_pool),
		("contains", query.contains),
		("min_severity", query.min_severity),
		("before", query.before),
		("after", query.after),
	] {
		if let Some(value) = value {
			params.push(format!("{key}={}", encode(value)));
		}
	}
	if let Some(limit) = query.limit {
		params.push(format!("limit={limit}"));
	}
	let mut path = format!("/v2/clusters/{}/logs", encode(cluster));
	if !params.is_empty() {
		path.push('?');
		path.push_str(&params.join("&"));
	}
	request(cloud, Method::GET, &path, None).await
}
