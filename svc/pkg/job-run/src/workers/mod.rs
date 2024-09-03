mod cleanup;
mod create;
mod drain_all;
mod nomad_monitor_alloc_plan;
mod nomad_monitor_alloc_update;
mod nomad_monitor_eval_update;
mod stop;

lazy_static::lazy_static! {
	pub static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
	pub static ref NEW_NOMAD_CONFIG: nomad_client_new::apis::configuration::Configuration =
		nomad_util::new_config_from_env().unwrap();
}

chirp_worker::workers![
	cleanup,
	create,
	drain_all,
	nomad_monitor_alloc_plan,
	nomad_monitor_alloc_update,
	nomad_monitor_eval_update,
	stop,
];
