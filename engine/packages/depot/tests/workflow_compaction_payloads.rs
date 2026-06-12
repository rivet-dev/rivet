use depot::workflows::compaction::{
	DeltasAvailable, DestroyDatabaseBranch, ForceCompaction, HotJobFinished, ReclaimJobFinished,
	RunHotJob, RunReclaimJob,
};
use gas::prelude::SignalTrait;

#[test]
fn compaction_signal_names_are_stable() {
	assert_eq!(
		<DeltasAvailable as SignalTrait>::NAME,
		"depot_sqlite_cmp_deltas_available"
	);
	assert_eq!(
		<HotJobFinished as SignalTrait>::NAME,
		"depot_sqlite_cmp_hot_job_finished"
	);
	assert_eq!(
		<ReclaimJobFinished as SignalTrait>::NAME,
		"depot_sqlite_cmp_reclaim_job_finished"
	);
	assert_eq!(
		<ForceCompaction as SignalTrait>::NAME,
		"depot_sqlite_cmp_force_compaction"
	);
	assert_eq!(
		<DestroyDatabaseBranch as SignalTrait>::NAME,
		"depot_sqlite_cmp_destroy_database_branch"
	);
	assert_eq!(
		<RunHotJob as SignalTrait>::NAME,
		"depot_sqlite_cmp_run_hot_job"
	);
	assert_eq!(
		<RunReclaimJob as SignalTrait>::NAME,
		"depot_sqlite_cmp_run_reclaim_job"
	);
}
