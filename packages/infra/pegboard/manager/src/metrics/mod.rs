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
}
