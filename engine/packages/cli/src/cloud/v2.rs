use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, bail};
use reqwest::{Method, StatusCode, header::CONTENT_LENGTH};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use super::CloudClient;
use crate::util::encode;

#[derive(Deserialize)]
struct ApiError {
	message: String,
	/// Request validation failures, by field where one is named.
	#[serde(default)]
	errors: Vec<FieldError>,
}

#[derive(Deserialize)]
struct FieldError {
	#[serde(default)]
	field: Option<String>,
	message: String,
}

impl ApiError {
	/// The message followed by each field error.
	fn describe(&self) -> String {
		let mut text = self.message.clone();
		for error in &self.errors {
			match &error.field {
				Some(field) => text.push_str(&format!("; {field}: {}", error.message)),
				None => text.push_str(&format!("; {}", error.message)),
			}
		}
		text
	}
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

/// One of a runner pool's variables: a string, or a cluster secret by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvValue {
	String(String),
	Secret(SecretRef),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
	pub name: String,
	/// Set while a rollback pinned the version the pool runs, until `env` is
	/// next sent. The Cloud API only accepts a pin resent exactly as read, and
	/// the CLI drops every pin when it sends `env`, so the pool takes the
	/// latest version.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pinned_version: Option<u64>,
}

#[derive(Deserialize)]
pub struct RunnerPool {
	pub build_id: Option<String>,
	pub cidr_allowlist: Vec<String>,
	#[serde(default)]
	pub env: BTreeMap<String, EnvValue>,
	pub endpoints: Endpoints,
	pub status: Status,
}

