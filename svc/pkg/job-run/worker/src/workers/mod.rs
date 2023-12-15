mod cleanup;
mod create;
mod nomad_monitor_alloc_plan;
mod nomad_monitor_alloc_update;
mod nomad_monitor_eval_update;
mod stop;

chirp_worker::workers![
	cleanup,
	create,
	nomad_monitor_alloc_plan,
	nomad_monitor_alloc_update,
	nomad_monitor_eval_update,
	stop,
];
