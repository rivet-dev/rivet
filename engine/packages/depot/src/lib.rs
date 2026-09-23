pub mod burst_mode;
pub mod startup;
use gas::prelude::*;

mod compaction;
pub mod conveyer;
pub mod doctor;
#[cfg(feature = "test-faults")]
pub mod fault;
pub mod gc;
pub mod inspect;
pub mod metrics;
pub mod recovery;
#[cfg(debug_assertions)]
pub mod takeover;
pub mod workflows;

pub use conveyer::constants::*;
#[cfg(debug_assertions)]
pub use conveyer::debug;
pub use conveyer::pitr_interval;
pub use conveyer::{constants, error, keys, ltx, page_index, policy, quota, types, udb};

pub fn registry(config: &rivet_config::Config) -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();

	if !config.sqlite().unstable_disable_compaction() {
		registry.register_workflow::<compaction_backfill::CompactionBackfillWorkflow>()?;
		registry.register_workflow::<db_hot_compactor::DbHotCompactorWorkflow>()?;
		registry.register_workflow::<db_manager::DbManagerWorkflow>()?;
		registry.register_workflow::<db_reclaimer::DbReclaimerWorkflow>()?;
	}

	Ok(registry)
}
