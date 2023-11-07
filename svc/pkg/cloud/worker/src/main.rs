use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![device_link_complete].await?;

		Ok(())
	})?
}
