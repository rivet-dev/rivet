use std::cmp::min;
use std::collections::{HashMap, HashSet, VecDeque};

use bytes::Bytes;

use crate::compactor::{CompactionScheduler, CompactionSchedulerSupplier};
use crate::compactor_state::{Compaction, CompactionSpec, SourceId};
use crate::compactor_state_protocols::CompactorStateView;
use crate::config::{CompactorOptions, SizeTieredCompactionSchedulerOptions};
use crate::error::Error;
use crate::manifest::{LsmTreeState, ManifestCore};
use log::warn;

const DEFAULT_MAX_CONCURRENT_COMPACTIONS: usize = 4;

#[derive(Clone, Copy, Debug)]
struct CompactionSource {
    source: SourceId,
    size: u64,
}

/// Checks a candidate compaction to make sure that it does not conflict
/// with any other ongoing compactions. A compaction conflict if it uses a
/// source (SST or SR) that is already in use by a running compaction.
pub(crate) struct ConflictChecker {
    sources_used: HashSet<SourceId>,
}

impl ConflictChecker {
    fn new<'a>(compactions: impl Iterator<Item = &'a CompactionSpec>) -> Self {
        let mut checker = Self {
            sources_used: HashSet::new(),
        };
        for compaction in compactions {
            checker.add_compaction(compaction);
        }
        checker
    }

    fn check_source(&self, source: SourceId) -> bool {
        !self.sources_used.contains(&source)
    }

    fn check_compaction(&self, sources: &VecDeque<CompactionSource>, dst: u32) -> bool {
        for source in sources.iter() {
            if !self.check_source(source.source) {
                return false;
            }
        }
        self.check_source(SourceId::SortedRun(dst))
    }

    fn add_compaction(&mut self, compaction: &CompactionSpec) {
        for source in compaction.sources().iter() {
            self.sources_used.insert(*source);
        }
        // Drain specs produce no destination SR; only tiered specs reserve
        // a destination id in the conflict set.
        if let Some(dst) = compaction.destination() {
            self.sources_used.insert(SourceId::SortedRun(dst));
        }
    }
}

/// Checks a candidate compaction to ensure that the db is not compacting smaller
/// ssts too quickly. First, the checker estimates the size of the output of a candidate compaction.
/// Then, it sees whether there are already lots of sorted runs that are sized similarly to the
/// output of the candidate compaction. If there are, then it rejects the compaction. To estimate
/// the size of the output, the checker simply sums up the sizes of the sources. To look for
/// similarly sized runs, the db uses the same include_size_threshold as the compaction scheduler.
/// It rejects a compaction if there are more than max_compaction_sources that are <= output
/// size * include_size_threshold.
struct BackpressureChecker {
    include_size_threshold: f32,
    max_compaction_sources: usize,
    longest_compactable_runs_by_sr: HashMap<u32, VecDeque<CompactionSource>>,
}

impl BackpressureChecker {
    fn new(
        include_size_threshold: f32,
        max_compaction_sources: usize,
        srs: &[CompactionSource],
    ) -> Self {
        let mut longest_compactable_runs_by_sr = HashMap::new();
        for (i, sr) in srs.iter().enumerate() {
            let sr_id = sr.source.unwrap_sorted_run();
            let compactable_run = SizeTieredCompactionScheduler::build_compactable_run(
                Some(include_size_threshold),
                srs,
                i,
                None,
                None,
            );
            longest_compactable_runs_by_sr.insert(sr_id, compactable_run);
        }
        Self {
            include_size_threshold,
            max_compaction_sources,
            longest_compactable_runs_by_sr,
        }
    }

    fn check_compaction(
        &self,
        sources: &VecDeque<CompactionSource>,
        next_sr: Option<&CompactionSource>,
    ) -> bool {
        let estimated_result_size = sources.iter().map(|src| src.size).sum();
        self.check_estimated_size(estimated_result_size, next_sr)
    }

    fn check_estimated_size(
        &self,
        estimated_result_size: u64,
        next_sr: Option<&CompactionSource>,
    ) -> bool {
        // checks that we are not creating a run that itself needs to be compacted
        // with lots of runs. If we are, we should wait till those runs are compacted
        if let Some(next_sr) = next_sr {
            if next_sr.size <= ((estimated_result_size as f32) * self.include_size_threshold) as u64
            {
                // the result of this compaction can be compacted with the next sr. Check to see
                // if there's already a max sized list of compactable runs there
                if self
                    .longest_compactable_runs_by_sr
                    .get(&next_sr.source.unwrap_sorted_run())
                    .map(|r| r.len())
                    .unwrap_or(0)
                    >= self.max_compaction_sources
                {
                    return false;
                }
            }
        }
        true
    }
}

/// Per-tree planning state for one `propose()` call: prefix, source
/// lists, and the conflict + backpressure checks that gate picks for
/// this tree. `conflicts` is updated as picks land. Source IDs (L0
/// ULIDs and SR ids) are globally unique per RFC-0024 and a spec's
/// sources must come from a single tree, so each tree only needs to
/// track its own in-flight sources.
struct TreeState {
    prefix: Bytes,
    l0: Vec<CompactionSource>,
    srs: Vec<CompactionSource>,
    projected_sorted_runs: HashSet<u32>,
    conflicts: ConflictChecker,
    bp: BackpressureChecker,
}

impl TreeState {
    fn project_compaction(&mut self, compaction: &CompactionSpec) {
        for source in compaction.sources() {
            if let SourceId::SortedRun(id) = source {
                self.projected_sorted_runs.remove(id);
            }
        }
        if let Some(destination) = compaction.destination() {
            self.projected_sorted_runs.insert(destination);
        }
    }

    fn check_compaction(
        &self,
        sources: &VecDeque<CompactionSource>,
        dst: u32,
        next_sr: Option<&CompactionSource>,
    ) -> bool {
        self.conflicts.check_compaction(sources, dst) && self.bp.check_compaction(sources, next_sr)
    }
}

/// Implements a size-tiered compaction scheduler. The scheduler works by scheduling compaction for
/// one of:
/// - options.min_compaction_sources L0 SSTs
/// - A series of at least options.min_compaction_sources sorted runs where the size of the largest
///   run <= options.include_size_threshold * size of the smallest run.
///
/// The scheduler ensures that a compaction has at most options.max_compaction_sources. The scheduler
/// rejects compactions that violate one of the compaction checkers defined above.
pub(crate) struct SizeTieredCompactionScheduler {
    options: SizeTieredCompactionSchedulerOptions,
    max_concurrent_compactions: usize,
}

impl Default for SizeTieredCompactionScheduler {
    fn default() -> Self {
        Self::new(
            SizeTieredCompactionSchedulerOptions::default(),
            DEFAULT_MAX_CONCURRENT_COMPACTIONS,
        )
    }
}

