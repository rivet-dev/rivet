//! Host hardware gauges and per-device disk counters. Directories are labelled by role and
//! devices by kernel name; both are bounded per host.

use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	pub(crate) static ref CPU_LOGICAL: IntGauge = register_int_gauge_with_registry!("host_cpu_logical_count", "Logical CPUs online on the host.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref INFO: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("host_info", "Always 1, labelled with the host's CPU model."), &["cpu_model"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MEMORY_TOTAL: IntGauge = register_int_gauge_with_registry!("host_memory_total_bytes", "Physical memory of the host.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref MEMORY_LIMIT: IntGauge = register_int_gauge_with_registry!("host_memory_limit_bytes", "Memory limit of the engine's cgroup. Absent when it has none.", *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_INFO: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("host_disk_info", "Always 1, labelled with the block device holding each data directory and whether it reports rotating media."), &["dir", "device", "rotational"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_TOTAL: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("host_disk_total_bytes", "Size of the filesystem holding each data directory."), &["dir"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_FREE: IntGaugeVec = register_int_gauge_vec_with_registry!(Opts::new("host_disk_free_bytes", "Bytes an unprivileged writer can still use on the filesystem holding each data directory."), &["dir"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref PROBE_WRITE_THROUGHPUT: GaugeVec = register_gauge_vec_with_registry!(Opts::new("host_disk_probe_write_bytes_per_second", "Sequential write throughput of the startup disk probe, including the final fsync."), &["dir"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref PROBE_FSYNC: GaugeVec = register_gauge_vec_with_registry!(Opts::new("host_disk_probe_fsync_seconds", "Latency of the startup disk probe's 4 KiB append plus fdatasync, by quantile; 1 is the maximum."), &["dir", "quantile"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_READ_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("host_disk_read_bytes_total", "Bytes read from each data device, from /proc/diskstats."), &["device"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_WRITTEN_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("host_disk_written_bytes_total", "Bytes written to each data device, from /proc/diskstats."), &["device"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_WRITES: IntCounterVec = register_int_counter_vec_with_registry!(Opts::new("host_disk_writes_completed_total", "Write requests completed by each data device, from /proc/diskstats."), &["device"], *rivet_metrics::REGISTRY).unwrap();
	pub(crate) static ref DISK_BUSY: CounterVec = register_counter_vec_with_registry!(Opts::new("host_disk_busy_seconds_total", "Time each data device had a request in flight, from /proc/diskstats."), &["device"], *rivet_metrics::REGISTRY).unwrap();
}
