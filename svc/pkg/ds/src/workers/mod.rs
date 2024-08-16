pub mod nomad_monitor_alloc_plan;
pub mod nomad_monitor_alloc_update;
pub mod nomad_monitor_eval_update;
mod webhook;

chirp_worker::workers![
	nomad_monitor_alloc_plan,
	nomad_monitor_alloc_update,
	nomad_monitor_eval_update
];
