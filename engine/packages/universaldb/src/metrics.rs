use rivet_metrics::{MICRO_BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref KEY_PACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_key_pack_count",
		"How many times a key has been packed.",
		&["type"],
		*REGISTRY
	).unwrap();
	pub static ref KEY_UNPACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_key_unpack_count",
		"How many times a key has been unpacked.",
		&["type"],
		*REGISTRY
	).unwrap();
}
