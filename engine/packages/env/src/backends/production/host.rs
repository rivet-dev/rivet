//! Production adapter for `rivet_env::host`, reading Linux `/proc` and `/sys`.

#![allow(
	clippy::disallowed_methods,
	reason = "production host adapter owns reads of kernel pseudo-files"
)]

use crate::{
	fs::FsFuture,
	host::{BlockDevice, CpuInfo, DiskIo, HostBackend, MemoryInfo},
};
use std::{
	io,
	path::{Path, PathBuf},
};

/// `/proc/diskstats` counts sectors of 512 bytes regardless of the device's sector size.
const DISKSTATS_SECTOR_BYTES: u64 = 512;

/// cgroup v1 reports "no limit" as a page-rounded `i64::MAX`. Anything this large is not a
/// real limit.
const CGROUP_V1_UNLIMITED: u64 = 1 << 60;

pub struct ProductionHost;

impl HostBackend for ProductionHost {
	fn cpu(&self) -> FsFuture<'_, CpuInfo> {
		Box::pin(blocking(|| {
			// SAFETY: sysconf has no preconditions.
			let online = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
			let logical_cpus = u32::try_from(online)
				.ok()
				.filter(|count| *count > 0)
				.ok_or_else(|| io::Error::other("sysconf reported no online CPUs"))?;
			let model = match std::fs::read_to_string("/proc/cpuinfo") {
				Ok(text) => parse_cpu_model(&text),
				Err(error) => {
					tracing::debug!(%error, "cannot read /proc/cpuinfo; reporting no cpu model");
					None
				}
			};
			Ok(CpuInfo {
				logical_cpus,
				model,
			})
		}))
	}

	fn memory(&self) -> FsFuture<'_, MemoryInfo> {
		Box::pin(blocking(|| {
			let meminfo = std::fs::read_to_string("/proc/meminfo")?;
			let total_bytes = parse_meminfo_total(&meminfo)
				.ok_or_else(|| io::Error::other("/proc/meminfo has no MemTotal"))?;
			Ok(MemoryInfo {
				total_bytes,
				limit_bytes: cgroup_memory_limit(),
			})
		}))
	}

	fn block_device<'a>(&'a self, path: &'a Path) -> FsFuture<'a, Option<BlockDevice>> {
		let path = path.to_path_buf();
		Box::pin(blocking(move || block_device(&path)))
	}

	fn disk_io<'a>(&'a self, device: &'a str) -> FsFuture<'a, DiskIo> {
		let device = device.to_owned();
		Box::pin(blocking(move || {
			let text = std::fs::read_to_string("/proc/diskstats")?;
			parse_diskstats(&text, &device).ok_or_else(|| {
				io::Error::new(
					io::ErrorKind::NotFound,
					format!("{device} is not in /proc/diskstats"),
				)
			})
		}))
	}
}

/// Runs a pseudo-file read on Tokio's blocking pool; sysfs reads can stall on slow drivers.
async fn blocking<T: Send + 'static>(
	work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
	tokio::task::spawn_blocking(work)
		.await
		.map_err(io::Error::other)?
}

fn block_device(path: &Path) -> io::Result<Option<BlockDevice>> {
	use std::os::unix::fs::MetadataExt;

	let dev = std::fs::metadata(path)?.dev();
	let (major, minor) = split_dev(dev);
	// Major 0 is the anonymous device of tmpfs, overlay and other virtual filesystems.
	if major == 0 {
		return Ok(None);
	}

	let sys = PathBuf::from(format!("/sys/dev/block/{major}:{minor}"));
	let uevent = match std::fs::read_to_string(sys.join("uevent")) {
		Ok(text) => text,
		Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
		Err(error) => return Err(error),
	};
	let Some(name) = parse_uevent_devname(&uevent) else {
		return Ok(None);
	};

	// A partition has no queue of its own; its parent disk's queue describes the media.
	let rotational = ["queue/rotational", "../queue/rotational"]
		.iter()
		.find_map(|relative| std::fs::read_to_string(sys.join(relative)).ok())
		.and_then(|text| match text.trim() {
			"0" => Some(false),
			"1" => Some(true),
			_ => None,
		});

	Ok(Some(BlockDevice { name, rotational }))
}

