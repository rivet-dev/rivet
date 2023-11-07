use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![
			create,
			delete,
			event_chat_last_read_ts_update,
			event_chat_message_create_complete,
			event_team_member_remove,
			event_user_mm_lobby_join,
			event_user_presence_update,
			event_user_update,
			profile_set,
			search_update_user_follow_create,
			search_update_user_update,
			search_update,
			updated_user_follow_create,
			updated_user_follow_delete,
			updated_user_presence_update,
			updated_user_update,
		]
		.await?;

		Ok(())
	})?
}
