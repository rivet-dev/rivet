pub const BUCKETS: &[f64] = &[
	0.001,
	0.0025, // Copied from https://docs.rs/prometheus/latest/src/prometheus/histogram.rs.html#25-27
	0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0,
];

/// Latency buckets for the actor start path, in seconds: 2 ms steps to 12 ms, about 1.25x up to
/// 1 s, then a coarse tail to 60 s so an overloaded path reports how slow it is instead of
/// saturating at 1 s. `BUCKETS` and `MICRO_BUCKETS` cover this range but jump 10, 25, 50 and
/// 100 ms across exactly the span where start phases and single quorum appends fall, so their
/// percentiles are interpolations inside one bucket.
pub const START_BUCKETS: &[f64] = &[
	0.002, 0.004, 0.006, 0.008, 0.010, 0.012, 0.015, 0.020, 0.025, 0.030, 0.040, 0.050, 0.065,
	0.080, 0.100, 0.130, 0.160, 0.200, 0.250, 0.320, 0.400, 0.500, 0.750, 1.0, 1.5, 2.5, 5.0, 10.0,
	30.0, 60.0,
];

pub const MICRO_BUCKETS: &[f64] = &[
	0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.10, 0.25, 0.5, 1.0, 2.5,
	5.0, 10.0, 25.0, 50.0,
];

pub const LIFETIME_BUCKETS: &[f64] = &[1.0, 10.0, 60.0, 300.0, 600.0, 1200.0, 1800.0, 3600.0];

/// Sizes in bytes from 4 KiB to 32 MiB, in steps of 4 up to 1 MiB and doubling above it, for
/// write and batch sizes bounded by a write cap of a few MiB.
pub const BYTE_SIZE_BUCKETS: &[f64] = &[
	4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 2097152.0, 4194304.0, 8388608.0, 16777216.0,
	33554432.0,
];

pub const PAGE_COUNT_BUCKETS: &[f64] = &[0.0, 1.0, 4.0, 16.0, 64.0, 256.0, 1024.0, 4096.0, 8192.0];

/// Small counts such as SSTs or sorted runs of one database, fine around a limit of 16.
pub const SMALL_COUNT_BUCKETS: &[f64] = &[
	0.0, 1.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 48.0, 64.0,
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
