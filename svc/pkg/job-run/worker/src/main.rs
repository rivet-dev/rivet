use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		worker_group![
			cleanup,
			create,
			nomad_monitor_alloc_plan,
			nomad_monitor_alloc_update,
			nomad_monitor_eval_update,
			stop,
		]
		.await?;

		Ok(())
	})?
}
