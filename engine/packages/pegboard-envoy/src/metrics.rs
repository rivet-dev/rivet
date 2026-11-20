use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub(crate) static ref ACTOR_HANDLES: IntGauge = register_int_gauge_with_registry!(
		Opts::new("pegboard_node_actor_handles",
		"Retained actor handles across node connections."),
		*REGISTRY
	).unwrap();
	pub(crate) static ref ACTOR_HANDLE_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("pegboard_node_actor_handles_capacity",
		"Configured actor handle cap per node connection."),
		*REGISTRY
	).unwrap();
	pub(crate) static ref MISSING_POOLS: IntGauge = register_int_gauge_with_registry!(
		Opts::new("pegboard_node_missing_pools",
		"Pool names this process refused node connects for, tracked for the sustained refusal warning."),
		*REGISTRY
	).unwrap();
	pub(crate) static ref MISSING_POOLS_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("pegboard_node_missing_pools_capacity",
		"Configured cap on the pool names one process tracks for refused node connects."),
		*REGISTRY
	).unwrap();
}