impl CompactionScheduler for SizeTieredCompactionScheduler {
    fn propose(&self, state: &CompactorStateView) -> Vec<CompactionSpec> {
        let mut compactions = Vec::new();
        let db_state = state.manifest().core();
        let active_compactions = state
            .compactions()
            .into_iter()
            .flat_map(|c| c.core().recent_compactions())
            .filter(|c| c.active())
            .collect::<Vec<_>>();
        // A Compacted job has finished using a worker slot, even though it
        // remains active until its output is committed to the manifest. Keep
        // it in `active_compactions` for conflict checks and destination-id
        // reservation, but do not count it against execution capacity.
        let compaction_slots_in_use = active_compactions
            .iter()
            .filter(|c| c.status().counts_against_max_concurrent())
            .count();
        let mut next_fresh_sr_id = next_global_sr_id(db_state, &active_compactions);

        // Precompute per-tree (sources, conflict checker, backpressure) once
        // and reuse across round-robin passes. Active in-flight compactions
        // are partitioned by their target segment; cross-segment conflicts
        // are impossible (RFC-0024). Trees are sorted by L0 length
        // descending so the tree closest to L0 backpressure gets first dibs
        // — writer-side backpressure is the most pressing pressure to
        // relieve. Stable sort preserves root-first ordering among ties.
        let mut active_by_segment: HashMap<Bytes, Vec<&CompactionSpec>> = HashMap::new();
        for compaction in &active_compactions {
            active_by_segment
                .entry(compaction.spec().segment().clone())
                .or_default()
                .push(compaction.spec());
        }
        let mut trees: Vec<TreeState> = db_state
            .trees_with_prefix()
            .map(|(prefix, tree)| {
                let (l0, srs) = compaction_sources(tree);
                let conflicts = ConflictChecker::new(
                    active_by_segment
                        .get(&prefix)
                        .into_iter()
                        .flatten()
                        .copied(),
                );
                let bp = BackpressureChecker::new(
                    self.options.include_size_threshold,
                    self.options.max_compaction_sources,
                    &srs,
                );
                let mut planning = TreeState {
                    projected_sorted_runs: srs
                        .iter()
                        .map(|source| source.source.unwrap_sorted_run())
                        .collect(),
                    prefix,
                    l0,
                    srs,
                    conflicts,
                    bp,
                };
                for compaction in active_by_segment
                    .get(&planning.prefix)
                    .into_iter()
                    .flatten()
                {
                    planning.project_compaction(compaction);
                }
                planning
            })
            .collect();
        trees.sort_by_key(|t| std::cmp::Reverse(t.l0.len()));

        // Round-robin: one pick per tree per pass, looping until either the
        // budget is exhausted or no tree produced a spec on the last pass.
        // This guarantees every tree gets a chance before any one tree gets
        // a second slot — see the fairness rationale in the test below.
        loop {
            let mut picked_any = false;
            for tree in &mut trees {
                if compaction_slots_in_use + compactions.len() >= self.max_concurrent_compactions {
                    break;
                }
                if let Some(compaction) = self.pick_next_compaction(tree, &mut next_fresh_sr_id) {
                    tree.conflicts.add_compaction(&compaction);
                    tree.project_compaction(&compaction);
                    compactions.push(compaction);
                    picked_any = true;
                }
            }
            if !picked_any {
                break;
            }
        }

        compactions
    }

