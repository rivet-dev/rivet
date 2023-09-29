use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

mod workers;
use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![
			create_complete_chat_message_create,
			create,
			join_request_create,
			join_request_resolve,
			member_create,
			member_kick,
			member_remove,
			owner_transfer,
			profile_set,
			user_ban,
			user_unban,
		]
		.await?;

		Ok(())
	})?
}
