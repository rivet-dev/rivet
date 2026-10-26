use crate::compactor::{CompactionScheduler, CompactionSchedulerSupplier};
use crate::compactor_state::{CompactionSpec, SourceId};
use crate::compactor_state_protocols::CompactorStateView;
use crate::config::{CompactorOptions, PutOptions, WriteOptions};
use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView, SstType};
use crate::error::{RetryReason, SlateDBError};
use crate::format::row::SstRowCodecV0;
use crate::format::sst::SST_FORMAT_VERSION_LATEST;
use crate::iter::{IterationOrder, RowEntryIterator};
use crate::object_store_tag::ObjectStoreCallTag;
use crate::tablestore::{TableStore, TableStoreKind};
use crate::types::{KeyValue, RowEntry, ValueDeletable};
use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use futures::stream::BoxStream;
use futures::{stream, StreamExt};
use object_store::path::Path;
use object_store::{
    CopyOptions, GetOptions, GetRange, GetResult, ListResult, MultipartUpload, ObjectMeta,
    ObjectStore, PutMultipartOptions, PutOptions as OS_PutOptions, PutPayload, PutResult,
    RenameOptions,
};
use rand::{Rng, RngCore};
use std::cmp::Ordering as CmpOrdering;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::ops::{Bound, RangeBounds};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Once;
use std::thread;
use std::time::Duration;
use tokio::sync::Notify;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use ulid::Ulid;

pub(crate) fn bounded_sst_view(id: u64, first: &'static [u8], last: &'static [u8]) -> SsTableView {
    SsTableView::identity(SsTableHandle::new(
        SsTableId::from(Ulid::from_parts(id, 0)),
        SST_FORMAT_VERSION_LATEST,
        SsTableInfo {
            first_entry: Some(Bytes::from_static(first)),
            last_entry: Some(Bytes::from_static(last)),
            ..SsTableInfo::default()
        },
    ))
}

/// Asserts that the iterator returns the exact set of expected values in correct order.
pub(crate) async fn assert_iterator<T: RowEntryIterator>(iterator: &mut T, entries: Vec<RowEntry>) {
    iterator
        .init()
        .await
        .expect("iterator init failed in assert_iterator");
    for expected_entry in entries.iter() {
        assert_next(iterator, expected_entry).await;
    }
    assert!(iterator
        .next()
        .await
        .expect("iterator next failed")
        .is_none());
}

pub(crate) async fn assert_next<T: RowEntryIterator>(iterator: &mut T, expected_entry: &RowEntry) {
    iterator
        .init()
        .await
        .expect("iterator init failed in assert_next");
    let actual_entry = iterator
        .next()
        .await
        .expect("iterator next failed")
        .expect("expected iterator to return a value");
    assert_eq!(actual_entry, expected_entry.clone())
}

pub(crate) fn assert_kv(kv: &KeyValue, key: &[u8], val: &[u8]) {
    assert_eq!(kv.key, key);
    assert_eq!(kv.value, val);
}

#[derive(Clone, Debug)]
pub(crate) struct RecordedSpan {
    id: tracing::Id,
    pub(crate) name: String,
    pub(crate) level: String,
    pub(crate) parent_name: Option<String>,
    pub(crate) fields: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub(crate) struct SpanRecorder {
    span_name: Option<&'static str>,
    spans: Arc<std::sync::Mutex<Vec<RecordedSpan>>>,
}

impl SpanRecorder {
    pub(crate) fn for_name(span_name: &'static str) -> Self {
        Self {
            span_name: Some(span_name),
            spans: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn spans(&self) -> Vec<RecordedSpan> {
        self.spans
            .lock()
            .expect("The span recorder lock is poisoned.")
            .clone()
    }

    pub(crate) fn only_span(&self) -> RecordedSpan {
        let spans = self.spans();
        assert_eq!(
            spans.len(),
            1,
            "The recorder must contain one matching span."
        );
        spans[0].clone()
    }
}

struct SpanFieldRecorder<'a> {
    fields: &'a mut HashMap<String, String>,
}

impl tracing::field::Visit for SpanFieldRecorder<'_> {
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

impl<S> Layer<S> for SpanRecorder
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::Id,
        ctx: Context<'_, S>,
    ) {
        if self
            .span_name
            .is_some_and(|span_name| attrs.metadata().name() != span_name)
        {
            return;
        }
        let mut fields = HashMap::new();
        attrs.record(&mut SpanFieldRecorder {
            fields: &mut fields,
        });

        let explicit_parent = attrs.parent().and_then(|id| ctx.span(id));
        let contextual_parent = ctx.current_span().id().and_then(|id| ctx.span(id));
        let parent_name = explicit_parent
            .or(contextual_parent)
            .map(|span| span.metadata().name().to_string());

        self.spans
            .lock()
            .expect("The span recorder lock is poisoned.")
            .push(RecordedSpan {
                id: id.clone(),
                name: attrs.metadata().name().to_string(),
                level: attrs.metadata().level().to_string(),
                parent_name,
                fields,
            });
    }

    fn on_record(
        &self,
        id: &tracing::Id,
        values: &tracing::span::Record<'_>,
        _ctx: Context<'_, S>,
    ) {
        let mut fields = HashMap::new();
        values.record(&mut SpanFieldRecorder {
            fields: &mut fields,
        });
        let mut spans = self
            .spans
            .lock()
            .expect("The span recorder lock is poisoned.");
        if let Some(span) = spans.iter_mut().find(|span| span.id == *id) {
            span.fields.extend(fields);
        }
    }
}

pub(crate) struct TestIterator {
    entries: VecDeque<Result<RowEntry, SlateDBError>>,
}

impl TestIterator {
    pub(crate) fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    pub(crate) fn with_entry(mut self, key: &'static [u8], val: &'static [u8], seq: u64) -> Self {
        let entry = RowEntry::new_value(key, val, seq);
        self.entries.push_back(Ok(entry));
        self
    }

    pub(crate) fn with_row_entry(mut self, entry: RowEntry) -> Self {
        self.entries.push_back(Ok(entry));
        self
    }
}

#[async_trait]
impl RowEntryIterator for TestIterator {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        self.entries.pop_front().map_or(Ok(None), |e| match e {
            Ok(kv) => Ok(Some(kv)),
            Err(err) => Err(err),
        })
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        while let Some(entry_result) = self.entries.front() {
            let entry = entry_result.clone()?;
            if entry.key < next_key {
                self.entries.pop_front();
            } else {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) fn gen_rand_bytes(n: usize) -> Bytes {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..n).map(|_| rng.random::<u8>()).collect();
    Bytes::from(random_bytes)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ExtensionMarker;

#[derive(Clone)]
pub(crate) struct ExtensionObjectStore {
    inner: Arc<dyn ObjectStore>,
}

impl ExtensionObjectStore {
    pub(crate) fn new(inner: Arc<dyn ObjectStore>) -> Self {
        Self { inner }
    }
}

impl fmt::Debug for ExtensionObjectStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExtensionObjectStore({})", self.inner)
    }
}

