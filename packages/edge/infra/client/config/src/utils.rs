// IMPORTANT: This cannot be just `rivet-` because this is used as a prefix to filter cgroup names
// in cadvisor.
//
// If this was "rivet-", we'd have to report on non-actor cgropus with cadvisor.
//
// See also packages/core/services/cluster/src/workflows/server/install/install_scripts/files/cadvisor_metric_exporter.sh & packages/core/api/actor/src/route/metrics.rs
pub const RIVET_CONTAINER_PREFIX: &str = "pegboard-actor-";

pub fn format_container_id(actor_id: &str, generation: u32) -> String {
	format!("{RIVET_CONTAINER_PREFIX}{actor_id}-{generation}")
}
