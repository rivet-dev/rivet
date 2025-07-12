use std::{
	collections::{hash_map::Entry, HashMap},
	hash::{DefaultHasher, Hasher},
	io::ErrorKind,
	result::Result::Ok,
	sync::Arc,
	time::Instant,
};

use anyhow::*;
use indoc::indoc;
use pegboard::protocol;
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha12Rng;
use sqlx::Acquire;
use tokio::{fs, process::Command, sync::Mutex};
use url::Url;
use uuid::Uuid;

use crate::{metrics, pull_addr_handler::PullAddrHandler, utils, Ctx};

/// Handles downloading images by queuing downloads of the same image together and reading from cache if
/// it exists.
pub struct ImageDownloadHandler {
	pull_addr_handler: PullAddrHandler,
	downloads: Mutex<HashMap<Uuid, Arc<Mutex<bool>>>>,
}

impl ImageDownloadHandler {
	pub fn new() -> Self {
		ImageDownloadHandler {
			pull_addr_handler: PullAddrHandler::new(),
			downloads: Mutex::new(HashMap::new()),
		}
	}

	pub async fn download(&self, ctx: &Ctx, image_config: &protocol::Image) -> Result<()> {
		metrics::IMAGE_DOWNLOAD_REQUEST_TOTAL.inc();

		let mut downloads_guard = self.downloads.lock().await;

		let entry_lock = match downloads_guard.entry(image_config.id) {
			// The image download started at some point in the current runtime
			Entry::Occupied(entry) => entry.get().clone(),
			// The image is not currently being downloaded
			Entry::Vacant(entry) => entry.insert(Arc::new(Mutex::new(false))).clone(),
		};

		drop(downloads_guard);

		let mut entry_guard = entry_lock.lock().await;

		// The image was successfully downloaded before
		if *entry_guard {
			tracing::debug!(image_id=?image_config.id, "image already downloaded");

			drop(entry_guard);

			// Update LRU cache
			sqlx::query(indoc!(
				"
					UPDATE images_cache
					SET last_used_ts = ?2
					WHERE image_id = ?1
					",
			))
			.bind(image_config.id)
			.bind(utils::now())
			.execute(&mut *ctx.sql().await?)
			.await?;

			return Ok(());
		}

		// Check database for image
		let row = sqlx::query_as::<_, (i64,)>(indoc!(
			"
			SELECT 1
			FROM images_cache
			WHERE image_id = ?1 AND download_complete_ts IS NOT NULL
			",
		))
		.bind(image_config.id)
		.fetch_optional(&mut *ctx.sql().await?)
		.await?;

		// Image exists and is downloaded
		if row.is_some() {
			tracing::debug!(image_id=?image_config.id, "image already downloaded");

			*entry_guard = true;

			return Ok(());
		}

		// Image does not exist/wasn't fully downloaded and isn't currently downloading, continue
		metrics::IMAGE_DOWNLOAD_CACHE_MISS_TOTAL.inc();

		let start_instant = Instant::now();
		tracing::info!(image_id=?image_config.id, "downloading image");

		let image_path = ctx.image_path(image_config.id);

		// Clear any previous content and make image dir
		match fs::remove_dir_all(&image_path).await {
			Err(e) if e.kind() == ErrorKind::NotFound => {}
			res => res.context("failed to delete image dir")?,
		}
		fs::create_dir(&image_path)
			.await
			.context("failed to create image dir")?;

		// NOTE: Txn here so that we prune and insert the new image row at the same time. This ensures
		// if another image is downloading concurrently that it will calculate the correct images
		// dir size.
		let mut conn = ctx.sql().await?;
		let mut tx = conn.begin().await?;

		let ((cache_count, images_dir_size), image_download_size) = tokio::try_join!(
			async {
				// Get total size of images directory. Note that it doesn't matter if this doesn't
				// match the actual fs size because it should either be exactly at or below actual fs
				// size. Also calculating fs size manually is expensive.
				sqlx::query_as::<_, (i64, i64)>(indoc!(
					"
					SELECT COUNT(size), COALESCE(SUM(size), 0) FROM images_cache
					",
				))
				.fetch_one(&mut *tx)
				.await
				.map_err(Into::<anyhow::Error>::into)
			},
			// NOTE: The image size here is somewhat misleading because its only the size of the
			// downloaded archive and not the total disk usage after it is unpacked. However, this is
			// good enough
			self.fetch_image_download_size(ctx, image_config),
		)?;

		let rows = sqlx::query_as::<_, (Uuid, i64, Option<i64>, i64)>(indoc!(
			"
				SELECT
					ic.image_id,
					ic.size,
					ic.last_used_ts,
					SUM(ic.size)
						OVER (ORDER BY ic.last_used_ts ROWS UNBOUNDED PRECEDING)
						AS running_total
				FROM images_cache AS ic
				LEFT JOIN actors AS a
				-- Filter out images that are currently in use by actors
				ON
					ic.image_id = a.image_id AND
					a.stop_ts IS NULL AND
					a.exit_ts IS NULL
				WHERE
					-- Filter out current image, will be upserted
					ic.image_id != ?1 AND
					a.image_id IS NULL
				ORDER BY ic.last_used_ts
				",
		))
		.bind(image_config.id)
		.bind(
			(images_dir_size as u64)
				.saturating_add(image_download_size)
				.saturating_sub(ctx.config().images.max_cache_size()) as i64,
		)
		.fetch_all(&mut *tx)
		.await?;

		// Prune images
		let (removed_count, removed_bytes) = if images_dir_size as u64 + image_download_size
			> ctx.config().images.max_cache_size()
		{
			// Fetch as many images as it takes to clear up enough space for this new image.
			// Ordered by LRU
			let rows = sqlx::query_as::<_, (Uuid, i64)>(indoc!(
				"
				WITH
					cumulative_sizes AS (
						SELECT
							ic.image_id,
							ic.size,
							ic.last_used_ts,
							SUM(ic.size)
								OVER (ORDER BY ic.last_used_ts ROWS UNBOUNDED PRECEDING)
								AS running_total
						FROM images_cache AS ic
						LEFT JOIN actors AS a
						-- Filter out images that are currently in use by actors
						ON
							ic.image_id = a.image_id AND
							a.stop_ts IS NULL AND
							a.exit_ts IS NULL
						WHERE
							-- Filter out current image, will be upserted
							ic.image_id != ?1 AND
							a.image_id IS NULL
						ORDER BY ic.last_used_ts
					)
				DELETE FROM images_cache
				WHERE image_id IN (
					SELECT image_id
					FROM cumulative_sizes
					WHERE running_total - size < ?2
					ORDER BY last_used_ts
				)
				RETURNING image_id, size
				",
			))
			.bind(image_config.id)
			.bind(
				(images_dir_size as u64)
					.saturating_add(image_download_size)
					.saturating_sub(ctx.config().images.max_cache_size()) as i64,
			)
			.fetch_all(&mut *tx)
			.await?;

			let rows_len = rows.len();

			if rows.is_empty() {
				tracing::error!(
					image_id=?image_config.id,
					"no inactive images to delete to make space for new image, downloading anyway",
				);
			} else {
				tracing::debug!(image_id=?image_config.id, count=?rows_len, "cache full, clearing LRU entries");
			}

			let mut total_removed_bytes = 0;

			for (image_id, size) in rows {
				total_removed_bytes += size;

				// NOTE: The sql query does not return the current image id so there is no chance
				// for a deadlock here
				// Acquire lock on entry
				let mut downloads_guard = self.downloads.lock().await;

				match fs::remove_dir_all(ctx.image_path(image_id)).await {
					Err(e) if e.kind() == ErrorKind::NotFound => {}
					res => res.context("failed to delete image dir")?,
				}

				// Remove entry and release lock
				downloads_guard.remove(&image_id);
			}

			(rows_len as i64, total_removed_bytes as i64)
		} else {
			(0, 0)
		};

		metrics::IMAGE_CACHE_COUNT.set(cache_count + 1 - removed_count);
		metrics::IMAGE_CACHE_SIZE.set(images_dir_size + image_download_size as i64 - removed_bytes);

		sqlx::query(indoc!(
			"
			INSERT OR REPLACE INTO images_cache (image_id, size, last_used_ts, download_complete_ts)
			VALUES (?1, 0, ?2, NULL)
			",
		))
		.bind(image_config.id)
		.bind(utils::now())
		.execute(&mut *tx)
		.await?;

		tx.commit().await?;

		// Release lock on sqlite pool
		drop(conn);

		// Download image & compute size
		//
		// `image_size` is a slight over-estimate of the image size, since this is
		// counting the number of bytes read from the tar. This is fine since
		// over-estimating is safe for caching.
		let download_start_instant = Instant::now();
		let image_size = self.download_inner(ctx, image_config).await?;
		let download_duration = download_start_instant.elapsed().as_secs_f64();

		let convert_start_instant = Instant::now();
		self.convert(ctx, image_config).await?;
		let convert_duration = convert_start_instant.elapsed().as_secs_f64();

		// Update metrics after unpacking
		metrics::IMAGE_CACHE_SIZE.set(images_dir_size + image_size as i64 - removed_bytes);

		// Update state to signify download completed successfully
		sqlx::query(indoc!(
			"
			UPDATE images_cache
			SET
				download_complete_ts = ?2,
				size = ?3
			WHERE image_id = ?1
			",
		))
		.bind(image_config.id)
		.bind(utils::now())
		.bind(image_size as i64)
		.execute(&mut *ctx.sql().await?)
		.await?;

		let total_duration = start_instant.elapsed().as_secs_f64();
		crate::metrics::DOWNLOAD_IMAGE_DURATION.observe(total_duration);
		tracing::info!(
			total_duration,
			download_duration,
			convert_duration,
			"image download completed"
		);

		// The lock on entry is held until this point. After this any other parallel downloaders will
		// continue with the image already downloaded
		*entry_guard = true;

		Ok(())
	}