impl fmt::Display for ExtensionObjectStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExtensionObjectStore({})", self.inner)
    }
}

#[async_trait]
impl ObjectStore for ExtensionObjectStore {
    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        let mut result = self.inner.get_opts(location, options).await?;
        result.extensions.insert(ExtensionMarker);
        Ok(result)
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: OS_PutOptions,
    ) -> object_store::Result<PutResult> {
        let mut result = self.inner.put_opts(location, payload, opts).await?;
        result.extensions.insert(ExtensionMarker);
        Ok(result)
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        self.inner.put_multipart_opts(location, opts).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        self.inner.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.inner.copy_opts(from, to, options).await
    }

    async fn rename_opts(
        &self,
        from: &Path,
        to: &Path,
        options: RenameOptions,
    ) -> object_store::Result<()> {
        self.inner.rename_opts(from, to, options).await
    }
}

// it seems that insta still does not allow to customize the snapshot path in insta.yaml,
// we can remove this macro once insta supports it.
macro_rules! assert_debug_snapshot {
    ($name:expr, $output:expr) => {
        let mut settings = insta::Settings::clone_current();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/snapshots");
        settings.set_snapshot_path(path);
        settings.bind(|| insta::assert_debug_snapshot!($name, $output));
    };
}

use crate::bytes_generator::OrderedBytesGenerator;
use crate::bytes_range::BytesRange;
use crate::db::Db;
use crate::db_iter::DbIterator;
use crate::format::sst::{EncodedSsTable, SsTableFormat};
use crate::merge_operator::{MERGE_OPERATOR_OPERANDS, MERGE_OPERATOR_PATH_LABEL};
use crate::{MergeOperator, MergeOperatorError};
pub(crate) use assert_debug_snapshot;
use slatedb_common::metrics::{lookup_metric_with_labels, DefaultMetricsRecorder};

pub(crate) fn decode_codec_entries(
    data: Bytes,
    offsets: &[u16],
) -> Result<Vec<RowEntry>, SlateDBError> {
    let codec = SstRowCodecV0::new();
    let mut entries = Vec::new();
    let mut last_key = Bytes::new(); // Track the last full key

    for &offset in offsets {
        let mut cursor = data.slice(offset as usize..);
        let sst_row_entry = codec.decode(&mut cursor)?;

        let full_key = sst_row_entry.restore_full_key(&last_key);

        // Update last_key for the next entry
        last_key = full_key.clone();

        let row_entry = RowEntry {
            key: full_key,
            value: sst_row_entry.value,
            seq: sst_row_entry.seq,
            create_ts: sst_row_entry.create_ts,
            expire_ts: sst_row_entry.expire_ts,
        };
        entries.push(row_entry);
    }

    Ok(entries)
}

pub(crate) async fn assert_ranged_db_scan<T: RangeBounds<Bytes>>(
    table: &BTreeMap<Bytes, Bytes>,
    range: T,
    ordering: IterationOrder,
    iter: &mut DbIterator,
) {
    let mut expected = table.range(range);
    loop {
        let expected_next = match ordering {
            IterationOrder::Ascending => expected.next(),
            IterationOrder::Descending => expected.next_back(),
        };
        let actual_next = iter.next().await.unwrap();
        if expected_next.is_none() && actual_next.is_none() {
            return;
        }
        assert_next_kv(expected_next, actual_next);
    }
}

pub(crate) fn lookup_merge_operator_operands(
    recorder: &DefaultMetricsRecorder,
    path: &'static str,
) -> Option<i64> {
    lookup_metric_with_labels(
        recorder,
        MERGE_OPERATOR_OPERANDS,
        &[(MERGE_OPERATOR_PATH_LABEL, path)],
    )
}

pub(crate) async fn assert_ranged_kv_scan<T: RowEntryIterator>(
    table: &BTreeMap<Bytes, Bytes>,
    range: &BytesRange,
    ordering: IterationOrder,
    iter: &mut T,
) {
    let mut expected = table.range((range.start_bound().cloned(), range.end_bound().cloned()));

    loop {
        let expected_next = match ordering {
            IterationOrder::Ascending => expected.next(),
            IterationOrder::Descending => expected.next_back(),
        };
        let actual_next = iter.next().await.unwrap().map(KeyValue::from);
        if expected_next.is_none() && actual_next.is_none() {
            return;
        }

        assert_next_kv(expected_next, actual_next);
    }
}

fn assert_next_kv(expected: Option<(&Bytes, &Bytes)>, actual: Option<KeyValue>) {
    match (expected, actual) {
        (None, None) => (),
        (Some((expected_key, expected_value)), Some(actual)) => {
            assert_eq!(expected_key, &actual.key);
            assert_eq!(expected_value, &actual.value);
        }
        (Some(expected_record), None) => {
            panic!("Expected record {expected_record:?} missing from scan result")
        }
        (None, Some(actual)) => panic!("Unexpected record {actual:?} in scan result"),
    }
}

pub(crate) fn bound_as_option<T>(bound: Bound<&T>) -> Option<&T>
where
    T: ?Sized,
{
    match bound {
        Included(b) | Excluded(b) => Some(b),
        Unbounded => None,
    }
}

pub(crate) async fn seed_database(
    db: &Db,
    table: &BTreeMap<Bytes, Bytes>,
    wait_for_durability: bool,
) -> Result<(), crate::Error> {
    let put_options = PutOptions::default();
    let write_options = WriteOptions::default();
    let mut last_handle = None;

    for (key, value) in table.iter() {
        last_handle = Some(
            db.put_with_options(key, value, &put_options, &write_options)
                .await?,
        );
    }

    if wait_for_durability {
        if let Some(handle) = last_handle {
            handle.await_durable().await?;
        }
    }

    Ok(())
}

pub(crate) async fn build_test_sst(format: &SsTableFormat, num_blocks: usize) -> EncodedSsTable {
    let mut rng = rand::rng();
    let mut keygen = OrderedBytesGenerator::new_with_suffix(&[], &[0u8; 16]);
    let mut encoded_sst_builder = format.table_builder();
    while encoded_sst_builder.num_blocks() < num_blocks {
        let k = keygen.next();
        let mut val = BytesMut::with_capacity(32);
        val.put_bytes(0u8, 32);
        rng.fill_bytes(&mut val);
        let row = RowEntry::new(k, ValueDeletable::Value(val.freeze()), 0u64, None, None);
        encoded_sst_builder.add(row).await.unwrap();
    }
    encoded_sst_builder.build().await.unwrap()
}

