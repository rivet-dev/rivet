use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

mod workers;
use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![
			invite_consume,
			state_mm_lobby_find_complete,
			state_mm_lobby_find_fail,
			invite_create,
			create,
			destroy,
			publicity_set,
			member_kick,
			member_leave_user_presence_leave,
			state_mm_lobby_find,
			member_state_mm_player_remove_cmpl,
			member_state_set_mm_pending,
			state_set_idle,
			leader_set,
			member_state_mm_lobby_find_fail,
			member_create,
			invite_destroy,
			member_remove,
			state_mm_lobby_cleanup,
			member_state_resolve,
			member_state_set_inactive,
			member_state_mm_lobby_find_complete,
		]
		.await;

		Result::<_, ManagerError>::Ok(())
	})?
}
