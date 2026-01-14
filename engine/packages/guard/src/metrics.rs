use lazy_static::lazy_static;
use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static! {
	pub static ref ROUTE_TOTAL: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"guard_route_total",
		"Total number of routing results handled.",
		&["router"],
		*REGISTRY
	)
	.unwrap();
}