/// Builds RowEntry values from (key, seq, value) tuples and sorts by key asc/seq desc.
pub(crate) fn build_row_entries(specs: Vec<(Bytes, u64, ValueDeletable)>) -> Vec<RowEntry> {
    let mut entries = specs
        .into_iter()
        .map(|(key, seq, value)| RowEntry::new(key, value, seq, None, None))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| match left.key.cmp(&right.key) {
        CmpOrdering::Equal => right.seq.cmp(&left.seq),
        other => other,
    });
    entries
}

/// Writes entries into SSTs, splitting when the accumulated block size exceeds `max_sst_size`.
pub(crate) async fn write_ssts(
    table_store: &Arc<TableStore>,
    entries: &[RowEntry],
    max_sst_size: usize,
) -> Vec<SsTableHandle> {
    if entries.is_empty() {
        return Vec::new();
    }

    let mut output_ssts = Vec::new();
    let mut writer = table_store.table_writer(SsTableId::from(Ulid::new()), Some(Bytes::new()));
    let mut bytes_written = 0usize;

    for (index, entry) in entries.iter().cloned().enumerate() {
        if let Some(block_size) = writer.add(entry).await.unwrap() {
            bytes_written += block_size;
        }

        if bytes_written > max_sst_size {
            output_ssts.push(writer.close().await.unwrap().0);
            bytes_written = 0;

            if index + 1 < entries.len() {
                writer = table_store.table_writer(SsTableId::from(Ulid::new()), Some(Bytes::new()));
            } else {
                return output_ssts;
            }
        }
    }

    output_ssts.push(writer.close().await.unwrap().0);
    output_ssts
}

/// Builds SortedRun inputs from nested entry sets, writing each set as one or more SSTs.
pub(crate) async fn build_sorted_runs(
    table_store: &Arc<TableStore>,
    sr_entry_sets: &[Vec<Vec<RowEntry>>],
    max_sst_size: usize,
) -> Vec<SortedRun> {
    let mut sorted_runs = Vec::new();

    for (sr_id, sr_sst_sets) in sr_entry_sets.iter().enumerate() {
        let mut sr_ssts = Vec::new();
        for entries in sr_sst_sets {
            let ssts = write_ssts(table_store, entries, max_sst_size).await;
            sr_ssts.extend(ssts.into_iter().map(SsTableView::identity));
        }
        sorted_runs.push(SortedRun::new(sr_id as u32, sr_ssts));
    }

    sorted_runs
}

/// A compactor that compacts if there are L0s and `should_compact` returns true.
/// All SSTs from L0 and all sorted runs are always compacted into sorted run 0.
#[derive(Clone)]
pub(crate) struct OnDemandCompactionScheduler {
    pub(crate) should_compact: Arc<dyn Fn(&CompactorStateView) -> bool + Send + Sync>,
}

impl OnDemandCompactionScheduler {
    fn new(should_compact: Arc<dyn Fn(&CompactorStateView) -> bool + Send + Sync>) -> Self {
        Self { should_compact }
    }
}

impl CompactionScheduler for OnDemandCompactionScheduler {
    fn propose(&self, state: &CompactorStateView) -> Vec<CompactionSpec> {
        if !(self.should_compact)(state) {
            return vec![];
        }

        let db_state = state.manifest().core();

        // always compact into sorted run 0
        let next_sr_id = 0;

        // Create a compaction of all SSTs from L0 and all sorted runs
        let mut sources: Vec<SourceId> = db_state
            .tree
            .l0
            .iter()
            .map(|view| SourceId::SstView(view.id))
            .collect();

        // Add SSTs from all sorted runs
        for sr in &db_state.tree.compacted {
            sources.push(SourceId::SortedRun(sr.id));
        }

        vec![CompactionSpec::new(sources, next_sr_id)]
    }
}

pub(crate) struct OnDemandCompactionSchedulerSupplier {
    pub(crate) scheduler: OnDemandCompactionScheduler,
}

impl OnDemandCompactionSchedulerSupplier {
    pub(crate) fn new(
        should_compact: Arc<dyn Fn(&CompactorStateView) -> bool + Send + Sync>,
    ) -> Self {
        Self {
            scheduler: OnDemandCompactionScheduler::new(should_compact),
        }
    }
}

impl CompactionSchedulerSupplier for OnDemandCompactionSchedulerSupplier {
    fn compaction_scheduler(
        &self,
        _compactor_options: &CompactorOptions,
    ) -> Box<dyn CompactionScheduler + Send + Sync> {
        Box::new(self.scheduler.clone())
    }
}

/// An ObjectStore wrapper that injects a transient timeout on the first N `put_opts` calls
/// to exercise retry logic. All operations delegate to the inner store otherwise.
#[derive(Debug)]
pub(crate) struct FlakyObjectStore {
    inner: Arc<dyn ObjectStore>,
    // Put options: transient failures on first N attempts
    fail_first_put_opts: AtomicUsize,
    put_opts_attempts: AtomicUsize,
    put_opts_attempt_notify: Notify,
    max_single_put_bytes: AtomicUsize,
    put_multipart_attempts: AtomicUsize,
    // Put options: if set, always return Precondition error (non-retryable)
    put_precondition_always: std::sync::atomic::AtomicBool,
    // Put options: if set, write succeeds but returns AlreadyExists error
    // This simulates a timeout that occurs after the write completes but before the response
    put_succeeds_but_returns_already_exists: std::sync::atomic::AtomicBool,
    // Head: transient failures on first N attempts
    fail_first_head: AtomicUsize,
    head_attempts: AtomicUsize,
    // List: transient failures on first N `list` calls
    fail_first_list: AtomicUsize,
    // List: on a failing call, inject an error after this many successful items
    list_fail_after: AtomicUsize,
    // List: total number of `list` invocations
    list_attempts: AtomicUsize,
    // List with offset: transient failures on first N `list_with_offset` calls
    fail_first_list_with_offset: AtomicUsize,
    // List with offset: on a failing call, inject an error after this many successful items
    list_with_offset_fail_after: AtomicUsize,
    // List with offset: total number of `list_with_offset` invocations
    list_with_offset_attempts: AtomicUsize,
    // get_range: transient failures on first N attempts
    fail_first_get_range: AtomicUsize,
    get_range_attempts: AtomicUsize,
    // get_range: truncate response body to this many bytes on first N attempts (0 = no truncation)
    truncate_get_range_bytes: AtomicUsize,
    truncate_get_range_count: AtomicUsize,
    // Get: return NotFound on the next N GETs
    fail_first_get_not_found: AtomicUsize,
}

