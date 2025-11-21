use rivet_metrics::{
	BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-gateway");

	/// Has no expected attributes
	pub static ref TUNNEL_PING_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gateway_tunnel_ping_duration")
		.with_description("RTT of messages from gateway to pegboard.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
}
