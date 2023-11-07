use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![arrive, leave, status_set, game_activity_set].await?;

		Ok(())
	})?
}