impl FlakyObjectStore {
    pub(crate) fn new(inner: Arc<dyn ObjectStore>, fail_first_put_opts: usize) -> Self {
        Self {
            inner,
            fail_first_put_opts: AtomicUsize::new(fail_first_put_opts),
            put_opts_attempts: AtomicUsize::new(0),
            put_opts_attempt_notify: Notify::new(),
            max_single_put_bytes: AtomicUsize::new(0),
            put_multipart_attempts: AtomicUsize::new(0),
            put_precondition_always: std::sync::atomic::AtomicBool::new(false),
            put_succeeds_but_returns_already_exists: std::sync::atomic::AtomicBool::new(false),
            fail_first_head: AtomicUsize::new(0),
            head_attempts: AtomicUsize::new(0),
            fail_first_list: AtomicUsize::new(0),
            list_fail_after: AtomicUsize::new(0),
            list_attempts: AtomicUsize::new(0),
            fail_first_list_with_offset: AtomicUsize::new(0),
            list_with_offset_fail_after: AtomicUsize::new(0),
            list_with_offset_attempts: AtomicUsize::new(0),
            fail_first_get_range: AtomicUsize::new(0),
            get_range_attempts: AtomicUsize::new(0),
            truncate_get_range_bytes: AtomicUsize::new(0),
            truncate_get_range_count: AtomicUsize::new(0),
            fail_first_get_not_found: AtomicUsize::new(0),
        }
    }

    pub(crate) fn with_get_not_found_failures(&self, n: usize) {
        self.fail_first_get_not_found.store(n, Ordering::SeqCst);
    }

    pub(crate) fn with_put_precondition_always(self) -> Self {
        self.put_precondition_always.store(true, Ordering::SeqCst);
        self
    }

    pub(crate) fn with_single_put_size_limit(self, max_single_put_bytes: usize) -> Self {
        self.max_single_put_bytes
            .store(max_single_put_bytes, Ordering::SeqCst);
        self
    }

    /// Configure the store to write data successfully but return an AlreadyExists error.
    /// This simulates a timeout that occurs after the write completes but before the response.
    pub(crate) fn with_put_succeeds_but_returns_already_exists(self) -> Self {
        self.put_succeeds_but_returns_already_exists
            .store(true, Ordering::SeqCst);
        self
    }

    pub(crate) fn with_head_failures(self, n: usize) -> Self {
        self.fail_first_head.store(n, Ordering::SeqCst);
        self
    }

    pub(crate) fn with_list_failures(self, fail_times: usize, fail_after: usize) -> Self {
        self.fail_first_list.store(fail_times, Ordering::SeqCst);
        self.list_fail_after.store(fail_after, Ordering::SeqCst);
        self
    }

    pub(crate) fn with_list_with_offset_failures(
        self,
        fail_times: usize,
        fail_after: usize,
    ) -> Self {
        self.fail_first_list_with_offset
            .store(fail_times, Ordering::SeqCst);
        self.list_with_offset_fail_after
            .store(fail_after, Ordering::SeqCst);
        self
    }

    pub(crate) fn put_attempts(&self) -> usize {
        self.put_opts_attempts.load(Ordering::SeqCst)
    }

    pub(crate) fn multipart_attempts(&self) -> usize {
        self.put_multipart_attempts.load(Ordering::SeqCst)
    }

    pub(crate) async fn wait_for_put_attempts(&self, expected: usize) {
        loop {
            let notified = self.put_opts_attempt_notify.notified();
            if self.put_attempts() >= expected {
                return;
            }
            notified.await;
        }
    }

    pub(crate) fn head_attempts(&self) -> usize {
        self.head_attempts.load(Ordering::SeqCst)
    }

    pub(crate) fn list_attempts(&self) -> usize {
        self.list_attempts.load(Ordering::SeqCst)
    }

    pub(crate) fn list_with_offset_attempts(&self) -> usize {
        self.list_with_offset_attempts.load(Ordering::SeqCst)
    }

    pub(crate) fn with_get_range_failures(self, n: usize) -> Self {
        self.fail_first_get_range.store(n, Ordering::SeqCst);
        self
    }

    pub(crate) fn with_truncate_get_range_bytes(self, bytes: usize, count: usize) -> Self {
        self.truncate_get_range_bytes.store(bytes, Ordering::SeqCst);
        self.truncate_get_range_count.store(count, Ordering::SeqCst);
        self
    }

    pub(crate) fn get_range_attempts(&self) -> usize {
        self.get_range_attempts.load(Ordering::SeqCst)
    }

    /// Inject a failure after `fail_after` successful items in the stream.
    ///
    /// ## Args
    /// * `stream`: The stream to inject the failure into
    /// * `fail_after`: The number of successful items to yield before injecting the failure
    /// * `store_label`: The label to use in the error
    /// * `message`: The message to use in the error
    fn fail_stream(
        stream: BoxStream<'static, object_store::Result<ObjectMeta>>,
        fail_after: usize,
        store_label: &'static str,
        message: &'static str,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        stream
            .take(fail_after)
            .chain(stream::once(async move {
                Err(object_store::Error::Generic {
                    store: store_label,
                    source: Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, message)),
                })
            }))
            .boxed()
    }
}

impl fmt::Display for FlakyObjectStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FlakyObjectStore({})", self.inner)
    }
}

