//! RFC-0029 invariants for the `.compactions` object.

use std::error::Error;
use std::sync::Arc;

use slatedb_txn_obj::Invariant;
use ulid::Ulid;

use crate::compactor_state::{Compaction, Compactions};
use crate::error::SlateDBError;

/// Rejects a new job with a timestamp below the current job watermark.
pub(crate) fn compaction_job_id_cutoff(
    dirty: &Compactions,
    current: &Compactions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(watermark) = current.iter().map(|job| job.id().timestamp_ms()).max() else {
        return Ok(());
    };

    for job in dirty.iter() {
        let job_id = job.id();
        if current.get(&job_id).is_none() && job_id.timestamp_ms() < watermark {
            return Err(Box::new(SlateDBError::InvalidClockTick {
                last_tick: watermark as i64,
                next_tick: job_id.timestamp_ms() as i64,
            }));
        }
    }
    Ok(())
}

/// Rejects an output SST with a timestamp below its compaction job timestamp.
/// This covers final and partial outputs. Final sorted-run views use the same ID as their physical SST.
pub(crate) fn sorted_run_ulid_cutoff(
    dirty: &Compactions,
    _current: &Compactions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for job in dirty.iter() {
        let watermark = job.id().timestamp_ms();
        for output_id in output_sst_ids(job) {
            if output_id.timestamp_ms() < watermark {
                return Err(Box::new(SlateDBError::InvalidClockTick {
                    last_tick: watermark as i64,
                    next_tick: output_id.timestamp_ms() as i64,
                }));
            }
        }
    }
    Ok(())
}

/// Returns final and partial output SST IDs for one compaction job.
fn output_sst_ids(job: &Compaction) -> Vec<Ulid> {
    let mut output_ids = job
        .output_ssts()
        .into_iter()
        .map(|sst| sst.id.value())
        .collect::<Vec<_>>();
    output_ids.extend(
        job.subcompactions()
            .iter()
            .flat_map(|subcompaction| subcompaction.output_ssts())
            .map(|sst| sst.id.value()),
    );
    output_ids
}

/// The RFC-0029 invariants that run before each `.compactions` update.
pub(crate) fn compactions_invariants() -> Vec<Invariant<Compactions>> {
    vec![
        Arc::new(compaction_job_id_cutoff),
        Arc::new(sorted_run_ulid_cutoff),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes_range::BytesRange;
    use crate::compactor_state::{CompactionContext, CompactionSpec};
    use crate::db_state::{SsTableHandle, SsTableId, SsTableInfo};
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::subcompaction::Subcompaction;

    #[test]
    fn compaction_job_id_cutoff_rejects_an_older_new_job() {
        let current_job = compaction(100, 0);
        let current = compactions([current_job.clone()]);
        let dirty = compactions([current_job, compaction(50, 0)]);

        assert_invalid_clock_tick(compaction_job_id_cutoff(&dirty, &current), 100, 50);
    }

    #[test]
    fn compaction_job_id_cutoff_allows_the_same_millisecond() {
        let current_job = compaction(100, 1);
        let current = compactions([current_job.clone()]);
        let dirty = compactions([current_job, compaction(100, 0)]);

        compaction_job_id_cutoff(&dirty, &current).unwrap();
    }

    #[test]
    fn compaction_job_id_cutoff_allows_an_older_existing_job() {
        let older_job = compaction(50, 0);
        let newest_job = compaction(100, 0);
        let current = compactions([older_job.clone(), newest_job.clone()]);
        let dirty = compactions([older_job, newest_job, compaction(101, 0)]);

        compaction_job_id_cutoff(&dirty, &current).unwrap();
    }

    #[test]
    fn sorted_run_ulid_cutoff_rejects_an_older_final_output() {
        let job = compaction(100, 0).with_output_ssts(vec![output_sst(50, 0)]);
        let dirty = compactions([job]);

        assert_invalid_clock_tick(
            sorted_run_ulid_cutoff(&dirty, &Compactions::new(0)),
            100,
            50,
        );
    }

    #[test]
    fn sorted_run_ulid_cutoff_rejects_an_older_partial_output() {
        let context = CompactionContext::new(
            vec![Subcompaction::new(BytesRange::unbounded())
                .with_output_ssts(vec![output_sst(50, 0)])],
            None,
        );
        let job = compaction(100, 0).with_ctx(Some(context));
        let dirty = compactions([job]);

        assert_invalid_clock_tick(
            sorted_run_ulid_cutoff(&dirty, &Compactions::new(0)),
            100,
            50,
        );
    }

    #[test]
    fn sorted_run_ulid_cutoff_allows_the_same_millisecond() {
        let job = compaction(100, 1).with_output_ssts(vec![output_sst(100, 0)]);
        let dirty = compactions([job]);

        sorted_run_ulid_cutoff(&dirty, &Compactions::new(0)).unwrap();
    }

    fn compactions(jobs: impl IntoIterator<Item = Compaction>) -> Compactions {
        Compactions::new(0).with_compactions(jobs.into_iter().collect())
    }

    fn compaction(timestamp_ms: u64, random: u128) -> Compaction {
        Compaction::new(
            Ulid::from_parts(timestamp_ms, random),
            CompactionSpec::new(vec![], 0),
        )
    }

    fn output_sst(timestamp_ms: u64, random: u128) -> SsTableHandle {
        SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(timestamp_ms, random)),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo::default(),
        )
    }

    fn assert_invalid_clock_tick(
        result: Result<(), Box<dyn Error + Send + Sync>>,
        last_tick: i64,
        next_tick: i64,
    ) {
        let error = result.unwrap_err().downcast::<SlateDBError>().unwrap();
        assert!(matches!(
            *error,
            SlateDBError::InvalidClockTick {
                last_tick: actual_last,
                next_tick: actual_next,
            } if actual_last == last_tick && actual_next == next_tick
        ));
    }
}
