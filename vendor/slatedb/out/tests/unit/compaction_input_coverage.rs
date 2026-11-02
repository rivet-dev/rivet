use super::*;
use crate::compactor_state::{CompactionSpec, SourceId};
use crate::db_state::{SsTableHandle, SsTableId, SsTableInfo, SsTableView};
use crate::format::sst::SST_FORMAT_VERSION_LATEST;

fn sst(n: u64) -> SsTableView {
    SsTableView::identity(SsTableHandle::new(
        SsTableId::from(Ulid::from_parts(n, 0)),
        SST_FORMAT_VERSION_LATEST,
        SsTableInfo::default(),
    ))
}
#[test]
fn compaction_input_coverage_requires_every_l0_and_sorted_run() {
    let mut manifest = ManifestCore::new();
    manifest.last_l0_seq = 200;
    let l0 = sst(10);
    let sorted = SortedRun::new(0, [sst(1)]);
    Arc::make_mut(&mut manifest.tree).l0.push_back(l0.clone());
    Arc::make_mut(&mut manifest.tree).compacted.push(sorted);
    let partial = Compaction::new(
        Ulid::from_parts(11, 0),
        CompactionSpec::new(vec![SourceId::SortedRun(0)], 0),
    );
    let args = CompactionWorkerHandler::build_job_args(&partial, &manifest, "test").unwrap();
    assert!(args.is_dest_last_run);
    assert_eq!(args.complete_input_through, None);
    let full = Compaction::new(
        Ulid::from_parts(12, 0),
        CompactionSpec::new(vec![SourceId::SstView(l0.id), SourceId::SortedRun(0)], 0),
    );
    let args = CompactionWorkerHandler::build_job_args(&full, &manifest, "test").unwrap();
    assert_eq!(args.complete_input_through, Some(200));
    // A newer L0 appears after the job was planned; resuming that same job
    // cannot use the newer manifest's cold barrier as complete input evidence.
    Arc::make_mut(&mut manifest.tree).l0.push_front(sst(13));
    manifest.last_l0_seq = 250;
    assert_eq!(
        CompactionWorkerHandler::build_job_args(&full, &manifest, "test")
            .unwrap()
            .complete_input_through,
        None
    );
}
