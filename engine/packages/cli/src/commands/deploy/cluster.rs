use std::{
	fs::File,
	io::Read,
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use tokio::time::{Instant, sleep};

use crate::{
	cloud::{
		CloudClient,
		v2::{self, BuildFailure, BuildStatus, PoolUpsert, State},
	},
	util::{default_image_tag, docker_build, run_command},
};

/// The largest tar the Cloud API accepts, since a build uploads in one PUT.
const MAX_BUILD_BYTES: u64 = 5 * 1024 * 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// How long a build may take to verify and prepare. Preparing its boot image
/// usually takes about 3 minutes, and the platform gives up on it after 30.
const BUILD_TIMEOUT: Duration = Duration::from_secs(35 * 60);
const BUILD_POLL_INTERVAL: Duration = Duration::from_secs(5);
/// How often a build that stays in one step is reported as still waiting.
const BUILD_PROGRESS_INTERVAL: Duration = Duration::from_secs(30);
const ROLLOUT_POLL_ATTEMPTS: u32 = 450;
/// How long a new cluster may refuse builds while its bucket comes up.
const UPLOAD_READY_INTERVAL: Duration = Duration::from_secs(10);
const UPLOAD_READY_ATTEMPTS: u32 = 60;

pub struct ClusterDeploy {
	pub cluster: String,
	pub pool: String,
	pub region: Option<String>,
	pub build: Option<String>,
	pub dockerfile: PathBuf,
	pub build_context: PathBuf,
	pub image: Option<String>,
	pub tag: Option<String>,
}

pub async fn deploy(cloud: &CloudClient, opts: ClusterDeploy) -> Result<()> {
	let cluster = opts.cluster.as_str();
	let pool = opts.pool.as_str();
	let region = resolve_region(cloud, cluster, opts.region).await?;

	let build_id = match opts.build {
		Some(build_id) => build_id,
		None => {
			let image_ref = format!(
				"{}:{}",
				opts.image.as_deref().unwrap_or(cluster),
				opts.tag.unwrap_or_else(default_image_tag)
			);
			upload_image(
				cloud,
				cluster,
				&opts.build_context,
				&opts.dockerfile,
				&image_ref,
			)
			.await?
		}
	};

	// A pool refuses a build until its boot image is ready, including one passed
	// with --build that is still being prepared.
	wait_for_build(cloud, cluster, &build_id).await?;

	// The upsert replaces the allowlist, so an existing pool's is sent back.
	let cidr_allowlist = v2::find_runner_pool(cloud, cluster, &region, pool)
		.await?
		.map(|pool| pool.cidr_allowlist)
		.unwrap_or_default();

	tracing::info!(%cluster, %region, %pool, %build_id, "rolling out build");
	upsert_runner_pool(cloud, cluster, &region, pool, &build_id, &cidr_allowlist).await?;
	wait_for_rollout(cloud, cluster, &region, pool, &build_id).await?;
	if let Some(url) = v2::find_runner_pool(cloud, cluster, &region, pool)
		.await?
		.and_then(|pool| pool.endpoints.public)
	{
		tracing::info!(%pool, %url, "deployed; the pool serves the app's PORT at this URL");
	}

	// The build id is the command's result; print it to stdout so it can be
	// captured by scripts and passed back as `--build`.
	println!("{build_id}");
	Ok(())
}

/// Creates the build, waiting while a new cluster refuses uploads because its
/// bucket is not ready yet.
async fn create_build_when_ready(
	cloud: &CloudClient,
	cluster: &str,
	image: &str,
	content_length: u64,
	content_md5: &str,
) -> Result<v2::CreatedBuild> {
	for _ in 0..UPLOAD_READY_ATTEMPTS {
		if let Some(build) =
			v2::create_build(cloud, cluster, image, content_length, content_md5).await?
		{
			return Ok(build);
		}
		tracing::info!(%cluster, "cluster is not ready for uploads yet; retrying");
		sleep(UPLOAD_READY_INTERVAL).await;
	}
	bail!("cluster {cluster} did not become ready for uploads; check its status in the dashboard")
}

async fn resolve_region(
	cloud: &CloudClient,
	cluster: &str,
	region: Option<String>,
) -> Result<String> {
	if let Some(region) = region {
		return Ok(region);
	}
	let mut regions = v2::list_regions(cloud, cluster).await?;
	match regions.len() {
		0 => bail!("cluster {cluster} has no regions; add one before deploying"),
		1 => Ok(regions.swap_remove(0).region),
		_ => bail!(
			"cluster {cluster} has several regions; pass --region with one of: {}",
			regions
				.iter()
				.map(|region| region.region.as_str())
				.collect::<Vec<_>>()
				.join(", ")
		),
	}
}

async fn upload_image(
	cloud: &CloudClient,
	cluster: &str,
	build_context: &Path,
	dockerfile: &Path,
	image_ref: &str,
) -> Result<String> {
	tracing::info!("building Docker image");
	docker_build(build_context, dockerfile, image_ref)?;

	let dir = tempfile::tempdir().context("create temporary directory for the image tar")?;
	let tar = dir.path().join("image.tar");
	tracing::info!("saving Docker image");
	run_command(
		"docker",
		&["image", "save", image_ref, "-o", &tar.to_string_lossy()],
		None,
	)?;

	let digest_tar = tar.clone();
	let (content_length, content_md5) = tokio::task::spawn_blocking(move || digest(&digest_tar))
		.await
		.context("image digest task failed")??;
	if content_length > MAX_BUILD_BYTES {
		bail!(
			"image tar is {content_length} bytes, over the {MAX_BUILD_BYTES} byte build limit; reduce the image size"
		);
	}

	let build =
		create_build_when_ready(cloud, cluster, image_ref, content_length, &content_md5).await?;
	tracing::info!(build_id = %build.build_id, content_length, "uploading build");
	v2::upload_build(cloud, &build.upload_url, &tar, content_length, &content_md5).await?;
	v2::complete_build(cloud, cluster, &build.build_id).await?;
	Ok(build.build_id)
}

/// Returns the file's size and its base64 MD5 digest, as `Content-MD5` carries it.
fn digest(path: &Path) -> Result<(u64, String)> {
	let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
	let mut context = md5::Context::new();
	let mut buffer = vec![0u8; 1024 * 1024];
	let mut length = 0u64;
	loop {
		let read = file
			.read(&mut buffer)
			.with_context(|| format!("read {}", path.display()))?;
		if read == 0 {
			break;
		}
		context.consume(&buffer[..read]);
		length += read as u64;
	}
	Ok((length, STANDARD.encode(context.compute().0)))
}

/// Sets the pool's build. A conflict on a build that is not ready yet waits for
/// it and tries once more; any other conflict is the caller's to fix.
async fn upsert_runner_pool(
	cloud: &CloudClient,
	cluster: &str,
	region: &str,
	pool: &str,
	build_id: &str,
	cidr_allowlist: &[String],
) -> Result<()> {
	let message =
		match v2::put_runner_pool(cloud, cluster, region, pool, build_id, cidr_allowlist).await? {
			PoolUpsert::Applied => return Ok(()),
			PoolUpsert::Conflict(message) => message,
		};
	match v2::get_build(cloud, cluster, build_id).await?.status {
		BuildStatus::Ready | BuildStatus::Failed | BuildStatus::Expired | BuildStatus::Deleting => {
			bail!("runner pool {pool} refused build {build_id}: {message}")
		}
		BuildStatus::Uploading | BuildStatus::Completing | BuildStatus::Preparing => {
			tracing::info!(%pool, %build_id, %message, "build is not ready yet; waiting for it");
		}
	}
	wait_for_build(cloud, cluster, build_id).await?;
	match v2::put_runner_pool(cloud, cluster, region, pool, build_id, cidr_allowlist).await? {
		PoolUpsert::Applied => Ok(()),
		PoolUpsert::Conflict(message) => {
			bail!("runner pool {pool} refused build {build_id}: {message}")
		}
	}
}

/// Polls the build through `completing` and `preparing` until it is ready.
async fn wait_for_build(cloud: &CloudClient, cluster: &str, build_id: &str) -> Result<()> {
	let started = Instant::now();
	let deadline = started + BUILD_TIMEOUT;
	let mut last_status: Option<BuildStatus> = None;
	let mut last_report = started;
	let mut last_retry: Option<String> = None;
	loop {
		let build = v2::get_build(cloud, cluster, build_id).await?;
		let step = match build.status {
			BuildStatus::Ready => {
				if last_status.is_some() {
					tracing::info!(%build_id, elapsed_s = started.elapsed().as_secs(), "build is ready");
				}
				return Ok(());
			}
			BuildStatus::Failed => bail!("{}", failure_message(build_id, build.failure.as_ref())),
			BuildStatus::Expired => bail!(
				"build {build_id} expired before the upload arrived; deploy again to create a new build"
			),
			BuildStatus::Deleting => bail!("build {build_id} was deleted before it became ready"),
			BuildStatus::Uploading => "waiting for the upload",
			BuildStatus::Completing => "verifying the upload",
			BuildStatus::Preparing => {
				"preparing the boot image, which usually takes about 3 minutes"
			}
		};

		// A preparing build carries the platform failure it is retrying.
		let retry = build.failure.map(|failure| failure.message);
		if retry != last_retry {
			if let Some(message) = &retry {
				tracing::warn!(%build_id, %message, "preparing the build failed on the platform side; retrying");
			}
			last_retry = retry;
		}

		let now = Instant::now();
		if last_status != Some(build.status)
			|| now.duration_since(last_report) >= BUILD_PROGRESS_INTERVAL
		{
			tracing::info!(%build_id, step, elapsed_s = started.elapsed().as_secs(), "build in progress");
			last_status = Some(build.status);
			last_report = now;
		}

		if now + BUILD_POLL_INTERVAL > deadline {
			bail!(
				"timed out after {} minutes waiting for build {build_id} to become ready; check it in the dashboard",
				BUILD_TIMEOUT.as_secs() / 60
			);
		}
		sleep(BUILD_POLL_INTERVAL).await;
	}
}

/// Says why a build failed and whose fault it was, so a platform failure is not
/// mistaken for a problem with the image.
fn failure_message(build_id: &str, failure: Option<&BuildFailure>) -> String {
	let Some(failure) = failure else {
		return format!("build {build_id} failed; deploy again to create a new build");
	};
	let advice = match failure.fault {
		v2::Fault::User => "fix the image and deploy again",
		v2::Fault::Platform => "this is a platform problem, not your image; deploy again to retry",
	};
	format!(
		"build {build_id} failed ({} fault, {}): {}; {advice}",
		failure.fault.as_str(),
		failure.code,
		failure.message
	)
}

async fn wait_for_rollout(
	cloud: &CloudClient,
	cluster: &str,
	region: &str,
	pool: &str,
	build_id: &str,
) -> Result<()> {
	let mut last_message: Option<String> = None;
	for _ in 0..ROLLOUT_POLL_ATTEMPTS {
		let current = v2::find_runner_pool(cloud, cluster, region, pool)
			.await?
			.with_context(|| format!("runner pool {pool} disappeared while rolling out"))?;
		if current.build_id.as_deref() != Some(build_id) {
			bail!("runner pool {pool} was moved to another build while rolling out");
		}
		let status = current.status;
		if status.rolled_out() {
			return Ok(());
		}
		match status.state {
			State::Deleting => bail!("runner pool {pool} is being deleted"),
			State::Degraded | State::Down if status.action_needed => bail!(
				"runner pool {pool} failed: {}",
				status
					.message
					.as_deref()
					.unwrap_or("the deployed build failed")
			),
			State::Ready
			| State::Provisioning
			| State::Updating
			| State::Degraded
			| State::Down => {
				if status.message != last_message {
					if let Some(message) = &status.message {
						tracing::info!(%pool, %message, "rollout in progress");
					}
					last_message = status.message;
				}
				sleep(POLL_INTERVAL).await;
			}
		}
	}
	bail!("timed out waiting for runner pool {pool} to roll out build {build_id}")
}


