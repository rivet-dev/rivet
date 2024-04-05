mod lobby_cleanup;
mod lobby_closed_set;
mod lobby_create;
mod lobby_find;
mod lobby_find_job_run_fail;
mod lobby_find_lobby_cleanup;
mod lobby_find_lobby_create_fail;
mod lobby_find_lobby_ready;
mod lobby_history_export;
mod lobby_job_run_cleanup;
mod lobby_ready_set;
mod lobby_state_set;
mod lobby_stop;
mod nomad_node_closed_set;
mod player_register;
mod player_remove;

chirp_worker::workers![
	nomad_node_closed_set,
	lobby_cleanup,
	lobby_closed_set,
	lobby_create,
	lobby_find,
	lobby_find_job_run_fail,
	lobby_find_lobby_cleanup,
	lobby_find_lobby_create_fail,
	lobby_find_lobby_ready,
	lobby_history_export,
	lobby_job_run_cleanup,
	lobby_ready_set,
	lobby_state_set,
	lobby_stop,
	player_register,
	player_remove,
];