/// Splits a Linux `dev_t` into its major and minor numbers using the kernel's encoding.
pub(crate) fn split_dev(dev: u64) -> (u64, u64) {
	let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0x0fff);
	let minor = ((dev >> 12) & 0xffff_ff00) | (dev & 0x00ff);
	(major, minor)
}

/// Reads the limit of this process's cgroup, trying cgroup v2 at the process's own
/// cgroup, then at the namespace root, then cgroup v1.
fn cgroup_memory_limit() -> Option<u64> {
	let mut candidates = Vec::new();
	if let Ok(text) = std::fs::read_to_string("/proc/self/cgroup")
		&& let Some(relative) = parse_cgroup_v2_path(&text)
	{
		candidates.push(format!("/sys/fs/cgroup{relative}/memory.max"));
	}
	candidates.push("/sys/fs/cgroup/memory.max".to_owned());
	candidates.push("/sys/fs/cgroup/memory/memory.limit_in_bytes".to_owned());

	candidates
		.iter()
		.find_map(|path| std::fs::read_to_string(path).ok())
		.and_then(|text| parse_cgroup_limit(&text))
}

/// The first model line of `/proc/cpuinfo`, with runs of whitespace collapsed.
pub(crate) fn parse_cpu_model(cpuinfo: &str) -> Option<String> {
	["model name", "Processor", "cpu model"]
		.iter()
		.find_map(|key| {
			cpuinfo.lines().find_map(|line| {
				let (name, value) = line.split_once(':')?;
				if name.trim() != *key {
					return None;
				}
				let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
				if value.is_empty() { None } else { Some(value) }
			})
		})
}

pub(crate) fn parse_meminfo_total(meminfo: &str) -> Option<u64> {
	meminfo.lines().find_map(|line| {
		let value = line.strip_prefix("MemTotal:")?;
		let kib = value
			.trim()
			.strip_suffix("kB")?
			.trim()
			.parse::<u64>()
			.ok()?;
		kib.checked_mul(1024)
	})
}

/// The cgroup v2 path in `/proc/self/cgroup`, from its `0::<path>` line.
pub(crate) fn parse_cgroup_v2_path(text: &str) -> Option<&str> {
	text.lines()
		.find_map(|line| line.strip_prefix("0::"))
		.map(|path| path.trim_end_matches('/'))
}

/// Parses `memory.max` or `memory.limit_in_bytes`. Both spellings of "no limit" map to `None`.
pub(crate) fn parse_cgroup_limit(text: &str) -> Option<u64> {
	let value = text.trim();
	if value == "max" {
		return None;
	}
	value
		.parse::<u64>()
		.ok()
		.filter(|limit| *limit < CGROUP_V1_UNLIMITED)
}

pub(crate) fn parse_uevent_devname(uevent: &str) -> Option<String> {
	uevent
		.lines()
		.find_map(|line| line.strip_prefix("DEVNAME="))
		.map(str::to_owned)
		.filter(|name| !name.is_empty())
}

/// Finds `device` in `/proc/diskstats`. Columns after the name are reads, merged reads,
/// sectors read, read ms, writes, merged writes, sectors written, write ms, in-flight
/// requests and busy ms.
pub(crate) fn parse_diskstats(text: &str, device: &str) -> Option<DiskIo> {
	text.lines().find_map(|line| {
		let fields: Vec<&str> = line.split_whitespace().collect();
		if fields.get(2) != Some(&device) {
			return None;
		}
		let field = |index: usize| fields.get(index)?.parse::<u64>().ok();
		Some(DiskIo {
			read_bytes: field(5)?.saturating_mul(DISKSTATS_SECTOR_BYTES),
			writes_completed: field(7)?,
			written_bytes: field(9)?.saturating_mul(DISKSTATS_SECTOR_BYTES),
			busy_ms: field(12)?,
		})
	})
}