	async fn download_inner(&self, ctx: &Ctx, image_config: &protocol::Image) -> Result<u64> {
		let image_path = ctx.image_path(image_config.id);

		let addresses = self.get_image_addresses(ctx, image_config).await?;

		// Log the URLs we're attempting to download from
		tracing::info!(
			image_id=?image_config.id,
			addresses=?addresses,
			"initiating image download"
		);

		// Try each URL until one succeeds
		let mut last_error = None;
		for url in &addresses {
			tracing::info!(image_id=?image_config.id, ?url, "attempting download");

			// Build the shell command based on image kind and compression
			// Using shell commands with native Unix pipes improves performance by:
			// 1. Reducing overhead of passing data through Rust
			// 2. Letting the OS handle data transfer between processes efficiently
			// 3. Avoiding unnecessary buffer copies in memory
			let shell_cmd = match (image_config.kind, image_config.compression) {
				// Docker image, no compression
				(protocol::ImageKind::DockerImage, protocol::ImageCompression::None) => {
					let docker_image_path = image_path.join("docker-image.tar");
					tracing::info!(image_id=?image_config.id, "downloading uncompressed docker image using curl");

					// Use curl to download directly to file
					format!("curl -sSfL '{}' -o '{}'", url, docker_image_path.display())
				}

				// Docker image with LZ4 compression
				(protocol::ImageKind::DockerImage, protocol::ImageCompression::Lz4) => {
					let docker_image_path = image_path.join("docker-image.tar");
					tracing::info!(
						image_id=?image_config.id,
						"downloading and decompressing docker image using curl | lz4",
					);

					// Use curl piped to lz4 for decompression
					format!(
						"curl -sSfL '{}' | lz4 -d - '{}'",
						url,
						docker_image_path.display()
					)
				}

				// OCI Bundle or JavaScript with no compression
				(
					protocol::ImageKind::OciBundle | protocol::ImageKind::JavaScript,
					protocol::ImageCompression::None,
				) => {
					tracing::info!(
						image_id=?image_config.id,
						"downloading and unarchiving uncompressed artifact using curl | tar",
					);

					// Use curl piped to tar for extraction
					format!(
						"curl -sSfL '{}' | tar -x --totals -C '{}'",
						url,
						image_path.display()
					)
				}

				// OCI Bundle or JavaScript with LZ4 compression
				(
					protocol::ImageKind::OciBundle | protocol::ImageKind::JavaScript,
					protocol::ImageCompression::Lz4,
				) => {
					tracing::info!(
						image_id=?image_config.id,
						"downloading, decompressing, and unarchiving artifact using curl | lz4 | tar",
					);

					// Use curl piped to lz4 for decompression, then to tar for extraction
					format!(
						"curl -sSfL '{}' | lz4 -d | tar -x --totals -C '{}'",
						url,
						image_path.display()
					)
				}
			};

			// Execute the shell command
			// Use curl's built-in error handling to fail silently and let us try the next URL
			let cmd_result = Command::new("sh").arg("-c").arg(&shell_cmd).output().await;

			match cmd_result {
				Ok(output) if output.status.success() => {
					// Parse bytes read from tar to get dir size efficiently
					//
					// This is an over-estimate since the size on disk is smaller than the actual
					// tar
					let stderr = String::from_utf8_lossy(&output.stderr);
					let bytes_read = match parse_tar_total_bytes(&stderr) {
						Some(x) => x,
						None => {
							tracing::error!(%stderr, "failed to parse bytes read from tar output");
							bail!("failed to parse bytes read from tar output")
						}
					};

					tracing::info!(
						image_id=?image_config.id,
						?url,
						bytes_read=?bytes_read,
						"successfully downloaded image"
					);

					return Ok(bytes_read);
				}
				Ok(output) => {
					// Command ran but failed
					let stderr = String::from_utf8_lossy(&output.stderr);
					tracing::warn!(
						image_id=?image_config.id,
						?url,
						status=?output.status,
						stderr=%stderr,
						"failed to download image"
					);
					last_error = Some(anyhow!("download failed: {}", stderr));
				}
				Err(e) => {
					// Failed to execute command
					tracing::warn!(
						image_id=?image_config.id,
						?url,
						error=?e,
						"failed to execute download command"
					);
					last_error = Some(anyhow!("download command failed: {}", e));
				}
			}
		}

		// If we get here, all URLs failed
		Err(last_error
			.unwrap_or_else(|| anyhow!("failed to download image from any available URL")))
	}

