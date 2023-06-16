use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

mod workers;
use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![
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
			lobby_stop,
			player_register,
			player_remove,
		]
		.await;

		Result::<_, ManagerError>::Ok(())
	})?
}