#[async_trait::async_trait]
impl ObjectStore for FlakyObjectStore {
    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        if self
            .fail_first_get_not_found
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                if v > 0 {
                    Some(v - 1)
                } else {
                    None
                }
            })
            .is_ok()
        {
            return Err(object_store::Error::NotFound {
                path: location.to_string(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "injected not-found (deleted between LIST and GET)",
                )),
            });
        }
        if options.head {
            self.head_attempts.fetch_add(1, Ordering::SeqCst);
            if self
                .fail_first_head
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                    if v > 0 {
                        Some(v - 1)
                    } else {
                        None
                    }
                })
                .is_ok()
            {
                return Err(object_store::Error::Generic {
                    store: "flaky_head",
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "injected head timeout",
                    )),
                });
            }
        } else if options.range.is_some() {
            self.get_range_attempts.fetch_add(1, Ordering::SeqCst);
            let should_fail = self
                .fail_first_get_range
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                    if v > 0 {
                        Some(v - 1)
                    } else {
                        None
                    }
                })
                .is_ok();
            if should_fail {
                return Err(object_store::Error::Generic {
                    store: "flaky_get_range",
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "injected get_range timeout",
                    )),
                });
            }
            let truncate_bytes = self.truncate_get_range_bytes.load(Ordering::SeqCst);
            let should_truncate = truncate_bytes > 0
                && self
                    .truncate_get_range_count
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                        if v > 0 {
                            Some(v - 1)
                        } else {
                            None
                        }
                    })
                    .is_ok();
            if should_truncate {
                let result = self.inner.get_opts(location, options).await?;
                let meta = result.meta.clone();
                let range = result.range.clone();
                let attributes = result.attributes.clone();
                let extensions = result.extensions.clone();
                let body = result.bytes().await?;
                let truncated = body.slice(..truncate_bytes.min(body.len()));
                return Ok(GetResult {
                    payload: object_store::GetResultPayload::Stream(
                        stream::once(async { Ok(truncated) }).boxed(),
                    ),
                    meta,
                    range,
                    attributes,
                    extensions,
                });
            }
        }
        self.inner.get_opts(location, options).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: OS_PutOptions,
    ) -> object_store::Result<PutResult> {
        self.put_opts_attempts.fetch_add(1, Ordering::SeqCst);
        self.put_opts_attempt_notify.notify_waiters();
        let max_single_put_bytes = self.max_single_put_bytes.load(Ordering::SeqCst);
        if max_single_put_bytes > 0 && payload.content_length() > max_single_put_bytes {
            return Err(object_store::Error::Generic {
                store: "flaky_single_put_limit",
                source: Box::new(std::io::Error::other(format!(
                    "single PUT payload of {} bytes exceeds limit of {} bytes",
                    payload.content_length(),
                    max_single_put_bytes
                ))),
            });
        }
        if self.put_precondition_always.load(Ordering::SeqCst) {
            return Err(object_store::Error::Precondition {
                path: location.to_string(),
                source: Box::new(std::io::Error::other("injected precondition")),
            });
        }
        // Simulate: write succeeds but returns AlreadyExists error (timeout after write)
        if self
            .put_succeeds_but_returns_already_exists
            .load(Ordering::SeqCst)
        {
            // Actually write the data
            let _ = self.inner.put_opts(location, payload, opts).await?;
            // But return an error as if we didn't get the response
            return Err(object_store::Error::AlreadyExists {
                path: location.to_string(),
                source: Box::new(std::io::Error::other(
                    "injected already exists after successful write",
                )),
            });
        }
        if self
            .fail_first_put_opts
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                if v > 0 {
                    Some(v - 1)
                } else {
                    None
                }
            })
            .is_ok()
        {
            // Inject a timeout error wrapped in object_store::Error so retry logic triggers
            return Err(object_store::Error::Generic {
                store: "flaky",
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "injected timeout",
                )),
            });
        }
        self.inner.put_opts(location, payload, opts).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        self.put_multipart_attempts.fetch_add(1, Ordering::SeqCst);
        self.inner.put_multipart_opts(location, opts).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        self.inner.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.list_attempts.fetch_add(1, Ordering::SeqCst);
        if self
            .fail_first_list
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                if v > 0 {
                    Some(v - 1)
                } else {
                    None
                }
            })
            .is_ok()
        {
            let fail_after = self.list_fail_after.load(Ordering::SeqCst);
            return Self::fail_stream(
                self.inner.list(prefix),
                fail_after,
                "flaky_list",
                "injected list failure",
            );
        }
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.list_with_offset_attempts
            .fetch_add(1, Ordering::SeqCst);
        if self
            .fail_first_list_with_offset
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                if v > 0 {
                    Some(v - 1)
                } else {
                    None
                }
            })
            .is_ok()
        {
            let fail_after = self.list_with_offset_fail_after.load(Ordering::SeqCst);
            return Self::fail_stream(
                self.inner.list_with_offset(prefix, offset),
                fail_after,
                "flaky_list_with_offset",
                "injected list_with_offset failure",
            );
        }
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.inner.copy_opts(from, to, options).await
    }
}

/// A per-operation gate for deterministic concurrency testing.
///
/// Each gate starts **open** (pass-through). Call [`close()`](Self::close) to block
/// callers, then [`release()`](Self::release) to let them proceed. Use
/// [`set_error`](Self::set_error) before releasing to inject a specific error.
pub(crate) struct Gate {
    open: std::sync::atomic::AtomicBool,
    /// Callers waiting at a closed gate may pass by consuming one permit
    /// (see [`admit`](Self::admit)).
    permits: AtomicUsize,
    arrival_count: AtomicUsize,
    /// If `Some`, callers receive the produced error after the gate opens.
    error_fn: std::sync::Mutex<Option<Box<dyn Fn() -> object_store::Error + Send + Sync>>>,
}

impl fmt::Debug for Gate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gate")
            .field("open", &self.open)
            .field("arrival_count", &self.arrival_count)
            .field("has_error", &self.error_fn.lock().unwrap().is_some())
            .finish()
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self {
            open: std::sync::atomic::AtomicBool::new(true),
            permits: AtomicUsize::new(0),
            arrival_count: AtomicUsize::new(0),
            error_fn: std::sync::Mutex::new(None),
        }
    }
}

impl Gate {
    /// Create a gate that starts closed (callers will block).
    #[allow(dead_code)]
    pub(crate) fn closed() -> Self {
        Self {
            open: std::sync::atomic::AtomicBool::new(false),
            ..Default::default()
        }
    }

    /// Close the gate so subsequent callers block.
    pub(crate) fn close(&self) {
        self.open.store(false, Ordering::Release);
    }

    /// Open the gate, allowing all blocked (and future) callers to proceed.
    pub(crate) fn release(&self) {
        self.open.store(true, Ordering::Release);
    }

    /// Let exactly `n` callers through a closed gate. Blocked (and future)
    /// callers each consume one permit to pass; once the permits are used the
    /// gate blocks again. Permits are ignored while the gate is open.
    #[allow(dead_code)]
    pub(crate) fn admit(&self, n: usize) {
        self.permits.fetch_add(n, Ordering::AcqRel);
    }

    /// Set an error factory. After the gate opens, callers will receive the
    /// error produced by `f` instead of proceeding to the inner store.
    pub(crate) fn set_error<F>(&self, f: F)
    where
        F: Fn() -> object_store::Error + Send + Sync + 'static,
    {
        *self.error_fn.lock().unwrap() = Some(Box::new(f));
    }

    /// Clear any previously set error, restoring success behavior.
    pub(crate) fn clear_error(&self) {
        *self.error_fn.lock().unwrap() = None;
    }

    /// Wait until at least `n` callers have arrived at the gate.
    pub(crate) async fn wait_for_arrivals(&self, n: usize) {
        while self.arrival_count.load(Ordering::Acquire) < n {
            tokio::task::yield_now().await;
        }
    }

    /// How many callers have arrived at the gate (cumulative).
    pub(crate) fn arrivals(&self) -> usize {
        self.arrival_count.load(Ordering::Acquire)
    }

