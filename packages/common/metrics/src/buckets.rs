pub const BUCKETS: &[f64] = &[
	// Added
	0.001, 0.0025,
	// Copied from https://docs.rs/prometheus/latest/src/prometheus/histogram.rs.html#25-27
	0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, // Added
	25.0, 50.0, 100.0, 250.0, 500.0,
];

pub const PROVISION_BUCKETS: &[f64] = &[
	0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 35.0, 50.0, 75.0, 100.0, 125.0, 250.0, 500.0, 1000.0,
];

pub const MICRO_BUCKETS: &[f64] = &[
	0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.10, 0.25, 0.5, 1.0, 2.5,
	5.0, 10.0, 25.0, 50.0,
];

// Calculated based on the LogHistogram configuration in `packages/common/runtime/src/lib.rs`
pub const TASK_POLL_BUCKETS: &[f64] = &[
	0.00002,
	0.000032768,
	0.000065536,
	0.000131072,
	0.000262144,
	0.000524288,
	0.001048576,
	0.002097152,
	0.004194304,
	0.008388608,
	0.016777216,
	0.032,
];
