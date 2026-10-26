//! Host hardware metrics, so load runs can tell engine hosts apart: CPU and memory at
//! startup, the filesystem and block device of each data directory, a background disk
//! probe of each, and periodic free-space and per-device I/O counters. Every read goes
//! through the injected `Fs` and `Host`; a failed read is logged and leaves its metric
//! unset, since none of this steers the engine.

pub mod metrics;
pub mod probe;

use rivet_config::config::depot::DepotConfig;
use rivet_env::{
	Env,
	fs::Fs,
	host::{DiskIo, Host},
};
use std::{
	collections::{BTreeMap, BTreeSet},
	path::PathBuf,
	sync::Arc,
};
use tokio_util::sync::CancellationToken;

/// How often free bytes of each data directory are re-read.
pub const FREE_SPACE_REFRESH_MS: u64 = 30_000;

/// How often per-device I/O counters are re-read.
pub const DISK_IO_REFRESH_MS: u64 = 10_000;

/// What a data directory holds. Its label names the role, never the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectoryRole {
	/// The Depot log shard.
	Log,
	/// SlateDB's local cache of cold-store objects.
	DiskCache,
}

impl DirectoryRole {
	pub fn label(self) -> &'static str {
		match self {
			Self::Log => "log",
			Self::DiskCache => "disk_cache",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataDirectory {
	pub role: DirectoryRole,
	pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Settings {
	pub directories: Vec<DataDirectory>,
	/// Sequential bytes of the disk probe, or `None` to skip it.
	pub probe_bytes: Option<u64>,
}

/// The local directories a storage worker writes, resolved as the disk budget resolves them.
pub fn data_directories(depot: &DepotConfig) -> Vec<DataDirectory> {
	let Some(log) = depot.log_directory.clone() else {
		return Vec::new();
	};
	let mut directories = vec![DataDirectory {
		role: DirectoryRole::Log,
		path: log.clone(),
	}];
	let cold_store = &depot.cold_store;
	if cold_store.disk_cache_bytes != Some(0) {
		let path = cold_store
			.disk_cache_directory
			.clone()
			.unwrap_or_else(|| log.join(depot_worker::disk_budget::DEFAULT_CACHE_DIRECTORY));
		directories.push(DataDirectory {
			role: DirectoryRole::DiskCache,
			path,
		});
	}
	directories
}

/// Publishes host facts, then probes and refreshes until `cancel` fires. Returns only after
/// the probe has stopped and removed its scratch file.
pub async fn run(
	env: Arc<dyn Env>,
	fs: Fs,
	host: Host,
	settings: Settings,
	cancel: CancellationToken,
) {
	report_cpu_and_memory(&host).await;
	let filesystems = report_directories(&fs, &host, &settings.directories).await;
	let devices = filesystems
		.iter()
		.filter_map(|placement| placement.device.clone())
		.collect::<BTreeSet<_>>();

	let probe = async {
		if let Some(bytes) = settings.probe_bytes {
			probe_directories(&env, &fs, &filesystems, bytes, &cancel).await;
		}
	};
	let refresh = async {
		rivet_env::select_biased!("cancellation stops refreshes",
			_ = cancel.cancelled() => {},
			_ = futures_util::future::join(
				refresh_free_space(&env, &fs, &settings.directories),
				refresh_disk_io(&env, &host, &devices),
			) => {},
		);
	};
	futures_util::future::join(probe, refresh).await;
}

async fn report_cpu_and_memory(host: &Host) {
	match host.cpu().await {
		Ok(cpu) => {
			metrics::CPU_LOGICAL.set(i64::from(cpu.logical_cpus));
			let model = cpu.model.as_deref().unwrap_or("unknown");
			metrics::INFO.with_label_values(&[model]).set(1);
			tracing::info!(
				logical_cpus = cpu.logical_cpus,
				cpu_model = model,
				"host cpu"
			);
		}
		Err(error) => tracing::warn!(%error, "cannot read host cpu; cpu metrics stay unset"),
	}
	match host.memory().await {
		Ok(memory) => {
			metrics::MEMORY_TOTAL.set(saturating_i64(memory.total_bytes));
			if let Some(limit) = memory.limit_bytes {
				metrics::MEMORY_LIMIT.set(saturating_i64(limit));
			}
			tracing::info!(total_bytes = memory.total_bytes, limit_bytes = ?memory.limit_bytes, "host memory");
		}
		Err(error) => tracing::warn!(%error, "cannot read host memory; memory metrics stay unset"),
	}
}

/// One data directory as found at startup.
struct Placement {
	directory: DataDirectory,
	filesystem_id: Option<u64>,
	device: Option<String>,
}

async fn report_directories(fs: &Fs, host: &Host, directories: &[DataDirectory]) -> Vec<Placement> {
	let mut placements = Vec::with_capacity(directories.len());
	for directory in directories {
		let role = directory.role.label();
		let filesystem_id = match fs.disk_space(&directory.path).await {
			Ok(space) => {
				metrics::DISK_TOTAL
					.with_label_values(&[role])
					.set(saturating_i64(space.total_bytes));
				metrics::DISK_FREE
					.with_label_values(&[role])
					.set(saturating_i64(space.available_bytes));
				Some(space.filesystem_id)
			}
			Err(error) => {
				tracing::warn!(%error, dir = role, path = %directory.path.display(), "cannot read data directory filesystem; check that it exists");
				None
			}
		};
		let device = match host.block_device(&directory.path).await {
			Ok(device) => device,
			Err(error) => {
				tracing::warn!(%error, dir = role, "cannot read the block device of a data directory");
				None
			}
		};
		let (name, rotational) = match &device {
			Some(device) => (
				device.name.as_str(),
				match device.rotational {
					Some(true) => "true",
					Some(false) => "false",
					None => "unknown",
				},
			),
			None => ("none", "unknown"),
		};
		metrics::DISK_INFO
			.with_label_values(&[role, name, rotational])
			.set(1);
		tracing::info!(dir = role, path = %directory.path.display(), device = name, rotational, "data directory");
		placements.push(Placement {
			directory: directory.clone(),
			filesystem_id,
			device: device.map(|device| device.name),
		});
	}
	placements
}

/// Probes each filesystem once; directories sharing a filesystem share its result.
async fn probe_directories(
	env: &Arc<dyn Env>,
	fs: &Fs,
	placements: &[Placement],
	bytes: u64,
	cancel: &CancellationToken,
) {
	let mut rng = env.rng("host_metrics.disk_probe");
	let mut results = BTreeMap::<u64, probe::ProbeResult>::new();
	for placement in placements {
		let Some(filesystem_id) = placement.filesystem_id else {
			continue;
		};
		let role = placement.directory.role.label();
		let result = match results.get(&filesystem_id) {
			Some(result) => result.clone(),
			None => {
				match probe::probe(fs, &mut rng, &placement.directory.path, bytes, cancel).await {
					Ok(probe::ProbeOutcome::Finished(result)) => {
						tracing::info!(
							dir = role,
							write_bytes_per_second = result.write_bytes_per_second,
							fsync_p50_seconds = result.fsync_quantile(0.5),
							fsync_p99_seconds = result.fsync_quantile(0.99),
							"disk probe finished"
						);
						results.insert(filesystem_id, result.clone());
						result
					}
					Ok(probe::ProbeOutcome::Cancelled) => return,
					Ok(probe::ProbeOutcome::SkippedLowSpace { available_bytes }) => {
						tracing::warn!(
							dir = role,
							available_bytes,
							probe_bytes = bytes,
							"skipped disk probe for lack of free space; lower metrics.hardware.disk_probe_bytes"
						);
						continue;
					}
					Err(error) => {
						tracing::warn!(?error, dir = role, "disk probe failed");
						continue;
					}
				}
			}
		};
		publish_probe(role, &result);
	}
}

fn publish_probe(role: &str, result: &probe::ProbeResult) {
	metrics::PROBE_WRITE_THROUGHPUT
		.with_label_values(&[role])
		.set(result.write_bytes_per_second);
	for (label, quantile) in [("0.5", 0.5), ("0.99", 0.99), ("1", 1.0)] {
		metrics::PROBE_FSYNC
			.with_label_values(&[role, label])
			.set(result.fsync_quantile(quantile));
	}
}

async fn refresh_free_space(env: &Arc<dyn Env>, fs: &Fs, directories: &[DataDirectory]) {
	if directories.is_empty() {
		return;
	}
	let mut interval = rivet_env::interval(env.clock(), FREE_SPACE_REFRESH_MS);
	// The first tick is immediate and startup already published free space.
	loop {
		if let Err(error) = interval.tick().await {
			tracing::warn!(%error, "clock failed; stopping free space refresh");
			return;
		}
		for directory in directories {
			match fs.disk_space(&directory.path).await {
				Ok(space) => metrics::DISK_FREE
					.with_label_values(&[directory.role.label()])
					.set(saturating_i64(space.available_bytes)),
				Err(error) => {
					tracing::debug!(%error, dir = directory.role.label(), "cannot refresh free space")
				}
			}
		}
	}
}

async fn refresh_disk_io(env: &Arc<dyn Env>, host: &Host, devices: &BTreeSet<String>) {
	if devices.is_empty() {
		return;
	}
	let mut previous = BTreeMap::<String, DiskIo>::new();
	let mut interval = rivet_env::interval(env.clock(), DISK_IO_REFRESH_MS);
	loop {
		if let Err(error) = interval.tick().await {
			tracing::warn!(%error, "clock failed; stopping disk I/O refresh");
			return;
		}
		for device in devices {
			let current = match host.disk_io(device).await {
				Ok(current) => current,
				Err(error) => {
					tracing::debug!(%error, device, "cannot read disk I/O counters");
					continue;
				}
			};
			let last = previous.insert(device.clone(), current).unwrap_or_default();
			let labels = [device.as_str()];
			metrics::DISK_READ_BYTES
				.with_label_values(&labels)
				.inc_by(advance(last.read_bytes, current.read_bytes));
			metrics::DISK_WRITTEN_BYTES
				.with_label_values(&labels)
				.inc_by(advance(last.written_bytes, current.written_bytes));
			metrics::DISK_WRITES
				.with_label_values(&labels)
				.inc_by(advance(last.writes_completed, current.writes_completed));
			metrics::DISK_BUSY
				.with_label_values(&labels)
				.inc_by(advance(last.busy_ms, current.busy_ms) as f64 / 1000.0);
		}
	}
}

/// Growth of a kernel counter between samples. A counter that went backwards was reset
/// (by a device re-attach or a 32-bit wrap), so all of its current value is new.
fn advance(previous: u64, current: u64) -> u64 {
	if current >= previous {
		current - previous
	} else {
		current
	}
}

fn saturating_i64(value: u64) -> i64 {
	i64::try_from(value).unwrap_or(i64::MAX)
}