    /// Block at this gate until released or admitted. Returns the configured
    /// error (if any) or `Ok(())` to let the caller proceed to the inner store.
    async fn wait(&self) -> object_store::Result<()> {
        // Signal arrival.
        self.arrival_count.fetch_add(1, Ordering::AcqRel);

        // Spin-yield until the gate is opened or a permit is available.
        loop {
            if self.open.load(Ordering::Acquire) {
                break;
            }
            let mut permits = self.permits.load(Ordering::Acquire);
            let mut admitted = false;
            while permits > 0 {
                match self.permits.compare_exchange(
                    permits,
                    permits - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        admitted = true;
                        break;
                    }
                    Err(current) => permits = current,
                }
            }
            if admitted {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Check if an error is configured.
        if let Some(error_fn) = self.error_fn.lock().unwrap().as_ref() {
            Err(error_fn())
        } else {
            Ok(())
        }
    }
}

/// A gate-controlled ObjectStore wrapper for deterministic concurrency testing.
///
/// Each ObjectStore method has its own [`Gate`] that can be independently closed,
/// released, and configured to inject errors. Gates default to **open** (pass-through).
///
/// # Usage pattern (mirrors `single_flight.rs` tests)
/// ```ignore
/// let inner = Arc::new(InMemory::new());
/// let gated = Arc::new(GatedObjectStore::new(inner));
///
/// // Close the gate for the method under test.
/// gated.get_opts_gate.close();
///
/// // Spawn concurrent callers (they block at the gate inside get_opts)
/// // ...
///
/// // Wait until the expected number of callers have arrived.
/// gated.get_opts_gate.wait_for_arrivals(1).await;
///
/// // Release them — success path.
/// gated.get_opts_gate.release();
///
/// // Or: inject failure before releasing.
/// gated.get_opts_gate.set_should_fail(true);
/// gated.get_opts_gate.release();
/// ```
#[derive(Debug)]
pub(crate) struct GatedObjectStore {
    inner: Arc<dyn ObjectStore>,
    pub(crate) get_opts_gate: Gate,
    pub(crate) head_gate: Gate,
    pub(crate) put_opts_gate: Gate,
    pub(crate) put_multipart_opts_gate: Gate,
    pub(crate) copy_gate: Gate,
    pub(crate) rename_gate: Gate,
    /// Gates each path emitted by `delete_stream`. Per-item gating preserves
    /// the pre-0.13 single-call `delete` semantics now that callers go through
    /// `ObjectStoreExt::delete`, which fans out into `delete_stream`.
    pub(crate) delete_stream_gate: Arc<Gate>,
}

impl GatedObjectStore {
    pub(crate) fn new(inner: Arc<dyn ObjectStore>) -> Self {
        Self {
            inner,
            get_opts_gate: Gate::default(),
            head_gate: Gate::default(),
            put_opts_gate: Gate::default(),
            put_multipart_opts_gate: Gate::default(),
            copy_gate: Gate::default(),
            rename_gate: Gate::default(),
            delete_stream_gate: Arc::new(Gate::default()),
        }
    }
}

impl fmt::Display for GatedObjectStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GatedObjectStore({})", self.inner)
    }
}

#[async_trait]
impl ObjectStore for GatedObjectStore {
    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        if options.head {
            self.head_gate.wait().await?;
        } else {
            self.get_opts_gate.wait().await?;
        }
        self.inner.get_opts(location, options).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: OS_PutOptions,
    ) -> object_store::Result<PutResult> {
        self.put_opts_gate.wait().await?;
        self.inner.put_opts(location, payload, opts).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        self.put_multipart_opts_gate.wait().await?;
        self.inner.put_multipart_opts(location, opts).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        let gate = Arc::clone(&self.delete_stream_gate);
        let gated = locations
            .then(move |loc| {
                let gate = Arc::clone(&gate);
                async move {
                    let loc = loc?;
                    gate.wait().await?;
                    Ok(loc)
                }
            })
            .boxed();
        self.inner.delete_stream(gated)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.copy_gate.wait().await?;
        self.inner.copy_opts(from, to, options).await
    }

    async fn rename_opts(
        &self,
        from: &Path,
        to: &Path,
        options: RenameOptions,
    ) -> object_store::Result<()> {
        self.rename_gate.wait().await?;
        self.inner.rename_opts(from, to, options).await
    }
}

pub(crate) struct StringConcatMergeOperator;

impl MergeOperator for StringConcatMergeOperator {
    fn merge(
        &self,
        _key: &Bytes,
        existing_value: Option<Bytes>,
        value: Bytes,
    ) -> Result<Bytes, MergeOperatorError> {
        let mut result = BytesMut::new();
        existing_value.inspect(|v| result.extend_from_slice(v.as_ref()));
        result.extend_from_slice(value.as_ref());
        Ok(result.freeze())
    }
}

/// Extract library name from a file path
/// e.g., "/Users/.../slatedb/src/db.rs" -> "slatedb"
/// e.g., "/.cargo/registry/.../parking_lot-0.12.1/src/mutex.rs" -> "parking_lot"
/// e.g., "/rustc/.../library/std/src/sync/mutex.rs" -> "std"
fn extract_library(path: &str) -> String {
    // Check for .cargo/registry paths (external crates)
    if let Some(idx) = path.find(".cargo/registry/") {
        let after_registry = &path[idx..];
        // Format: .cargo/registry/src/<hash>/<crate-name>-<version>/...
        // Split: [".cargo", "registry", "src", "<hash>", "<crate-name-version>", ...]
        if let Some(crate_start) = after_registry.split('/').nth(4) {
            // Remove version suffix (e.g., "parking_lot-0.12.1" -> "parking_lot")
            if let Some(dash_idx) = crate_start.rfind('-') {
                // Check if what follows the dash looks like a version
                let after_dash = &crate_start[dash_idx + 1..];
                if after_dash
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit())
                {
                    return crate_start[..dash_idx].to_string();
                }
            }
            return crate_start.to_string();
        }
    }

    // Check for rustc paths (std library)
    if path.contains("/rustc/") {
        if let Some(idx) = path.find("/library/") {
            let after_library = &path[idx + 9..];
            if let Some(lib_name) = after_library.split('/').next() {
                return lib_name.to_string();
            }
        }
    }

    // Check for local workspace crates (look for /src/ and take the directory before it)
    if let Some(src_idx) = path.rfind("/src/") {
        let before_src = &path[..src_idx];
        if let Some(last_slash) = before_src.rfind('/') {
            return before_src[last_slash + 1..].to_string();
        }
    }

    "-".to_string()
}

