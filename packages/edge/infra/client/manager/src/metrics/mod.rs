use prometheus::*;

// mod buckets;
mod registry;
mod server;

// pub use buckets::BUCKETS;
pub use registry::REGISTRY;
pub use server::run_standalone;

lazy_static::lazy_static! {
	pub static ref PACKET_RECV_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"packet_recv_total",
		"Total number of packets received.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref PACKET_SEND_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"packet_send_total",
		"Total number of packets sent.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref UNKNOWN_ISOLATE_RUNNER: IntCounterVec = register_int_counter_vec_with_registry!(
		"unknown_isolate_runner",
		"Total number of unknown isolate runners that were found and killed.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref DUPLICATE_RUNNER: IntCounterVec = register_int_counter_vec_with_registry!(
		"duplicate_runner",
		"Total number of duplicate runners that were found and killed.",
		&["pid"],
		*REGISTRY,
	).unwrap();

	pub static ref SQL_ERROR: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_error",
		"An SQL error occurred.",
		&["error"],
		*REGISTRY,
	).unwrap();

	// Actor setup step duration metrics
	pub static ref SETUP_TOTAL_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_total_duration_seconds",
		"Duration of the complete actor setup process in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_MAKE_FS_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_make_fs_duration_seconds",
		"Duration of make_fs step in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_DOWNLOAD_IMAGE_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_download_image_duration_seconds",
		"Duration of download_image step in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_BIND_PORTS_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_bind_ports_duration_seconds",
		"Duration of bind_ports step in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_CNI_NETWORK_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_cni_network_duration_seconds",
		"Duration of setup_cni_network step in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_OCI_BUNDLE_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_oci_bundle_duration_seconds",
		"Duration of setup_oci_bundle step in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_ISOLATE_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_isolate_duration_seconds",
		"Duration of setup_isolate step in seconds",
		*REGISTRY,
	).unwrap();

	pub static ref SETUP_PARALLEL_TASKS_DURATION: Histogram = register_histogram_with_registry!(
		"actor_setup_parallel_tasks_duration_seconds",
		"Duration of parallel setup tasks (image download/fs + ports/network) in seconds",
		*REGISTRY,
	).unwrap();
}
