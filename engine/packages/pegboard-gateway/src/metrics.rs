use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref TUNNEL_PING_DURATION: Histogram = register_histogram_with_registry!(
		"gateway_tunnel_ping_duration",
		"RTT of messages from gateway to pegboard.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
}