/// Format a backtrace as a table, showing all frames
pub(crate) fn format_backtrace(bt: &backtrace::Backtrace) -> String {
    // Collect frame info: (library, filename, line, function_name)
    let mut frames: Vec<(String, String, String, String)> = Vec::new();

    for frame in bt.frames() {
        for symbol in frame.symbols() {
            let name = symbol
                .name()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            let (library, filename, line) =
                if let (Some(f), Some(l)) = (symbol.filename(), symbol.lineno()) {
                    let path_str = f.to_string_lossy();
                    let lib = extract_library(&path_str);
                    let file = f
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "-".to_string());
                    (lib, file, l.to_string())
                } else {
                    ("-".to_string(), "-".to_string(), "-".to_string())
                };

            frames.push((library, filename, line, name));
        }
    }

    if frames.is_empty() {
        return "  (no relevant frames found - try RUST_BACKTRACE=1)\n".to_string();
    }

    // Calculate column widths
    let lib_width = frames
        .iter()
        .map(|(l, _, _, _)| l.len())
        .max()
        .unwrap_or(0)
        .max(7); // min width for "Library"
    let file_width = frames
        .iter()
        .map(|(_, f, _, _)| f.len())
        .max()
        .unwrap_or(0)
        .max(4); // min width for "File"
    let line_width = frames
        .iter()
        .map(|(_, _, l, _)| l.len())
        .max()
        .unwrap_or(0)
        .max(4); // min width for "Line"
    let func_width = frames.iter().map(|(_, _, _, f)| f.len()).max().unwrap_or(0);

    let mut output = String::new();

    // Header: Library | File | Line | Function
    output.push_str(&format!(
        "  {:<lib_width$} | {:<file_width$} | {:>line_width$} | {:<func_width$}\n",
        "Library", "File", "Line", "Function"
    ));
    output.push_str(&format!(
        "  {:-<lib_width$}-+-{:-<file_width$}-+-{:->line_width$}-+-{:-<func_width$}\n",
        "", "", "", ""
    ));

    // Rows
    for (library, filename, line, func) in &frames {
        output.push_str(&format!(
            "  {:<lib_width$} | {:<file_width$} | {:>line_width$} | {:<func_width$}\n",
            library, filename, line, func
        ));
    }

    output
}

/// Test extractor that always extracts a fixed 3-byte prefix. Returns
/// `None` for inputs shorter than 3 bytes; callers that exercise the
/// extractor's segmentation path should use keys ≥ 3 bytes.
#[derive(Debug)]
pub(crate) struct FixedThreeBytePrefixExtractor;

impl crate::prefix_extractor::PrefixExtractor for FixedThreeBytePrefixExtractor {
    fn name(&self) -> &str {
        "fixed-3"
    }
    fn prefix_len(&self, target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
        let len = match target {
            crate::prefix_extractor::PrefixTarget::Point(b)
            | crate::prefix_extractor::PrefixTarget::Prefix(b) => b.len(),
        };
        if len >= 3 {
            Some(3)
        } else {
            None
        }
    }
}

/// Test extractor that segments on the leading `data` / `idx` path
/// component, modelling a store that keeps bulky records in one segment and
/// a smaller index over them in another. Tenants live in the *next*
/// component (`data/{tenant}/…`), so a tenant's rows are split across both
/// segments rather than forming one contiguous key range.
// Only the union tests in `clone.rs` use this, and those need `wal_disable`.
#[cfg(feature = "wal_disable")]
#[derive(Debug)]
pub(crate) struct DataIdxPrefixExtractor;

#[cfg(feature = "wal_disable")]
impl crate::prefix_extractor::PrefixExtractor for DataIdxPrefixExtractor {
    fn name(&self) -> &str {
        "data-idx"
    }
    fn prefix_len(&self, target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
        let key: &[u8] = match target {
            crate::prefix_extractor::PrefixTarget::Point(b)
            | crate::prefix_extractor::PrefixTarget::Prefix(b) => b.as_ref(),
        };
        for kind in [b"data".as_slice(), b"idx".as_slice()] {
            if key == kind || key.starts_with(&[kind, b"/".as_slice()].concat()) {
                return Some(kind.len());
            }
        }
        None
    }
}

/// Test extractor that deliberately violates the
/// [`crate::prefix_extractor::PrefixExtractor`] `Point` invariant by
/// returning different prefix lengths for keys that share a common
/// prefix — `Some(3)` for keys starting with `"abc"` and `Some(2)`
/// for keys starting with `"ab"` but not `"abc"`. A batch with both
/// yields nestable prefixes `"ab"` and `"abc"`, exercising the
/// antichain checks at write time and replay.
#[derive(Debug)]
pub(crate) struct NonAntichainTestPrefixExtractor;

impl crate::prefix_extractor::PrefixExtractor for NonAntichainTestPrefixExtractor {
    fn name(&self) -> &str {
        "non-antichain-test-only"
    }
    fn prefix_len(&self, target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
        let bytes: &[u8] = match target {
            crate::prefix_extractor::PrefixTarget::Point(b)
            | crate::prefix_extractor::PrefixTarget::Prefix(b) => b.as_ref(),
        };
        if bytes.starts_with(b"abc") {
            Some(3)
        } else if bytes.starts_with(b"ab") {
            Some(2)
        } else {
            None
        }
    }
}

/// Shares `name()` with [`FixedThreeBytePrefixExtractor`] but
/// returns divergent prefix lengths: `Some(3)` for keys starting
/// with `"abc"`, `Some(2)` for keys starting with `"ab"` but not
/// `"abc"`, and `None` otherwise. Lets a test reopen a DB whose
/// open-time name check passes while WAL replay encounters the
/// swapped logic.
#[derive(Debug)]
pub(crate) struct AliasedFixed3PrefixExtractor;

impl crate::prefix_extractor::PrefixExtractor for AliasedFixed3PrefixExtractor {
    fn name(&self) -> &str {
        // Match FixedThreeBytePrefixExtractor's name so the open-time
        // reconciliation check succeeds.
        "fixed-3"
    }
    fn prefix_len(&self, target: &crate::prefix_extractor::PrefixTarget) -> Option<usize> {
        let bytes: &[u8] = match target {
            crate::prefix_extractor::PrefixTarget::Point(b)
            | crate::prefix_extractor::PrefixTarget::Prefix(b) => b.as_ref(),
        };
        if bytes.starts_with(b"abc") {
            Some(3)
        } else if bytes.starts_with(b"ab") {
            Some(2)
        } else {
            None
        }
    }
}

static INIT_LOGGING: Once = Once::new();
static INIT_DEADLOCK_DETECTOR: Once = Once::new();

/// Initialize tracing/logging for tests. Uses `RUST_LOG` environment
/// variable to set the log level, or defaults to `debug` if not set.
pub(crate) fn init_logging() {
    INIT_LOGGING.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_test_writer()
            .init();
    });
}