/// A runner pool as the cluster-wide list returns it, reduced to what the CLI
/// reads from it.
#[derive(Deserialize)]
pub struct RunnerPoolSummary {
	pub region: String,
	pub pool: String,
	#[serde(default)]
	pub env: BTreeMap<String, EnvValue>,
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
			Ok(error) => bail!("{}", error.describe()),
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

/// The cluster's region named by `region`, or its only one. A named region
/// is checked against the cluster's, so a typo is not reported as a missing
/// runner pool.
pub async fn resolve_region(
	cloud: &CloudClient,
	cluster: &str,
	region: Option<String>,
) -> Result<String> {
	let regions = list_regions(cloud, cluster)
		.await?
		.into_iter()
		.map(|region| region.region)
		.collect();
	pick_region(cluster, region, regions)
}

/// Picks `requested` from the cluster's `regions`, or the only one.
pub fn pick_region(
	cluster: &str,
	requested: Option<String>,
	mut regions: Vec<String>,
) -> Result<String> {
	if regions.is_empty() {
		bail!("cluster {cluster} has no regions; add one before deploying");
	}
	match requested {
		Some(region) => {
			if regions.contains(&region) {
				Ok(region)
			} else {
				bail!(
					"region {region} is not in cluster {cluster}; its regions are: {}",
					regions.join(", ")
				)
			}
		}
		None => {
			if regions.len() == 1 {
				Ok(regions.swap_remove(0))
			} else {
				bail!(
					"cluster {cluster} has several regions; pass --region with one of: {}",
					regions.join(", ")
				)
			}
		}
	}
}

/// Every runner pool in the cluster, across its regions.
pub async fn list_runner_pools(
	cloud: &CloudClient,
	cluster: &str,
) -> Result<Vec<RunnerPoolSummary>> {
	let path = format!("/v2/clusters/{}/runner-pools", encode(cluster));
	let list: List<RunnerPoolSummary> = request(cloud, Method::GET, &path, None).await?;
	Ok(list.items)
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

/// Rolls the pool out to `build_id`. `env`, when given, replaces the pool's
/// variables; left out they are kept.
pub async fn put_runner_pool(
	cloud: &CloudClient,
	cluster: &str,
	region: &str,
	pool: &str,
	build_id: &str,
	cidr_allowlist: &[String],
	env: Option<&BTreeMap<String, EnvValue>>,
) -> Result<PoolUpsert> {
	let path = runner_pool_path(cluster, region, pool);
	let mut body = json!({
		"build_id": build_id,
		"cidr_allowlist": cidr_allowlist,
	});
	if let Some(env) = env {
		body["env"] = serde_json::to_value(env).context("encode the pool's env")?;
	}
	let (status, text) = send(cloud, Method::PUT, &path, Some(body)).await?;
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

/// A cluster secret, without its value, which the Cloud API never returns.
#[derive(Serialize, Deserialize)]
pub struct Secret {
	pub name: String,
	pub latest_version: u64,
	pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct SecretRollout {
	pub name: String,
	pub latest_version: u64,
	/// False when the value equals the latest version's, which rolls nothing.
	pub changed: bool,
}

fn secrets_path(cluster: &str) -> String {
	format!("/v2/clusters/{}/secrets", encode(cluster))
}

pub async fn list_secrets(cloud: &CloudClient, cluster: &str) -> Result<Vec<Secret>> {
	let page: List<Secret> = request(cloud, Method::GET, &secrets_path(cluster), None).await?;
	Ok(page.items)
}

/// Stores `value` as the secret's next version and rolls every pool naming it.
pub async fn rollout_secret(
	cloud: &CloudClient,
	cluster: &str,
	name: &str,
	value: &str,
) -> Result<SecretRollout> {
	request(
		cloud,
		Method::POST,
		&format!("{}/rollout", secrets_path(cluster)),
		Some(json!({ "name": name, "value": value })),
	)
	.await
}

pub async fn delete_secret(cloud: &CloudClient, cluster: &str, name: &str) -> Result<()> {
	let path = format!("{}/{}", secrets_path(cluster), encode(name));
	let (status, text) = send(cloud, Method::DELETE, &path, None).await?;
	if !status.is_success() {
		decode::<Value>(&path, status, &text)?;
	}
	Ok(())
}

/// A BYOC build as the Cloud API returns it, reduced to what the CLI reads.
#[derive(Deserialize)]
pub struct ByocBuild {
	pub id: String,
	pub status: ByocBuildStatus,
	/// Why a `failed` build failed, such as `mismatch` for a tar that is not
	/// the declared size and MD5.
	#[serde(default)]
	pub failure: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ByocBuildStatus {
	Uploading,
	Ready,
	Failed,
	Expired,
}

/// Where and how the tar of a new BYOC build is uploaded. Both headers are
/// signed into the URL, so the upload sends exactly these.
#[derive(Deserialize)]
pub struct ByocUpload {
	pub url: String,
	pub method: String,
	pub headers: BTreeMap<String, String>,
}

#[derive(Deserialize)]
pub struct CreatedByocBuild {
	pub build: ByocBuild,
	pub upload: ByocUpload,
}

/// What a new BYOC build is registered with.
#[derive(Serialize)]
pub struct NewByocBuild<'a> {
	pub image: &'a str,
	pub architecture: &'a str,
	pub content_length: u64,
	/// Base64, as `Content-MD5` carries it.
	pub content_md5: &'a str,
	/// Lowercase hex.
	pub sha256: &'a str,
}

/// The pool a BYOC deploy applies. `env` and `secret_env`, when given,
/// replace the pool's. A field left out is never sent, and the cluster's
/// operator keeps the pool's current value, or for a new pool 1 vCPU, 2Gi,
/// 1 replica, `max_replicas` equal to `min_replicas` and no variables.
#[derive(Serialize)]
pub struct ByocPoolApply {
	pub build_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resources: Option<ByocResources>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_replicas: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_replicas: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub env: Option<BTreeMap<String, String>>,
	/// `NAME -> "<secret>/<key>"`, read from Kubernetes Secrets in the cluster.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub secret_env: Option<BTreeMap<String, String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pool_config: Option<serde_json::Map<String, Value>>,
}

/// Kubernetes quantities for each actor node. Only the sizes given are sent.
#[derive(Serialize)]
pub struct ByocResources {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cpu: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub memory: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ByocCommandStatus {
	Completed,
	Failed,
	/// The Cloud API gave up on the command; `timed_out` says why. A late
	/// report from the operator can still complete it.
	#[serde(rename = "timed_out")]
	TimedOut,
}

/// Why the Cloud API timed a command out, past its server-side limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByocTimeoutReason {
	/// The operator never took the command; queued since `since`.
	NotPickedUp,
	/// The operator took the command at `since` and never reported.
	NoReport,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
pub struct ByocTimedOut {
	pub reason: ByocTimeoutReason,
	pub since: String,
}

/// A command queued for a BYOC cluster's operator, reduced to what the CLI
/// reads while it waits on one.
#[derive(Deserialize)]
pub struct ByocCommand {
	pub idx: i64,
	/// Null until the operator reports the command.
	pub status: Option<ByocCommandStatus>,
	/// What the operator reported: `code`, a stable result code, and a few
	/// fields of data.
	#[serde(default)]
	pub result: Option<serde_json::Map<String, Value>>,
	/// When an operator received the command.
	#[serde(default)]
	pub seen_at: Option<String>,
	/// Set exactly when `status` is `timed_out`. Parsed but not shown: the
	/// user only learns that the command timed out.
	#[serde(default)]
	#[allow(dead_code)]
	pub timed_out: Option<ByocTimedOut>,
}

#[derive(Deserialize)]
struct WrappedByocCommand {
	command: ByocCommand,
}

#[derive(Deserialize)]
struct WrappedByocBuild {
	build: ByocBuild,
}

fn byoc_cluster_path(cluster: &str) -> String {
	format!("/v2/byoc/clusters/{}", encode(cluster))
}

/// Whether `cluster` is one of the organization's BYOC clusters. A managed
/// cluster is not found there, one whose name is not a DNS label is refused
/// as a bad request since every BYOC cluster name is one, and a credential
/// that may not read BYOC clusters is forbidden; all three mean the managed
/// path. A server error fails instead, because guessing managed could deploy
/// a BYOC cluster's build to the wrong place.
pub async fn is_byoc_cluster(cloud: &CloudClient, cluster: &str) -> Result<bool> {
	let path = byoc_cluster_path(cluster);
	let (status, text) = send(cloud, Method::GET, &path, None).await?;
	if status == StatusCode::NOT_FOUND
		|| status == StatusCode::BAD_REQUEST
		|| status == StatusCode::FORBIDDEN
	{
		return Ok(false);
	}
	if status.is_server_error() {
		bail!(
			"Rivet Cloud returned an internal server error (status {}) while checking the cluster; try again",
			status.as_u16()
		);
	}
	decode::<Value>(&path, status, &text)?;
	Ok(true)
}

pub async fn create_byoc_build(
	cloud: &CloudClient,
	cluster: &str,
	build: &NewByocBuild<'_>,
) -> Result<CreatedByocBuild> {
	let path = format!("{}/builds", byoc_cluster_path(cluster));
	let body = serde_json::to_value(build).context("encode the build")?;
	request(cloud, Method::POST, &path, Some(body)).await
}

/// PUTs the tar to the presigned URL with exactly the returned headers and
/// its length, and without the Cloud API token.
pub async fn upload_byoc_build(
	cloud: &CloudClient,
	upload: &ByocUpload,
	tar: &Path,
	content_length: u64,
) -> Result<()> {
	if upload.method != "PUT" {
		bail!(
			"the Cloud API asked for a build upload by {}; this CLI only uploads by PUT, update the CLI",
			upload.method
		);
	}
	let file = tokio::fs::File::open(tar)
		.await
		.with_context(|| format!("open {}", tar.display()))?;
	let mut request = cloud.http.put(&upload.url);
	for (name, value) in &upload.headers {
		request = request.header(name.as_str(), value.as_str());
	}
	let response = request
		.header(CONTENT_LENGTH, content_length)
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

pub async fn complete_byoc_build(
	cloud: &CloudClient,
	cluster: &str,
	build_id: &str,
) -> Result<ByocBuild> {
	let path = format!(
		"{}/builds/{}/complete",
		byoc_cluster_path(cluster),
		encode(build_id)
	);
	let wrapped: WrappedByocBuild = request(cloud, Method::POST, &path, None).await?;
	Ok(wrapped.build)
}

/// Queues an `actor_node_pool_apply` for the pool and returns the command.
pub async fn put_byoc_pool(
	cloud: &CloudClient,
	cluster: &str,
	pool: &str,
	apply: &ByocPoolApply,
) -> Result<ByocCommand> {
	let path = format!("{}/pools/{}", byoc_cluster_path(cluster), encode(pool));
	let body = serde_json::to_value(apply).context("encode the pool")?;
	let queued: WrappedByocCommand = request(cloud, Method::PUT, &path, Some(body)).await?;
	Ok(queued.command)
}

/// Reads one command of the cluster by its index.
pub async fn get_byoc_command(cloud: &CloudClient, cluster: &str, idx: i64) -> Result<ByocCommand> {
	let path = format!("{}/commands/{idx}", byoc_cluster_path(cluster));
	let read: WrappedByocCommand = request(cloud, Method::GET, &path, None).await?;
	Ok(read.command)
}
