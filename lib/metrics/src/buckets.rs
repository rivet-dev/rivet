pub const BUCKETS: &[f64] = &[
	// Added
	0.001, 0.0025,
	// Copied from https://docs.rs/prometheus/latest/src/prometheus/histogram.rs.html#25-27
	0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, // Added
	25.0, 50.0, 100.0, 250.0, 500.0,
];