/// Start the deadlock detector thread that periodically checks for parking_lot deadlocks.
pub(crate) fn init_deadlock_detector() {
    INIT_DEADLOCK_DETECTOR.call_once(|| {
        thread::spawn(|| loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = parking_lot::deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }

            let separator = "=".repeat(60);
            eprintln!("\n{separator}");
            eprintln!("DEADLOCK DETECTED: {} deadlock(s) found", deadlocks.len());
            eprintln!("{separator}\n");

            for (i, threads) in deadlocks.iter().enumerate() {
                eprintln!("--- Deadlock #{} ({} threads) ---\n", i + 1, threads.len());
                for (j, t) in threads.iter().enumerate() {
                    eprintln!("Thread {} (id: {:?}):", j + 1, t.thread_id());
                    eprintln!("{}", format_backtrace(t.backtrace()));
                }
            }

            eprintln!("{separator}\n");
        });
    });
}

/// Initialize all test infrastructure (logging and deadlock detection).
pub(crate) fn init_test_infrastructure() {
    init_logging();
    init_deadlock_detector();
}

#[cfg(test)]
mod tests {
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    /// Test to verify the deadlock detector is working.
    /// This test intentionally creates a deadlock and is ignored by default.
    /// Run with: cargo test --package slatedb test_deadlock_detector -- --ignored --nocapture
    /// The deadlock detector should print deadlock info after ~10 seconds.
    #[test]
    #[ignore]
    fn test_deadlock_detector_should_detect_deadlock() {
        let lock_a = Arc::new(Mutex::new(()));
        let lock_b = Arc::new(Mutex::new(()));

        let lock_a_clone = lock_a.clone();
        let lock_b_clone = lock_b.clone();

        // Thread 1: acquire lock_a, then try to acquire lock_b
        let thread1 = thread::spawn(move || {
            let _guard_a = lock_a_clone.lock();
            thread::sleep(Duration::from_millis(100)); // Give thread2 time to acquire lock_b
            let _guard_b = lock_b_clone.lock(); // This will deadlock
        });

        // Thread 2: acquire lock_b, then try to acquire lock_a
        let thread2 = thread::spawn(move || {
            let _guard_b = lock_b.lock();
            thread::sleep(Duration::from_millis(100)); // Give thread1 time to acquire lock_a
            let _guard_a = lock_a.lock(); // This will deadlock
        });

        // Wait for the deadlock detector to report (it checks every 10 seconds)
        // This test will hang indefinitely due to the deadlock - that's expected.
        // The deadlock detector should print the deadlock info to the logs.
        thread1.join().unwrap();
        thread2.join().unwrap();
    }
}

/// One object store call observed by [`RecordingObjectStore`], with the
/// `TableStoreKind` / `SstType` / `RetryReason` tags it carried.
#[derive(Clone, Debug)]
pub(crate) enum RecordedCall {
    Get {
        head: bool,
        range: Option<GetRange>,
        kind: Option<TableStoreKind>,
        sst_type: Option<SstType>,
        retry: Option<RetryReason>,
        segment: Option<Bytes>,
    },
    Put {
        kind: Option<TableStoreKind>,
        sst_type: Option<SstType>,
        segment: Option<Bytes>,
    },
    PutMultipart {
        kind: Option<TableStoreKind>,
        sst_type: Option<SstType>,
        segment: Option<Bytes>,
    },
}

/// Wraps an object store and records the tags carried by each
/// get/put/multipart-init call, delegating all I/O to the inner store.
#[derive(Debug)]
pub(crate) struct RecordingObjectStore {
    inner: Arc<dyn ObjectStore>,
    calls: parking_lot::Mutex<Vec<RecordedCall>>,
}

impl RecordingObjectStore {
    pub(crate) fn new(inner: Arc<dyn ObjectStore>) -> Self {
        Self {
            inner,
            calls: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn clear(&self) {
        self.calls.lock().clear();
    }

    pub(crate) fn get_kinds(&self, head: bool) -> Vec<Option<TableStoreKind>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Get { head: h, kind, .. } if *h == head => Some(*kind),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn get_sst_types(&self, head: bool) -> Vec<Option<SstType>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Get {
                    head: h, sst_type, ..
                } if *h == head => Some(*sst_type),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn get_retries(&self, head: bool) -> Vec<Option<RetryReason>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Get { head: h, retry, .. } if *h == head => Some(*retry),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn get_segments(&self, head: bool) -> Vec<Option<Bytes>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Get {
                    head: h, segment, ..
                } if *h == head => Some(segment.clone()),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn recorded_get_ranges(&self, head: bool) -> Vec<Option<GetRange>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|call| match call {
                RecordedCall::Get {
                    head: is_head,
                    range,
                    ..
                } if *is_head == head => Some(range.clone()),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn write_kinds(&self) -> Vec<Option<TableStoreKind>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Put { kind, .. } | RecordedCall::PutMultipart { kind, .. } => {
                    Some(*kind)
                }
                _ => None,
            })
            .collect()
    }

    pub(crate) fn write_sst_types(&self) -> Vec<Option<SstType>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Put { sst_type, .. }
                | RecordedCall::PutMultipart { sst_type, .. } => Some(*sst_type),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn write_segments(&self) -> Vec<Option<Bytes>> {
        self.calls
            .lock()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::Put { segment, .. } | RecordedCall::PutMultipart { segment, .. } => {
                    Some(segment.clone())
                }
                _ => None,
            })
            .collect()
    }
}

impl fmt::Display for RecordingObjectStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RecordingObjectStore({})", self.inner)
    }
}

#[async_trait]
impl ObjectStore for RecordingObjectStore {
    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        let tag = ObjectStoreCallTag::from_extensions(&options.extensions);
        self.calls.lock().push(RecordedCall::Get {
            head: options.head,
            range: options.range.clone(),
            kind: tag.as_ref().map(|t| t.kind),
            sst_type: tag.as_ref().map(|t| t.sst_type),
            retry: tag.as_ref().and_then(|t| t.retry),
            segment: tag.as_ref().and_then(|t| t.segment.clone()),
        });
        self.inner.get_opts(location, options).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: OS_PutOptions,
    ) -> object_store::Result<PutResult> {
        let tag = ObjectStoreCallTag::from_extensions(&opts.extensions);
        self.calls.lock().push(RecordedCall::Put {
            kind: tag.as_ref().map(|t| t.kind),
            sst_type: tag.as_ref().map(|t| t.sst_type),
            segment: tag.as_ref().and_then(|t| t.segment.clone()),
        });
        self.inner.put_opts(location, payload, opts).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        let tag = ObjectStoreCallTag::from_extensions(&opts.extensions);
        self.calls.lock().push(RecordedCall::PutMultipart {
            kind: tag.as_ref().map(|t| t.kind),
            sst_type: tag.as_ref().map(|t| t.sst_type),
            segment: tag.as_ref().and_then(|t| t.segment.clone()),
        });
        self.inner.put_multipart_opts(location, opts).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        self.inner.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.inner.copy_opts(from, to, options).await
    }
}
