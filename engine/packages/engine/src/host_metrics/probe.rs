//! A short, bounded write test of one data directory. It times small appends each followed
//! by an fdatasync, then a sequential write ending in an fsync, in a scratch file it always
//! removes. Results are telemetry only and never steer engine behavior.

use anyhow::{Context, Result};
use rivet_env::{RngStream, fs::Fs};
use std::{io::ErrorKind, path::Path, time::Duration};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

/// Scratch file name inside the probed directory. Storage code ignores root-level files
/// it does not own.
pub const PROBE_FILE: &str = ".rivet-disk-probe";

/// Appends timed in the fsync phase.
pub const FSYNC_COUNT: usize = 100;

/// Bytes of each timed append.
pub const FSYNC_BYTES: usize = 4096;

/// Largest single write of the sequential phase, and the probe's only buffer.
const CHUNK_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct ProbeResult {
	pub write_bytes_per_second: f64,
	/// Latency of every append plus fdatasync, sorted ascending.
	pub fsync_seconds: Vec<f64>,
}

impl ProbeResult {
	/// Nearest-rank quantile of the fsync latencies; `1.0` is the maximum.
	pub fn fsync_quantile(&self, quantile: f64) -> f64 {
		let Some(last) = self.fsync_seconds.len().checked_sub(1) else {
			return f64::NAN;
		};
		let rank = (quantile * self.fsync_seconds.len() as f64).ceil() as usize;
		self.fsync_seconds[rank.saturating_sub(1).min(last)]
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProbeOutcome {
	Finished(ProbeResult),
	Cancelled,
	/// The filesystem lacks room for the probe plus an equal margin.
	SkippedLowSpace {
		available_bytes: u64,
	},
}

/// Bytes the probe writes at most for a sequential size: the sequential phase plus every
/// timed append.
pub fn max_written_bytes(sequential_bytes: u64) -> u64 {
	sequential_bytes.saturating_add((FSYNC_COUNT * FSYNC_BYTES) as u64)
}

/// Probes `directory`, stopping between operations once `cancel` fires. The scratch file
/// is removed on every exit after its creation, including cancellation and I/O errors.
pub async fn probe(
	fs: &Fs,
	rng: &mut RngStream,
	directory: &Path,
	sequential_bytes: u64,
	cancel: &CancellationToken,
) -> Result<ProbeOutcome> {
	let space = fs
		.disk_space(directory)
		.await
		.with_context(|| format!("failed to read free space of {}", directory.display()))?;
	let needed = max_written_bytes(sequential_bytes).saturating_mul(2);
	if space.available_bytes < needed {
		return Ok(ProbeOutcome::SkippedLowSpace {
			available_bytes: space.available_bytes,
		});
	}

	let path = directory.join(PROBE_FILE);
	let result = probe_file(fs, rng, &path, sequential_bytes, cancel).await;
	match fs.remove_file(&path).await {
		Ok(()) => {}
		Err(error) if error.kind() == ErrorKind::NotFound => {}
		Err(error) => {
			tracing::warn!(%error, path = %path.display(), "failed to remove disk probe file; delete it by hand");
		}
	}
	result
}

/// Telemetry-only elapsed time. Metrics use real time even under simulation.
#[allow(clippy::disallowed_methods, reason = "telemetry-only clock read")]
fn now() -> std::time::Instant {
	std::time::Instant::now()
}

/// Awaits one I/O operation unless `cancel` fires first. `None` means cancelled.
async fn unless_cancelled<T>(
	cancel: &CancellationToken,
	operation: impl std::future::Future<Output = std::io::Result<T>>,
) -> Result<Option<T>> {
	rivet_env::select_biased!("cancellation preempts probe I/O",
		_ = cancel.cancelled() => Ok(None),
		result = operation => Ok(Some(result?)),
	)
}

async fn probe_file(
	fs: &Fs,
	rng: &mut RngStream,
	path: &Path,
	sequential_bytes: u64,
	cancel: &CancellationToken,
) -> Result<ProbeOutcome> {
	if cancel.is_cancelled() {
		return Ok(ProbeOutcome::Cancelled);
	}
	let mut file = fs
		.open_options()
		.write(true)
		.create(true)
		.truncate(true)
		.open(path)
		.await
		.with_context(|| format!("failed to create {}", path.display()))?;

	// Random bytes keep compressing filesystems from shrinking the write.
	let chunk_len = usize::try_from(sequential_bytes)
		.unwrap_or(CHUNK_BYTES)
		.clamp(FSYNC_BYTES, CHUNK_BYTES);
	let chunk = random_bytes(rng, chunk_len);

	// Fsync phase: each append grows the file, so the sync also persists its size, as a
	// log append does.
	let mut fsync_seconds = Vec::with_capacity(FSYNC_COUNT);
	for _ in 0..FSYNC_COUNT {
		let started = now();
		let append = async {
			file.write_all(&chunk[..FSYNC_BYTES]).await?;
			file.flush().await?;
			file.sync_data().await
		};
		if unless_cancelled(cancel, append).await?.is_none() {
			return Ok(ProbeOutcome::Cancelled);
		}
		fsync_seconds.push(now().duration_since(started).as_secs_f64());
	}
	fsync_seconds.sort_by(f64::total_cmp);

	// Sequential phase over an emptied file, so its bytes are new allocations.
	let reset = async {
		file.set_len(0).await?;
		file.seek(std::io::SeekFrom::Start(0)).await.map(|_| ())
	};
	if unless_cancelled(cancel, reset).await?.is_none() {
		return Ok(ProbeOutcome::Cancelled);
	}
	let started = now();
	let mut written = 0u64;
	while written < sequential_bytes {
		let remaining = sequential_bytes - written;
		let len = usize::try_from(remaining)
			.unwrap_or(chunk.len())
			.min(chunk.len());
		if unless_cancelled(cancel, file.write_all(&chunk[..len]))
			.await?
			.is_none()
		{
			return Ok(ProbeOutcome::Cancelled);
		}
		written += len as u64;
	}
	let finish = async {
		file.flush().await?;
		file.sync_all().await
	};
	if unless_cancelled(cancel, finish).await?.is_none() {
		return Ok(ProbeOutcome::Cancelled);
	}
	let elapsed = now().duration_since(started).max(Duration::from_nanos(1));

	Ok(ProbeOutcome::Finished(ProbeResult {
		write_bytes_per_second: written as f64 / elapsed.as_secs_f64(),
		fsync_seconds,
	}))
}

/// Fills a buffer from a xorshift generator seeded by `rng`.
fn random_bytes(rng: &mut RngStream, len: usize) -> Vec<u8> {
	let mut state = rng.gen_range(1..u64::MAX);
	let mut bytes = Vec::with_capacity(len + 8);
	while bytes.len() < len {
		state ^= state << 13;
		state ^= state >> 7;
		state ^= state << 17;
		bytes.extend_from_slice(&state.to_le_bytes());
	}
	bytes.truncate(len);
	bytes
}
