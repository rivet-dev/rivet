pub mod db_cold_compacter;
pub mod db_hot_compacter;
pub mod db_manager;
pub mod db_reclaimer;

pub mod compaction {
	pub use crate::compaction::types::*;

	pub use super::db_cold_compacter::*;
	pub use super::db_hot_compacter::*;
	pub use super::db_manager::*;
	pub use super::db_reclaimer::*;

	#[cfg(feature = "test-faults")]
	pub use crate::compaction::test_driver::*;
	#[cfg(debug_assertions)]
	pub use crate::compaction::test_hooks;
}

#[cfg(test)]
use crate::compaction::shared::{
	content_hash, fingerprint_repair_reclaim_range, plan_cold_job, plan_hot_job,
	read_reclaim_input_snapshot,
};
#[cfg(test)]
use compaction::*;
#[cfg(test)]
use db_manager::{
	ManagerEffect, manager_effect_for_requested_stop, manager_effects_after_refresh,
	manager_effects_for_cold_job_finished, manager_effects_for_hot_job_finished,
	manager_effects_for_reclaim_job_finished, repair_reclaim_input_range,
};
#[cfg(test)]
use db_reclaimer::{cleanup_repair_fdb_outputs_tx, plan_orphan_cold_object_deletes_tx};

#[cfg(test)]
#[path = "../../tests/inline/workflows_compaction.rs"]
mod tests;