	// Convert downloaded image to other formats (if needed)
	async fn convert(&self, ctx: &Ctx, image_config: &protocol::Image) -> Result<()> {
		let image_path = ctx.image_path(image_config.id);

		// We need to convert the Docker image to an OCI bundle in order to run it.
		// Allows us to work with the build with umoci
		if let protocol::ImageKind::DockerImage = image_config.kind {
			let docker_image_path = image_path.join("docker-image.tar");
			let oci_image_path = image_path.join("oci-image");

			tracing::info!("converting Docker image -> OCI image",);
			let conversion_start = Instant::now();
			let cmd_out = Command::new("skopeo")
				.arg("copy")
				.arg(format!("docker-archive:{}", docker_image_path.display()))
				.arg(format!("oci:{}:default", oci_image_path.display()))
				.output()
				.await?;
			ensure!(
				cmd_out.status.success(),
				"failed `skopeo` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);
			tracing::info!(
				duration_seconds = conversion_start.elapsed().as_secs_f64(),
				"docker to OCI conversion completed",
			);

			// Allows us to run the bundle natively with runc
			tracing::info!("converting OCI image -> OCI bundle");
			let unpack_start = Instant::now();
			let cmd_out = Command::new("umoci")
				.arg("unpack")
				.arg("--image")
				.arg(format!("{}:default", oci_image_path.display()))
				.arg(&image_path)
				.output()
				.await?;
			ensure!(
				cmd_out.status.success(),
				"failed `umoci` command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);
			tracing::info!(
				duration_seconds = unpack_start.elapsed().as_secs_f64(),
				"OCI image unpacking completed",
			);

			// Remove artifacts
			tracing::info!("cleaning up temporary image artifacts");
			tokio::try_join!(
				fs::remove_file(&docker_image_path),
				fs::remove_dir_all(&oci_image_path),
			)
			.context("failed to delete temporary image artifacts")?;
		}

		Ok(())
	}

	/// Generates a list of address URLs for a given build ID, with deterministic shuffling.
	///
	/// This function accepts a build ID and returns an array of URLs, including both
	/// the seeded shuffling and the fallback address (if provided).
	async fn get_image_addresses(
		&self,
		ctx: &Ctx,
		image_config: &protocol::Image,
	) -> Result<Vec<String>> {
		// Get hash from image id
		let mut hasher = DefaultHasher::new();
		hasher.write(image_config.id.as_bytes());
		let hash = hasher.finish();

		let mut rng = ChaCha12Rng::seed_from_u64(hash);

		// Shuffle based on hash
		let mut addresses = self
			.pull_addr_handler
			.addresses(ctx.config())
			.await?
			.iter()
			.map(|addr| {
				Ok(
					Url::parse(&format!("{addr}{}", image_config.artifact_url_stub))
						.context("failed to build artifact url")?
						.to_string(),
				)
			})
			.collect::<Result<Vec<_>>>()?;
		addresses.shuffle(&mut rng);

		// Add fallback url to the end if one is set
		if let Some(fallback_artifact_url) = &image_config.fallback_artifact_url {
			addresses.push(fallback_artifact_url.to_string());
		}

		ensure!(
			!addresses.is_empty(),
			"no artifact urls available (no pull addresses nor fallback)"
		);

		Ok(addresses)
	}

	/// Attempts to fetch HEAD for the image download url and determine the image's download size.
	async fn fetch_image_download_size(
		&self,
		ctx: &Ctx,
		image_config: &protocol::Image,
	) -> Result<u64> {
		// HEAD does not work if not using ATS
		if std::env::var("__HACK__DISABLE_FETCH_IMAGE_SIZE").map_or(false, |x| x == "1") {
			return Ok(0);
		}

		let addresses = self.get_image_addresses(ctx, image_config).await?;

		let mut iter = addresses.into_iter();
		while let Some(artifact_url) = iter.next() {
			// Log the full URL we're attempting to download from
			tracing::info!(image_id=?image_config.id, %artifact_url, "attempting to download image");

			match reqwest::Client::new()
				.head(&artifact_url)
				.send()
				.await
				.and_then(|res| res.error_for_status())
			{
				Ok(res) => {
					tracing::info!(image_id=?image_config.id, %artifact_url, "successfully fetched image HEAD");

					// Read Content-Length header from response
					let image_size = res
						.headers()
						.get(reqwest::header::CONTENT_LENGTH)
						.context("no Content-Length header")?
						.to_str()?
						.parse::<u64>()
						.context("invalid Content-Length header")?;

					return Ok(image_size);
				}
				Err(err) => {
					tracing::warn!(
						image_id=?image_config.id,
						%artifact_url,
						%err,
						"failed to fetch image HEAD"
					);
				}
			}
		}

		bail!("artifact url could not be resolved");
	}
}

/// Parses total bytes read from tar output.
fn parse_tar_total_bytes(stderr: &str) -> Option<u64> {
	// Example: "Total bytes read: 646737920 (617MiB, 339MiB/s)"
	for line in stderr.lines() {
		if line.starts_with("Total bytes read: ") {
			if let Some(bytes_str) = line.strip_prefix("Total bytes read: ") {
				if let Some(space_pos) = bytes_str.find(' ') {
					let bytes_part = &bytes_str[..space_pos];
					if let Ok(bytes) = bytes_part.parse::<u64>() {
						return Some(bytes);
					}
				}
			}
		}
	}
	None
}