    fn validate(
        &self,
        state: &CompactorStateView,
        compaction: &CompactionSpec,
    ) -> Result<(), Error> {
        // Size-tiered does not propose drain specs and has no policy
        // opinions on them. Drain invariants belong to the compactor-level
        // validation.
        if compaction.is_drain() {
            return Ok(());
        }

        let core = state.manifest().core();
        let Some(tree) = core.tree_for_segment(compaction.segment()) else {
            warn!(
                "submitted compaction targets unknown segment: {:?}",
                compaction.segment()
            );
            return Err(Error::invalid("unknown target segment".to_string()));
        };
        // Logical order of sources within the target tree: [L0 (newest → oldest),
        // then SRs (highest id → 0)].
        let sources_logical_order: Vec<SourceId> = tree
            .l0
            .iter()
            .map(|view| SourceId::SstView(view.id))
            .chain(tree.compacted.iter().map(|sr| SourceId::SortedRun(sr.id)))
            .collect();

        // Validate if the compaction sources are strictly consecutive elements in the target tree.
        if !sources_logical_order
            .windows(compaction.sources().len())
            .any(|w| w == compaction.sources())
        {
            warn!("submitted compaction is not a consecutive series of sources from db state: {:?} {:?}",
            compaction.sources(), sources_logical_order);
            return Err(Error::invalid(
                "non-consecutive compaction sources".to_string(),
            ));
        }

        // Must merge into the lowest-id SR among sources.
        let min_sr = compaction
            .sources()
            .iter()
            .filter_map(|s| s.maybe_unwrap_sorted_run())
            .min();

        if let Some(min_sr) = min_sr {
            if compaction.destination() != Some(min_sr) {
                warn!(
                    "destination does not match lowest-id SR among sources: {:?} {:?}",
                    compaction.destination(),
                    min_sr
                );
                return Err(Error::invalid(
                    "destination not the lowest-id SR among sources".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl SizeTieredCompactionScheduler {
    pub(crate) fn new(
        mut options: SizeTieredCompactionSchedulerOptions,
        max_concurrent_compactions: usize,
    ) -> Self {
        if options.sorted_run_consolidation_threshold == 1 {
            warn!(
                "sorted_run_consolidation_threshold must be 0 or at least 2. \
                 Disabling fallback consolidation."
            );
            options.sorted_run_consolidation_threshold = 0;
        }
        Self {
            options,
            max_concurrent_compactions,
        }
    }

    fn pick_next_compaction(
        &self,
        tree: &TreeState,
        next_fresh_sr_id: &mut u32,
    ) -> Option<CompactionSpec> {
        // compact l0s if required
        let l0_candidates: VecDeque<_> = tree.l0.iter().copied().collect();
        if let Some(mut l0_candidates) = self.clamp_min(l0_candidates) {
            l0_candidates = self.clamp_max(l0_candidates);
            // SR ids are globally unique (RFC-0024), so the fresh-id counter
            // must come from `next_global_sr_id` rather than this tree alone.
            let dst = *next_fresh_sr_id;
            let next_sr = tree.srs.first();
            if tree.check_compaction(&l0_candidates, dst, next_sr) {
                *next_fresh_sr_id = next_fresh_sr_id.saturating_add(1);
                return Some(self.create_compaction(&tree.prefix, l0_candidates, dst));
            }
        }

        // try to compact the lower levels
        for i in 0..tree.srs.len() {
            let compactable_run = Self::build_compactable_run(
                Some(self.options.include_size_threshold),
                &tree.srs,
                i,
                Some(tree),
                None,
            );
            let compactable_run = self.clamp_min(compactable_run);
            if let Some(mut compactable_run) = compactable_run {
                compactable_run = self.clamp_max(compactable_run);
                return Some(self.create_sorted_run_compaction(&tree.prefix, compactable_run));
            }
        }

        let threshold = self.options.sorted_run_consolidation_threshold;
        let projected_count = tree.projected_sorted_runs.len();
        if threshold > 0 && projected_count > threshold && self.options.max_compaction_sources >= 2
        {
            // Merging k sources removes k - 1 runs. Stop at the projected threshold.
            let source_limit = self
                .options
                .max_compaction_sources
                .min(projected_count - threshold + 1);
            // Try each starting run so a busy newer run does not block older groups.
            for i in 0..tree.srs.len() {
                let consolidation =
                    Self::build_compactable_run(None, &tree.srs, i, Some(tree), Some(source_limit));
                if consolidation.len() >= 2 {
                    return Some(self.create_sorted_run_compaction(&tree.prefix, consolidation));
                }
            }
        }
        None
    }

    fn clamp_min(&self, sources: VecDeque<CompactionSource>) -> Option<VecDeque<CompactionSource>> {
        if sources.len() < self.options.min_compaction_sources {
            return None;
        }
        Some(sources)
    }

    fn clamp_max(&self, mut sources: VecDeque<CompactionSource>) -> VecDeque<CompactionSource> {
        while sources.len() > self.options.max_compaction_sources {
            sources.pop_front();
        }
        sources
    }

    fn create_sorted_run_compaction(
        &self,
        prefix: &Bytes,
        sources: VecDeque<CompactionSource>,
    ) -> CompactionSpec {
        let dst = sources
            .back()
            .expect("expected non-empty sorted-run sources")
            .source
            .unwrap_sorted_run();
        self.create_compaction(prefix, sources, dst)
    }

    fn create_compaction(
        &self,
        prefix: &Bytes,
        sources: VecDeque<CompactionSource>,
        dst: u32,
    ) -> CompactionSpec {
        let sources: Vec<SourceId> = sources.iter().map(|src| src.source).collect();
        CompactionSpec::for_segment(prefix.clone(), sources, dst)
    }

    /// Collect consecutive sources from `start_idx`, stopping at the first rejected source.
    ///
    /// ## Arguments
    /// - `size_threshold`: Optional size multiplier relative to the smallest accepted source.
    ///   `None` disables size filtering.
    /// - `sources`: Sources ordered from newest to oldest.
    /// - `start_idx`: Index of the first candidate source.
    /// - `checker`: Optional conflict and backpressure checks. With `Some`, sources must be sorted runs.
    ///   `None` skips these checks.
    /// - `max_sources`: Maximum number of sources to collect. `None` imposes no count limit.
    fn build_compactable_run(
        size_threshold: Option<f32>,
        sources: &[CompactionSource],
        start_idx: usize,
        checker: Option<&TreeState>,
        max_sources: Option<usize>,
    ) -> VecDeque<CompactionSource> {
        let mut compactable_runs = VecDeque::new();
        let mut maybe_min_sz = None;
        let mut estimated_result_size = 0;
        for i in start_idx..sources.len() {
            if max_sources.is_some_and(|limit| compactable_runs.len() >= limit) {
                break;
            }
            let src = sources[i];
            if let Some(size_threshold) = size_threshold {
                if let Some(min_sz) = maybe_min_sz {
                    if src.size > ((min_sz as f32) * size_threshold) as u64 {
                        break;
                    }
                    maybe_min_sz = Some(min(min_sz, src.size));
                } else {
                    maybe_min_sz = Some(src.size);
                }
            }
            if let Some(checker) = checker {
                // The last sorted-run source is also the destination.
                let dst = src.source.unwrap_sorted_run();
                if !checker.conflicts.check_source(SourceId::SortedRun(dst)) {
                    break;
                }
                estimated_result_size += src.size;
                if !checker
                    .bp
                    .check_estimated_size(estimated_result_size, sources.get(i + 1))
                {
                    break;
                }
            }
            compactable_runs.push_back(src);
        }
        compactable_runs
    }
}

/// Collects L0 and sorted-run sources for a single tree (the empty-prefix
/// segment or one named segment).
fn compaction_sources(tree: &LsmTreeState) -> (Vec<CompactionSource>, Vec<CompactionSource>) {
    let l0: Vec<CompactionSource> = tree
        .l0
        .iter()
        .map(|view| CompactionSource {
            source: SourceId::SstView(view.id),
            size: view.estimate_visible_size(),
        })
        .collect();
    let srs: Vec<CompactionSource> = tree
        .compacted
        .iter()
        .map(|sr| CompactionSource {
            source: SourceId::SortedRun(sr.id),
            size: sr.estimate_visible_size(),
        })
        .collect();
    (l0, srs)
}

/// Computes the next sorted-run id that is safe to use as a fresh L0 → SR
/// destination. Sorted-run ids are globally unique across every tree
/// (RFC-0024), so the counter must skip past every committed run in every
/// tree as well as every in-flight compaction's destination.
fn next_global_sr_id(db_state: &ManifestCore, active_compactions: &[&Compaction]) -> u32 {
    let max_committed = db_state
        .trees()
        .flat_map(|tree| tree.compacted.iter().map(|sr| sr.id))
        .max();
    // Drain specs have no destination — `filter_map` skips them.
    let max_in_flight = active_compactions
        .iter()
        .filter_map(|c| c.spec().destination())
        .max();
    [max_committed, max_in_flight]
        .into_iter()
        .flatten()
        .max()
        .map(|id| id.saturating_add(1))
        .unwrap_or(0)
}

#[derive(Default)]
pub struct SizeTieredCompactionSchedulerSupplier;

impl SizeTieredCompactionSchedulerSupplier {
    pub const fn new() -> Self {
        Self
    }
}

impl CompactionSchedulerSupplier for SizeTieredCompactionSchedulerSupplier {
    fn compaction_scheduler(
        &self,
        compactor_options: &CompactorOptions,
    ) -> Box<dyn CompactionScheduler + Send + Sync> {
        let scheduler_options =
            SizeTieredCompactionSchedulerOptions::from(&compactor_options.scheduler_options);
        Box::new(SizeTieredCompactionScheduler::new(
            scheduler_options,
            compactor_options.max_concurrent_compactions,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::compactor::{CompactionScheduler, CompactionSchedulerSupplier};

    use crate::bytes_range::BytesRange;
    use crate::compactor_state::{
        Compaction, CompactionSpec, CompactionStatus, Compactions, CompactorState, SourceId,
    };
    use crate::config::{CompactorOptions, SizeTieredCompactionSchedulerOptions};
    use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::manifest::store::test_utils::new_dirty_manifest;
    use crate::manifest::{LsmTreeState, ManifestCore, Segment};
    use crate::seq_tracker::SequenceTracker;
    use crate::size_tiered_compaction::CompactionSource;
    use crate::size_tiered_compaction::{
        SizeTieredCompactionScheduler, SizeTieredCompactionSchedulerSupplier,
    };
    use crate::utils::IdGenerator;
    use bytes::Bytes;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::DbRand;
    use slatedb_txn_obj::test_utils::new_dirty_object;
    use std::ops::Bound::{Excluded, Included};
    use std::sync::Arc;

    #[test]
    fn test_should_compact_l0s_to_first_sr() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = [
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
        ];
        let state =
            &create_compactor_state(create_db_state(l0.iter().cloned().collect(), Vec::new()));

        // when:
        let requests: Vec<CompactionSpec> = scheduler.propose(&state.into());

        // then:
        assert_eq!(requests.len(), 1);
        let request = requests.first().unwrap();
        let expected_sources: Vec<SourceId> = l0.iter().map(|h| SourceId::SstView(h.id)).collect();
        assert_eq!(request.sources(), &expected_sources);
        assert_eq!(request.destination(), Some(0));
    }

    #[test]
    fn test_supplier_overrides_scheduler_options() {
        // given:
        let supplier = SizeTieredCompactionSchedulerSupplier;
        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 2,
            max_compaction_sources: 2,
            ..Default::default()
        }
        .into();
        let compactor_options = CompactorOptions {
            scheduler_options,
            ..CompactorOptions::default()
        };
        let scheduler = supplier.compaction_scheduler(&compactor_options);
        let l0 = [create_sst_view(1), create_sst_view(1)];
        let state =
            &create_compactor_state(create_db_state(l0.iter().cloned().collect(), Vec::new()));

        // when:
        let requests: Vec<CompactionSpec> = scheduler.propose(&state.into());

        // then:
        assert_eq!(requests.len(), 1);
        let request = requests.first().unwrap();
        let expected_sources: Vec<SourceId> = l0.iter().map(|h| SourceId::SstView(h.id)).collect();
        assert_eq!(request.sources(), &expected_sources);
    }

    #[test]
    fn test_should_compact_l0s_to_new_sr() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = [
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
        ];
        let state = &create_compactor_state(create_db_state(
            l0.iter().cloned().collect(),
            vec![create_sr2(10, 2), create_sr2(0, 2)],
        ));

        // when:
        let requests: Vec<CompactionSpec> = scheduler.propose(&state.into());

        // then:
        assert_eq!(requests.len(), 1);
        let request = requests.first().unwrap();
        assert_eq!(request.destination(), Some(11));
    }

    #[test]
    fn test_should_not_compact_l0s_if_fewer_than_min_threshold() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = [create_sst_view(1), create_sst_view(1), create_sst_view(1)];
        let state = &create_compactor_state(create_db_state(l0.iter().cloned().collect(), vec![]));

        // when:
        let compactions = scheduler.propose(&state.into());

        // then:
        assert_eq!(compactions.len(), 0);
    }

    #[test]
    fn test_should_compact_srs_if_enough_with_similar_size() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(4, 2),
                create_sr2(3, 2),
                create_sr2(2, 2),
                create_sr2(1, 2),
                create_sr2(0, 2),
            ],
        ));

        // when:
        let compactions = scheduler.propose(&state.into());

        // then:
        assert_eq!(compactions.len(), 1);
        let compaction = compactions.first().unwrap();
        let expected_compaction = create_sr_compaction(vec![4, 3, 2, 1, 0]);

        assert_eq!(compaction.clone(), expected_compaction,)
    }

    #[test]
    fn test_should_only_include_srs_if_with_similar_size() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(4, 2),
                create_sr2(3, 2),
                create_sr2(2, 2),
                create_sr2(1, 2),
                create_sr2(0, 10),
            ],
        ));

        // when:
        let compactions = scheduler.propose(&state.into());

        // then:
        assert_eq!(compactions.len(), 1);

        let compaction = compactions.first().unwrap();
        let expected_compaction = create_sr_compaction(vec![4, 3, 2, 1]);
        assert_eq!(compaction.clone(), expected_compaction)
    }

    #[rstest::rstest]
    #[case::disabled(0)]
    #[case::invalid(1)]
    #[case::minimum_enabled(2)]
    fn test_consolidation_threshold_validation(
        #[case] threshold: usize,
        #[values(false, true)] from_map: bool,
        #[values(2, 3)] run_count: u32,
    ) {
        let options = SizeTieredCompactionSchedulerOptions {
            sorted_run_consolidation_threshold: threshold,
            ..Default::default()
        };
        let scheduler: Box<dyn CompactionScheduler + Send + Sync> = if from_map {
            SizeTieredCompactionSchedulerSupplier.compaction_scheduler(&CompactorOptions {
                scheduler_options: options.into(),
                ..Default::default()
            })
        } else {
            Box::new(SizeTieredCompactionScheduler::new(options, 4))
        };
        let state = create_compactor_state(create_db_state(
            VecDeque::new(),
            (0..run_count)
                .rev()
                .map(|id| create_sr2(id, 2 * 8_u64.pow(run_count - id - 1)))
                .collect(),
        ));

        let expected = if threshold == 2 && run_count == 3 {
            vec![create_sr_compaction(vec![2, 1])]
        } else {
            vec![]
        };
        assert_eq!(scheduler.propose(&(&state).into()), expected);
    }

    #[test]
    fn test_consolidation_disabled_is_noop_for_dissimilar_runs() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(4, 2),
                create_sr2(3, 16),
                create_sr2(2, 128),
                create_sr2(1, 1024),
                create_sr2(0, 8192),
            ],
        ));

        let compactions = scheduler.propose(&state.into());

        assert!(compactions.is_empty());
    }

    #[rstest::rstest]
    #[case::below_threshold(6, false)]
    #[case::at_threshold(5, false)]
    #[case::above_threshold(4, true)]
    fn test_consolidation_threshold(#[case] threshold: usize, #[case] should_consolidate: bool) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: threshold,
                ..SizeTieredCompactionSchedulerOptions::default()
            },
            4,
        );
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(4, 2),
                create_sr2(3, 16),
                create_sr2(2, 128),
                create_sr2(1, 1024),
                create_sr2(0, 8192),
            ],
        ));

        let compactions = scheduler.propose(&state.into());

        let expected = if should_consolidate {
            vec![create_sr_compaction(vec![4, 3])]
        } else {
            vec![]
        };
        assert_eq!(compactions, expected);
    }

    #[rstest::rstest]
    #[case::small_run(2)]
    #[case::empty_run(0)]
    fn test_consolidation_ignores_minimum_source_count_and_size_threshold(#[case] first_size: u64) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 2,
                ..Default::default()
            },
            1,
        );
        let state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(2, first_size),
                create_sr2(1, 1024),
                create_sr2(0, 8192),
            ],
        ));

        assert_eq!(
            scheduler.propose(&(&state).into()),
            vec![create_sr_compaction(vec![2, 1])]
        );
    }

    #[rstest::rstest]
    #[case::zero(0)]
    #[case::one(1)]
    #[case::two(2)]
    #[case::three(3)]
    #[case::remaining_reduction(5)]
    #[case::above_run_count(8)]
    fn test_consolidation_source_limit_keeps_newest_sources(#[case] limit: usize) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 2,
                max_compaction_sources: limit,
                ..Default::default()
            },
            1,
        );
        let state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(5, 0),
                create_sr2(4, 2),
                create_sr2(3, 16),
                create_sr2(2, 128),
                create_sr2(1, 1024),
                create_sr2(0, 8192),
            ],
        ));

        let expected = if limit < 2 {
            vec![]
        } else {
            vec![create_sr_compaction(
                (0..6).rev().take(limit.min(5)).collect(),
            )]
        };
        assert_eq!(scheduler.propose(&(&state).into()), expected);
    }

    #[rstest::rstest]
    #[case::newest_busy(vec![5, 4], vec![vec![3, 2]])]
    #[case::middle_busy(vec![3, 2], vec![vec![5, 4]])]
    #[case::all_busy(vec![5, 4, 3, 2, 1, 0], vec![])]
    fn test_consolidation_tries_later_groups_after_conflicts(
        #[case] busy: Vec<u32>,
        #[case] expected_groups: Vec<Vec<u32>>,
    ) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 4,
                ..Default::default()
            },
            4,
        );
        let mut state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(5, 2),
                create_sr2(4, 16),
                create_sr2(3, 128),
                create_sr2(2, 1024),
                create_sr2(1, 8192),
                create_sr2(0, 65536),
            ],
        ));
        state
            .add_compaction(Compaction::new(
                ulid::Ulid::new(),
                create_sr_compaction(busy),
            ))
            .unwrap();

        let compactions = scheduler.propose(&(&state).into());
        let expected: Vec<_> = expected_groups
            .into_iter()
            .map(create_sr_compaction)
            .collect();
        assert_eq!(compactions, expected);
        for compaction in compactions {
            scheduler.validate(&(&state).into(), &compaction).unwrap();
        }
    }

    #[rstest::rstest]
    #[case::submitted(CompactionStatus::Submitted, vec![5, 4])]
    #[case::scheduled(CompactionStatus::Scheduled, vec![5, 4])]
    #[case::running(CompactionStatus::Running, vec![5, 4])]
    #[case::compacted(CompactionStatus::Compacted, vec![5, 4])]
    #[case::completed(CompactionStatus::Completed, vec![5, 4, 3])]
    #[case::failed(CompactionStatus::Failed, vec![5, 4, 3])]
    fn test_consolidation_projects_only_active_compactions(
        #[case] status: CompactionStatus,
        #[case] expected_sources: Vec<u32>,
    ) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 4,
                ..Default::default()
            },
            4,
        );
        let mut state = create_compactor_state(create_db_state(
            VecDeque::new(),
            (0..6)
                .rev()
                .map(|id| create_sr2(id, 2 * 8_u64.pow(5 - id)))
                .collect(),
        ));
        state.insert_compaction_for_test(
            Compaction::new(ulid::Ulid::new(), create_sr_compaction(vec![3, 2]))
                .with_status(status),
        );

        let proposed = scheduler.propose(&(&state).into());
        assert_eq!(proposed, vec![create_sr_compaction(expected_sources)]);
        state
            .add_compaction(Compaction::new(ulid::Ulid::new(), proposed[0].clone()))
            .unwrap();
        assert!(scheduler.propose(&(&state).into()).is_empty());
    }

    #[test]
    fn test_consolidation_stops_after_new_picks_reach_threshold() {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 3,
                max_compaction_sources: 3,
                ..Default::default()
            },
            4,
        );
        let state = create_compactor_state(create_db_state(
            VecDeque::new(),
            (0..6)
                .rev()
                .map(|id| create_sr2(id, 2 * 8_u64.pow(5 - id)))
                .collect(),
        ));

        assert_eq!(
            scheduler.propose(&(&state).into()),
            vec![
                create_sr_compaction(vec![5, 4, 3]),
                create_sr_compaction(vec![2, 1]),
            ]
        );
    }

    #[test]
    fn test_normal_compaction_can_satisfy_consolidation_threshold() {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 4,
                ..Default::default()
            },
            4,
        );
        let state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(5, 2),
                create_sr2(4, 16),
                create_sr2(3, 1024),
                create_sr2(2, 1024),
                create_sr2(1, 1024),
                create_sr2(0, 1024),
            ],
        ));

        assert_eq!(
            scheduler.propose(&(&state).into()),
            vec![create_sr_compaction(vec![3, 2, 1, 0])]
        );
    }

    #[rstest::rstest]
    #[case::pending(true)]
    #[case::newly_selected(false)]
    fn test_consolidation_accounts_for_l0_output(#[case] pending: bool) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 3,
                ..Default::default()
            },
            4,
        );
        let l0: Vec<_> = (0..4).map(|_| create_sst_view(1)).collect();
        let l0_compaction = create_l0_compaction(&l0, 3);
        let mut state = create_compactor_state(create_db_state(
            l0.into_iter().collect(),
            vec![create_sr2(2, 2), create_sr2(1, 16), create_sr2(0, 128)],
        ));
        if pending {
            state
                .add_compaction(Compaction::new(ulid::Ulid::new(), l0_compaction.clone()))
                .unwrap();
        }

        let mut expected = Vec::new();
        if !pending {
            expected.push(l0_compaction);
        }
        expected.push(create_sr_compaction(vec![2, 1]));
        assert_eq!(scheduler.propose(&(&state).into()), expected);
    }

    #[rstest::rstest]
    #[case::sorted_run_output(false)]
    #[case::l0_output(true)]
    fn test_consolidation_does_not_count_committed_output_twice(#[case] l0_output: bool) {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 4,
                ..Default::default()
            },
            4,
        );
        let mut state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(6, 2),
                create_sr2(5, 16),
                create_sr2(4, 128),
                create_sr2(3, 1024),
                create_sr2(0, 8192),
            ],
        ));
        let sources = if l0_output {
            vec![SourceId::SstView(ulid::Ulid::new())]
        } else {
            vec![
                SourceId::SortedRun(2),
                SourceId::SortedRun(1),
                SourceId::SortedRun(0),
            ]
        };
        state.insert_compaction_for_test(
            Compaction::new(ulid::Ulid::new(), CompactionSpec::new(sources, 0))
                .with_status(CompactionStatus::Compacted),
        );

        assert_eq!(
            scheduler.propose(&(&state).into()),
            vec![create_sr_compaction(vec![6, 5])]
        );
    }

    #[test]
    fn test_consolidation_projects_segment_drains_separately() {
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold: 3,
                ..Default::default()
            },
            4,
        );
        let mut core = create_db_state(
            VecDeque::new(),
            (0..5)
                .rev()
                .map(|id| create_sr2(id, 2 * 8_u64.pow(4 - id)))
                .collect(),
        );
        core.segments = vec![segment_with(
            b"seg/",
            VecDeque::new(),
            (5..10)
                .rev()
                .map(|id| create_sr2(id, 2 * 8_u64.pow(9 - id)))
                .collect(),
        )];
        let mut state = create_compactor_state(core);
        state.insert_compaction_for_test(Compaction::new(
            ulid::Ulid::new(),
            CompactionSpec::drain_segment(
                Bytes::from_static(b"seg/"),
                vec![
                    SourceId::SortedRun(7),
                    SourceId::SortedRun(6),
                    SourceId::SortedRun(5),
                ],
            ),
        ));

        assert_eq!(
            scheduler.propose(&(&state).into()),
            vec![create_sr_compaction(vec![4, 3, 2])]
        );
    }

    #[test]
    fn test_should_not_schedule_compaction_for_source_that_is_already_compacting() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let mut state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(4, 2),
                create_sr2(3, 2),
                create_sr2(2, 2),
                create_sr2(1, 2),
                create_sr2(0, 2),
            ],
        ));
        let rand = Arc::new(DbRand::default());
        let system_clock = Arc::new(DefaultSystemClock::new());

        let job_id = rand.rng().gen_ulid(system_clock.as_ref());
        let request = create_sr_compaction(vec![3, 2, 1, 0]);
        let compactor_job = Compaction::new(job_id, request);

        state
            .add_compaction(compactor_job)
            .expect("failed to add job");

        // when:
        let requests = scheduler.propose(&(&state).into());

        // then:
        assert_eq!(requests.len(), 0);
    }

    #[test]
    fn test_compacted_job_does_not_consume_compaction_slot() {
        // A finished worker job in one tree is still waiting for its manifest
        // commit, while a disjoint tree has eligible work. With one worker
        // slot, the scheduler should immediately refill that slot.
        let scheduler =
            SizeTieredCompactionScheduler::new(SizeTieredCompactionSchedulerOptions::default(), 1);
        let root_l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        let segment_l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        let mut core = create_db_state(root_l0.iter().cloned().collect(), Vec::new());
        core.segments = vec![segment_with(
            b"finished/",
            segment_l0.iter().cloned().collect(),
            Vec::new(),
        )];
        let mut state = create_compactor_state(core);

        let completed_worker_job = Compaction::new(
            ulid::Ulid::new(),
            create_segment_l0_compaction(b"finished/", &segment_l0, 0),
        )
        .with_status(CompactionStatus::Compacted);
        state.insert_compaction_for_test(completed_worker_job);

        let requests = scheduler.propose(&(&state).into());

        assert_eq!(requests.len(), 1);
        assert!(requests[0].segment().is_empty());
        assert_eq!(requests[0].destination(), Some(1));
    }

    #[test]
    fn test_should_not_compact_srs_if_fewer_than_min_threshold() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![create_sr2(2, 2), create_sr2(1, 2), create_sr4(0, 2)],
        ));

        // when:
        let compactions = scheduler.propose(&state.into());

        // then:
        assert!(compactions.is_empty());
    }

    #[test]
    fn test_should_clamp_compaction_size() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(11, 2),
                create_sr2(10, 2),
                create_sr2(9, 2),
                create_sr2(8, 2),
                create_sr2(7, 2),
                create_sr2(6, 2),
                create_sr2(5, 2),
                create_sr2(4, 2),
                create_sr2(3, 2),
                create_sr2(2, 2),
                create_sr2(1, 2),
                create_sr4(0, 2),
            ],
        ));

        // when:
        let compactions = scheduler.propose(&state.into());
        let expected_compaction = create_sr_compaction(vec![7, 6, 5, 4, 3, 2, 1, 0]);

        // then:
        assert_eq!(compactions.len(), 1);
        assert_eq!(compactions.first().unwrap(), &expected_compaction,);
    }

    #[rstest::rstest]
    fn test_candidate_backpressure_uses_combined_size(
        #[values(None, Some(4.0))] size_threshold: Option<f32>,
    ) {
        let sources = [(3, 1), (2, 1), (1, 7), (0, 7)]
            .into_iter()
            .map(|(id, size)| CompactionSource {
                source: SourceId::SortedRun(id),
                size,
            })
            .collect::<Vec<_>>();
        let tree = super::TreeState {
            prefix: Bytes::new(),
            l0: Vec::new(),
            projected_sorted_runs: Default::default(),
            conflicts: super::ConflictChecker::new(std::iter::empty()),
            bp: super::BackpressureChecker::new(4.0, 2, &sources),
            srs: sources,
        };

        let candidate = SizeTieredCompactionScheduler::build_compactable_run(
            size_threshold,
            &tree.srs,
            0,
            Some(&tree),
            Some(2),
        );

        // Two one-byte runs reach the next size group together, but neither does alone.
        assert_eq!(
            candidate
                .iter()
                .map(|source| source.source)
                .collect::<Vec<_>>(),
            vec![SourceId::SortedRun(3)]
        );
    }

    #[rstest::rstest]
    #[case::busy_source(false)]
    #[case::reserved_destination(true)]
    fn test_candidate_stops_at_conflict(
        #[case] reserved_destination: bool,
        #[values(None, Some(4.0))] size_threshold: Option<f32>,
    ) {
        let source = if reserved_destination {
            SourceId::SstView(ulid::Ulid::new())
        } else {
            SourceId::SortedRun(2)
        };
        let busy = CompactionSpec::new(vec![source], 2);
        let sources = (1..=3)
            .rev()
            .map(|id| CompactionSource {
                source: SourceId::SortedRun(id),
                size: 1,
            })
            .collect::<Vec<_>>();
        let tree = super::TreeState {
            prefix: Bytes::new(),
            l0: Vec::new(),
            projected_sorted_runs: Default::default(),
            conflicts: super::ConflictChecker::new(std::iter::once(&busy)),
            bp: super::BackpressureChecker::new(4.0, 8, &sources),
            srs: sources,
        };

        let candidate = SizeTieredCompactionScheduler::build_compactable_run(
            size_threshold,
            &tree.srs,
            0,
            Some(&tree),
            Some(3),
        );

        assert_eq!(
            candidate
                .iter()
                .map(|source| source.source)
                .collect::<Vec<_>>(),
            vec![SourceId::SortedRun(3)]
        );
    }

    #[rstest::rstest]
    #[case::consolidation_disabled(0)]
    #[case::consolidation_enabled(4)]
    fn test_should_apply_backpressure(#[case] sorted_run_consolidation_threshold: usize) {
        // given:
        let scheduler = SizeTieredCompactionScheduler::new(
            SizeTieredCompactionSchedulerOptions {
                sorted_run_consolidation_threshold,
                ..Default::default()
            },
            4,
        );
        let mut state = create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                create_sr2(11, 2),
                create_sr2(10, 2),
                create_sr2(9, 2),
                create_sr2(8, 2),
                create_sr2(7, 2),
                create_sr2(6, 2),
                create_sr2(5, 2),
                create_sr2(4, 2),
                create_sr2(3, 2),
                create_sr2(2, 2),
                create_sr2(1, 2),
                create_sr4(0, 2),
            ],
        ));
        let rand = Arc::new(DbRand::default());
        let system_clock = Arc::new(DefaultSystemClock::new());

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let request = create_sr_compaction(vec![7, 6, 5, 4, 3, 2, 1, 0]);
        let compactor_job = Compaction::new(compaction_id, request);
        state
            .add_compaction(compactor_job)
            .expect("failed to add job");

        // when:
        let requests = scheduler.propose(&(&state).into());

        // then:
        assert!(requests.is_empty());
    }

    #[test]
    fn test_should_apply_backpressure_for_l0s() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = [
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
        ];
        let mut state = create_compactor_state(create_db_state(
            l0.iter().cloned().collect(),
            vec![
                create_sr2(7, 2),
                create_sr2(6, 2),
                create_sr2(5, 2),
                create_sr2(4, 2),
                create_sr2(3, 2),
                create_sr2(2, 2),
                create_sr2(1, 2),
                create_sr4(0, 2),
            ],
        ));
        let rand = Arc::new(DbRand::default());
        let system_clock = Arc::new(DefaultSystemClock::new());

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let compactor_job = Compaction::new(
            compaction_id,
            create_sr_compaction(vec![7, 6, 5, 4, 3, 2, 1, 0]),
        );
        state
            .add_compaction(compactor_job)
            .expect("failed to add job");

        // when:
        let compactions = scheduler.propose(&(&state).into());

        // then:
        assert!(compactions.is_empty());
    }

    #[test]
    fn test_should_return_multiple_compactions() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = vec![
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
        ];
        let state = &create_compactor_state(create_db_state(
            l0.iter().cloned().collect(),
            vec![
                create_sr2(10, 2),
                create_sr2(9, 2),
                create_sr2(8, 2),
                create_sr2(7, 2),
                create_sr4(3, 16),
                create_sr4(2, 16),
                create_sr4(1, 16),
                create_sr4(0, 16),
            ],
        ));

        // when:
        let compactions = scheduler.propose(&state.into());

        // then:
        assert_eq!(compactions.len(), 3);
        let compaction = compactions.first().unwrap();
        let expected_l0_compaction = create_l0_compaction(&l0, 11);
        assert_eq!(compaction.clone(), expected_l0_compaction);
        let compaction = compactions.get(1).unwrap();
        let expected_sr0_compaction = create_sr_compaction(vec![10, 9, 8, 7]);
        assert_eq!(compaction.clone(), expected_sr0_compaction);
        let compaction = compactions.get(2).unwrap();
        let expected_sr1_compaction = create_sr_compaction(vec![3, 2, 1, 0]);
        assert_eq!(compaction.clone(), expected_sr1_compaction);
    }

    #[test]
    fn test_should_submit_invalid_compaction_wrong_order() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = VecDeque::from(vec![
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
        ]);
        let state = &create_compactor_state(create_db_state(l0.clone(), Vec::new()));

        let mut l0_sst = l0;
        let last_sst = l0_sst.pop_back();
        l0_sst.push_front(last_sst.unwrap());
        // when:
        let compaction = create_l0_compaction(l0_sst.make_contiguous(), 0);
        let result = scheduler.validate(&state.into(), &compaction);

        // then:
        assert!(result.is_err());
    }

    #[test]
    fn test_should_submit_invalid_compaction_skipped_sst() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0 = VecDeque::from(vec![
            create_sst_view(1),
            create_sst_view(1),
            create_sst_view(1),
        ]);
        let state = &create_compactor_state(create_db_state(l0.clone(), Vec::new()));

        let mut l0_sst = l0;
        let last_sst = l0_sst.pop_back().unwrap();
        l0_sst.push_front(last_sst);
        l0_sst.pop_back();
        // when:
        let compaction = create_l0_compaction(l0_sst.make_contiguous(), 0);
        let result = scheduler.validate(&state.into(), &compaction);

        // then:
        assert!(result.is_err());
    }

    #[test]
    fn test_should_submit_invalid_compaction_with_sr() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let state =
            &create_compactor_state(create_db_state(VecDeque::new(), vec![create_sr2(0, 2)]));

        let mut l0 = state.db_state().tree.l0.clone();
        let request = create_l0_compaction(l0.make_contiguous(), 0);
        let mut new_sources: Vec<SourceId> = request.sources().to_vec();
        new_sources.push(SourceId::SortedRun(5));
        let new_request =
            CompactionSpec::new(new_sources, request.destination().expect("tiered spec"));
        // when:
        let result = scheduler.validate(&state.into(), &new_request);

        // then:
        assert!(result.is_err());
    }

    #[test]
    fn test_should_submit_valid_compaction_with_srs() {
        // given:
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![create_sr2(0, 2), create_sr2(1, 2)],
        ));

        let srs = state.db_state().tree.compacted.clone();
        let compaction = create_sr_compaction(srs.iter().map(|sr| sr.id).collect());
        // when:
        let result = scheduler.validate(&state.into(), &compaction);

        // then:
        assert!(result.is_err());
    }

    fn create_sst_view(size: u64) -> SsTableView {
        let info = SsTableInfo {
            first_entry: None,
            last_entry: None,
            index_offset: size,
            index_len: 0,
            filter_offset: 0,
            filter_len: 0,
            compression_codec: None,
            ..Default::default()
        };
        SsTableView::identity(SsTableHandle::new(
            SsTableId::from(ulid::Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            info,
        ))
    }

    fn create_sr2(id: u32, size: u64) -> SortedRun {
        create_sr(id, size / 2, 2)
    }

    fn create_sr4(id: u32, size: u64) -> SortedRun {
        create_sr(id, size / 4, 4)
    }

    fn create_sr(id: u32, sst_size: u64, num_ssts: usize) -> SortedRun {
        let ssts: Vec<SsTableView> = (0..num_ssts).map(|_| create_sst_view(sst_size)).collect();
        SortedRun::new(id, ssts)
    }

    fn create_sst_view_with_bounds(first: &[u8], last: &[u8], size: u64) -> SsTableView {
        let info = SsTableInfo {
            first_entry: Some(Bytes::copy_from_slice(first)),
            last_entry: Some(Bytes::copy_from_slice(last)),
            index_offset: size,
            index_len: 0,
            filter_offset: 0,
            filter_len: 0,
            compression_codec: None,
            ..Default::default()
        };
        SsTableView::identity(SsTableHandle::new(
            SsTableId::new(ulid::Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            info,
        ))
    }

    #[test]
    fn test_borrowed_sst_views_join_compactable_run() {
        let parent = create_sst_view_with_bounds(b"a", b"z", 4_000_000);
        let borrowed_first_half = parent.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(b"a")),
            Excluded(Bytes::copy_from_slice(b"m")),
        ));
        let borrowed_second_half = parent.with_visible_range(BytesRange::new(
            Included(Bytes::copy_from_slice(b"m")),
            Included(Bytes::copy_from_slice(b"z")),
        ));
        let peer = create_sst_view_with_bounds(b"aa", b"az", 600_000);

        let sources = vec![
            CompactionSource {
                source: SourceId::SstView(peer.id),
                size: peer.estimate_visible_size(),
            },
            CompactionSource {
                source: SourceId::SstView(borrowed_first_half.id),
                size: borrowed_first_half.estimate_visible_size(),
            },
            CompactionSource {
                source: SourceId::SstView(borrowed_second_half.id),
                size: borrowed_second_half.estimate_visible_size(),
            },
        ];

        let run = SizeTieredCompactionScheduler::build_compactable_run(
            Some(4.0),
            &sources,
            0,
            None,
            None,
        );

        assert_eq!(
            run.len(),
            3,
            "borrowed halves should scale down enough to join the peer's \
             compactable run within the 4.0x size threshold, got sizes {:?}",
            sources.iter().map(|s| s.size).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_propose_compacts_sorted_run_mixing_borrowed_and_plain_views() {
        let scheduler = SizeTieredCompactionScheduler::default();

        let plain_view = create_sst_view_with_bounds(b"a", b"z", 50_000);

        let prefix = b"tenant00";
        let phys_low = [prefix.as_slice(), &[0]].concat();
        let phys_high = [prefix.as_slice(), &[200]].concat();
        let vis_low = [prefix.as_slice(), &[0]].concat();
        let vis_high = [prefix.as_slice(), &[10]].concat();
        let borrowed_view = create_sst_view_with_bounds(&phys_low, &phys_high, 1_000_000)
            .with_visible_range(BytesRange::new(
                Included(Bytes::copy_from_slice(&vis_low)),
                Excluded(Bytes::copy_from_slice(&vis_high)),
            ));

        let mixed_sr = SortedRun::new(0, [plain_view, borrowed_view]);
        let mixed_sr_size = mixed_sr.estimate_visible_size();
        assert!(
            (80_000..120_000).contains(&mixed_sr_size),
            "expected the mixed sorted run's aggregate to land near \
             plain (50KB) + borrowed-visible (~50KB) = ~100KB, got {mixed_sr_size}"
        );

        let state = &create_compactor_state(create_db_state(
            VecDeque::new(),
            vec![
                mixed_sr,
                create_sr(1, 100_000, 1),
                create_sr(2, 100_000, 1),
                create_sr(3, 100_000, 1),
            ],
        ));

        let compactions = scheduler.propose(&state.into());

        assert_eq!(
            compactions.len(),
            1,
            "the mixed sorted run's sane aggregate should let it join a compactable run with its peers"
        );
        let compaction = compactions.first().unwrap();
        let expected_compaction = create_sr_compaction(vec![0, 1, 2, 3]);
        assert_eq!(compaction.clone(), expected_compaction);
    }

    fn create_db_state(l0: VecDeque<SsTableView>, srs: Vec<SortedRun>) -> ManifestCore {
        ManifestCore {
            initialized: true,
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0,
                compacted: srs,
            }),
            segments: vec![],
            segment_extractor_name: None,
            next_wal_sst_id: 0,
            replay_after_wal_id: 0,
            last_l0_seq: 0,
            last_l0_clock_tick: 0,
            checkpoints: vec![],
            wal_object_store_uri: None,
            recent_snapshot_min_seq: 0,
            sequence_tracker: SequenceTracker::new(),
        }
    }

    fn create_compactor_state(db_state: ManifestCore) -> CompactorState {
        let mut dirty = new_dirty_manifest();
        dirty.value.core = db_state;
        let compactions = new_dirty_object(1u64, Compactions::new(dirty.value.compactor_epoch));
        CompactorState::new(dirty, compactions)
    }

    fn create_l0_compaction(l0: &[SsTableView], dst: u32) -> CompactionSpec {
        let sources: Vec<SourceId> = l0.iter().map(|h| SourceId::SstView(h.id)).collect();

        CompactionSpec::new(sources, dst)
    }

    fn create_sr_compaction(srs: Vec<u32>) -> CompactionSpec {
        let sources: Vec<SourceId> = srs.iter().map(|sr| SourceId::SortedRun(*sr)).collect();
        CompactionSpec::new(sources, *srs.last().unwrap())
    }

    fn segment_with(prefix: &[u8], l0: VecDeque<SsTableView>, srs: Vec<SortedRun>) -> Segment {
        Segment {
            prefix: Bytes::copy_from_slice(prefix),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0,
                compacted: srs,
            }),
        }
    }

    /// A spec targeting a named segment with `dst` derived from sources, like
    /// `create_l0_compaction` but for a non-empty prefix.
    fn create_segment_l0_compaction(prefix: &[u8], l0: &[SsTableView], dst: u32) -> CompactionSpec {
        let sources: Vec<SourceId> = l0.iter().map(|h| SourceId::SstView(h.id)).collect();
        CompactionSpec::for_segment(Bytes::copy_from_slice(prefix), sources, dst)
    }

    /// A manifest with no L0/SR in the root tree and a single named segment
    /// holding the supplied L0/SR state.
    fn create_db_state_with_segment(segment: Segment) -> ManifestCore {
        let mut core = create_db_state(VecDeque::new(), Vec::new());
        core.segments = vec![segment];
        core
    }

    /// Per-segment proposal: a manifest with an empty root tree and a named
    /// segment holding enough L0s should yield a spec for the segment.
    #[test]
    fn test_should_propose_per_segment_l0_compaction() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        let segment = segment_with(b"hour=12/", l0.iter().cloned().collect(), Vec::new());
        let state = &create_compactor_state(create_db_state_with_segment(segment));

        let requests = scheduler.propose(&state.into());

        assert_eq!(requests.len(), 1);
        let spec = &requests[0];
        assert_eq!(spec.segment().as_ref(), b"hour=12/");
        let expected_sources: Vec<SourceId> = l0.iter().map(|h| SourceId::SstView(h.id)).collect();
        assert_eq!(spec.sources(), &expected_sources);
        assert_eq!(spec.destination(), Some(0));
    }

    /// Cross-tree priority: when two trees are both eligible, the tree with
    /// the larger L0 count gets scheduled first.
    #[test]
    fn test_should_prioritize_tree_with_more_l0s() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let root_l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        let seg_l0: Vec<SsTableView> = (0..6).map(|_| create_sst_view(1)).collect();
        let mut core = create_db_state(root_l0.iter().cloned().collect(), Vec::new());
        core.segments = vec![segment_with(
            b"hour=12/",
            seg_l0.iter().cloned().collect(),
            Vec::new(),
        )];
        let state = &create_compactor_state(core);

        let requests = scheduler.propose(&state.into());

        // Both trees produce an L0 → SR compaction; the segment (more L0s) is first.
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].segment().as_ref(), b"hour=12/");
        assert!(requests[1].segment().is_empty());
        // Globally-unique destination ids: segment gets 0, root gets 1.
        assert_eq!(requests[0].destination(), Some(0));
        assert_eq!(requests[1].destination(), Some(1));
    }

    /// Fresh L0 → SR destinations skip past every committed SR id in every
    /// tree. With root holding SR(10) and a segment holding SR(20), a new
    /// L0 → SR compaction in either tree must use id 21.
    #[test]
    fn test_should_allocate_globally_unique_sr_ids_across_trees() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let seg_l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        let mut core = create_db_state(VecDeque::new(), vec![create_sr2(10, 2)]);
        core.segments = vec![segment_with(
            b"seg/",
            seg_l0.iter().cloned().collect(),
            vec![create_sr2(20, 2)],
        )];
        let state = &create_compactor_state(core);

        let requests = scheduler.propose(&state.into());

        assert_eq!(requests.len(), 1);
        let spec = &requests[0];
        assert_eq!(spec.segment().as_ref(), b"seg/");
        assert_eq!(spec.destination(), Some(21));
    }

    /// Round-robin fairness: when one tree could absorb the entire budget on
    /// its own (L0 + multiple SR groups), the scheduler must still service
    /// every other tree's eligible work in the same `propose()` call.
    #[test]
    fn test_should_distribute_picks_across_trees_round_robin() {
        let scheduler = SizeTieredCompactionScheduler::default();
        // Root tree has L0 + three compactable SR groups: enough work to
        // absorb every slot in the budget on its own.
        let root_l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        let mut core = create_db_state(
            root_l0.iter().cloned().collect(),
            vec![
                create_sr2(11, 2),
                create_sr2(10, 2),
                create_sr2(9, 2),
                create_sr2(8, 2),
                create_sr4(7, 16),
                create_sr4(6, 16),
                create_sr4(5, 16),
                create_sr4(4, 16),
                create_sr4(3, 256),
                create_sr4(2, 256),
                create_sr4(1, 256),
                create_sr4(0, 256),
            ],
        );
        // Segment has only one L0 group, so it's only scheduled if the
        // round-robin pass reserves a slot for it.
        let seg_l0: Vec<SsTableView> = (0..4).map(|_| create_sst_view(1)).collect();
        core.segments = vec![segment_with(
            b"seg/",
            seg_l0.iter().cloned().collect(),
            Vec::new(),
        )];
        let state = &create_compactor_state(core);

        let requests = scheduler.propose(&state.into());

        // Default max_concurrent is 4. Round-robin must reserve at least one
        // slot for the segment even though the root tree could absorb all 4.
        assert_eq!(requests.len(), 4);
        let targets: Vec<_> = requests.iter().map(|s| s.segment().clone()).collect();
        assert!(
            targets.iter().any(|p| p.as_ref() == b"seg/"),
            "segment must receive at least one slot, got targets {:?}",
            targets,
        );
        assert!(
            targets.iter().any(|p| p.is_empty()),
            "root tree must still receive work too, got targets {:?}",
            targets,
        );
    }

    /// `validate` rejects a spec whose target segment does not exist in the
    /// manifest.
    #[test]
    fn test_validate_rejects_unknown_segment() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let state = &create_compactor_state(create_db_state(VecDeque::new(), Vec::new()));

        let spec = CompactionSpec::for_segment(Bytes::from_static(b"missing/"), vec![], 0);

        let result = scheduler.validate(&state.into(), &spec);

        assert!(result.is_err());
    }

    /// `validate` accepts a spec whose sources are consecutive entries in the
    /// target segment's tree, even when the root tree is empty.
    #[test]
    fn test_validate_accepts_consecutive_segment_sources() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0: Vec<SsTableView> = (0..3).map(|_| create_sst_view(1)).collect();
        let segment = segment_with(b"seg/", l0.iter().cloned().collect(), Vec::new());
        let state = &create_compactor_state(create_db_state_with_segment(segment));

        let spec = create_segment_l0_compaction(b"seg/", &l0, 0);

        scheduler
            .validate(&state.into(), &spec)
            .expect("segment-targeted spec with consecutive sources must validate");
    }

    /// `validate` rejects a spec whose sources come from the root tree but
    /// whose target segment is a named (non-root) prefix — the consecutive
    /// check runs against the wrong tree and must fail.
    #[test]
    fn test_validate_rejects_segment_spec_with_root_sources() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let root_l0: Vec<SsTableView> = (0..3).map(|_| create_sst_view(1)).collect();
        let mut core = create_db_state(root_l0.iter().cloned().collect(), Vec::new());
        core.segments = vec![segment_with(b"seg/", VecDeque::new(), Vec::new())];
        let state = &create_compactor_state(core);

        // Sources reference the root L0s; the spec targets the empty segment.
        let spec = create_segment_l0_compaction(b"seg/", &root_l0, 0);

        let result = scheduler.validate(&state.into(), &spec);

        assert!(result.is_err());
    }

    /// `validate` accepts drain specs without running any size-tiered policy
    /// checks. A drain spec has no destination and lists SRs to be retired,
    /// neither of which fits the tiered "merge into the lowest-id SR" rule.
    #[test]
    fn test_validate_accepts_drain_spec_with_sr_sources() {
        let scheduler = SizeTieredCompactionScheduler::default();
        let l0: Vec<SsTableView> = (0..2).map(|_| create_sst_view(1)).collect();
        let srs = vec![create_sr2(1, 2), create_sr2(0, 2)];
        let segment = segment_with(b"seg/", l0.iter().cloned().collect(), srs.clone());
        let state = &create_compactor_state(create_db_state_with_segment(segment));

        let mut sources: Vec<SourceId> = l0.iter().map(|h| SourceId::SstView(h.id)).collect();
        sources.extend(srs.iter().map(|sr| SourceId::SortedRun(sr.id)));
        let spec = CompactionSpec::drain_segment(Bytes::from_static(b"seg/"), sources);

        scheduler
            .validate(&state.into(), &spec)
            .expect("drain spec with SR sources must validate");
    }
}
