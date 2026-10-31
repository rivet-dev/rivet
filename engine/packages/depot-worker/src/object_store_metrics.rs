//! Wraps an upstream `object_store::ObjectStore` to record request latency and payload
//! size. It is installed by the production object storage adapter, so the telemetry
//! clock reads here are the only ones in the request path.

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{StreamExt, stream::BoxStream};
use object_store::{
	CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
	PutMode, PutMultipartOptions, PutOptions, PutPayload, PutResult, RenameOptions, Result,
	path::Path,
};
use std::{
	cell::{Cell, RefCell},
	fmt,
	ops::Range,
	sync::Arc,
	time::Instant,
};

use crate::object_store_first_attempt::{
	self as first_attempt, FirstAttempt, FirstAttemptDeadlines, ShortFirstAttempts,
};

/// An object store that records `rivet_depot_object_store_request_duration_seconds` and
/// `rivet_depot_object_store_request_bytes` for puts, gets, heads, lists, deletes, copies
/// and renames, the retries of each, and multipart uploads by part and outcome. Each of
/// these calls can issue several billed HTTP requests (list pages, multipart parts, a
/// rename's copy and delete); `InstrumentedHttpConnector` counts those.
#[derive(Debug)]
pub struct InstrumentedObjectStore {
	inner: Arc<dyn ObjectStore>,
}

impl InstrumentedObjectStore {
	pub fn wrap(inner: Arc<dyn ObjectStore>) -> Arc<dyn ObjectStore> {
		Arc::new(Self { inner })
	}
}

impl fmt::Display for InstrumentedObjectStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Instrumented({})", self.inner)
	}
}

/// Why the tablet store issued an object-store request, for telemetry only. The tablet
/// maintainer's store executor sets it around each store operation; requests from
/// SlateDB's own tasks (memtable uploads, compaction, garbage collection) and from any
/// other caller report `Background`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectStorePurpose {
	Apply,
	Stage,
	Retention,
	Lookup,
	Query,
	Load,
	Open,
	Maintenance,
	Background,
}

impl ObjectStorePurpose {
	pub fn as_str(self) -> &'static str {
		match self {
			ObjectStorePurpose::Apply => "apply",
			ObjectStorePurpose::Stage => "stage",
			ObjectStorePurpose::Retention => "retention",
			ObjectStorePurpose::Lookup => "lookup",
			ObjectStorePurpose::Query => "query",
			ObjectStorePurpose::Load => "load",
			ObjectStorePurpose::Open => "open",
			ObjectStorePurpose::Maintenance => "maintenance",
			ObjectStorePurpose::Background => "background",
		}
	}
}

tokio::task_local! {
	static PURPOSE: ObjectStorePurpose;
	/// HTTP attempts of the request being issued on this task. The upstream retry loop
	/// runs inline in the caller's task, so every attempt of one request lands here.
	static ATTEMPTS: RequestAttempts;
	/// The object source of the request being issued on this task, for its HTTP attempts.
	static SOURCE: ObjectSource;
}

/// The HTTP attempts of one request so far, and the URI of a conditional PUT among them
/// that was cut off at its short first-attempt deadline. The URI keeps a credential request
/// issued in the same call from being mistaken for the PUT.
#[derive(Default)]
struct RequestAttempts {
	count: Cell<u32>,
	cut_off_put: RefCell<Option<http::Uri>>,
}

/// Runs one request's `future`, then counts its attempts past the first as retries of
/// `op`. A request that never reached HTTP, such as an in-memory store, counts none. Its
/// HTTP attempts report `source`.
async fn counting_retries<F: std::future::Future>(
	op: &'static str,
	source: ObjectSource,
	future: F,
) -> F::Output {
	let counted = ATTEMPTS.scope(RequestAttempts::default(), async move {
		let output = future.await;
		let attempts = ATTEMPTS.with(|attempts| attempts.count.get());
		if attempts > 1 {
			crate::metrics::OBJECT_STORE_RETRIES
				.with_label_values(&[op])
				.inc_by(u64::from(attempts - 1));
		}
		output
	});
	SOURCE.scope(source, counted).await
}

/// The billed request class of one HTTP attempt. Stores bill a GET that lists a bucket or
/// container as a LIST: S3 and GCS mark listings with `list-type` (multipart listings with
/// `uploads`) and Azure with `comp=list`. Multipart parts and copies are PUTs, as they are
/// billed. S3 bills its POSTs (multipart create and complete) as PUT-class requests, and a
/// batch delete is one POST however many objects it removes.
pub fn billed_method(method: &http::Method, query: Option<&str>) -> &'static str {
	let has_param = |name: &str, value: Option<&str>| {
		query.is_some_and(|query| {
			query.split('&').any(|pair| {
				let (key, found) = pair.split_once('=').unwrap_or((pair, ""));
				key == name && value.is_none_or(|value| found == value)
			})
		})
	};
	match *method {
		http::Method::HEAD => "head",
		http::Method::GET => {
			let lists = has_param("list-type", None)
				|| has_param("uploads", None)
				|| has_param("comp", Some("list"));
			if lists { "list" } else { "get" }
		}
		http::Method::PUT => "put",
		http::Method::POST => "post",
		http::Method::DELETE => "delete",
		_ => "other",
	}
}

/// Status label of an HTTP attempt dropped before its response headers arrived, such as one
/// cut off by its caller's deadline.
pub const ABANDONED: &str = "abandoned";

/// Status label of a first HTTP attempt cut off at its short response deadline and retried
/// by the client.
pub const CUT_OFF: &str = "cut_off";

/// Bounded label for one HTTP attempt's response status. Statuses Depot handles or that
/// mean throttling keep their own label; the rest are grouped by class.
pub fn status_label(status: Option<u16>) -> &'static str {
	match status {
		None => "transport_error",
		Some(200..=299) => "2xx",
		Some(304) => "304",
		Some(404) => "404",
		Some(409) => "409",
		Some(412) => "412",
		Some(429) => "429",
		Some(500) => "500",
		Some(503) => "503",
		Some(400..=499) => "4xx",
		Some(500..=599) => "5xx",
		Some(_) => "other",
	}
}

/// Counts one HTTP attempt by billed method, source and status, and against the request
/// issued on this task.
fn record_attempt(method: &str, source: ObjectSource, status: Option<u16>, started: Instant) {
	record_attempt_label(method, source, status_label(status), started);
}

fn record_attempt_label(method: &str, source: ObjectSource, label: &str, started: Instant) {
	let _ =
		ATTEMPTS.try_with(|attempts| attempts.count.set(attempts.count.get().saturating_add(1)));
	crate::metrics::OBJECT_STORE_HTTP_ATTEMPTS
		.with_label_values(&[method, source.as_str(), label])
		.inc();
	crate::metrics::OBJECT_STORE_HTTP_ATTEMPT_DURATION
		.with_label_values(&[label])
		.observe(started.elapsed().as_secs_f64());
}

/// An `HttpConnector` whose clients count every HTTP attempt by billed method, source and
/// response status, so billed requests, throttling (429, 503) and retries are visible. Install it with the cloud builders'
/// `with_http_connector`.
#[derive(Debug, Default)]
pub struct InstrumentedHttpConnector {
	inner: object_store::client::ReqwestConnector,
	first_attempt_deadlines: Option<FirstAttemptDeadlines>,
}

impl InstrumentedHttpConnector {
	/// A connector whose clients also give the first attempt of each small request a short
	/// response deadline; see `object_store_first_attempt`. Only requests issued through
	/// `InstrumentedObjectStore` qualify, because their attempts are counted per request.
	pub fn with_short_first_attempts(deadlines: FirstAttemptDeadlines) -> Self {
		Self {
			inner: object_store::client::ReqwestConnector::default(),
			first_attempt_deadlines: Some(deadlines),
		}
	}
}

impl object_store::client::HttpConnector for InstrumentedHttpConnector {
	fn connect(
		&self,
		options: &object_store::ClientOptions,
	) -> Result<object_store::client::HttpClient> {
		let inner = self.inner.connect(options)?;
		let short_first_attempts = self.first_attempt_deadlines.map(ShortFirstAttempts::new);
		Ok(object_store::client::HttpClient::new(
			InstrumentedHttpService {
				inner,
				short_first_attempts,
			},
		))
	}
}

#[derive(Debug)]
struct InstrumentedHttpService {
	inner: object_store::client::HttpClient,
	/// The cut-off budget of this client, when its first attempts get short deadlines.
	short_first_attempts: Option<ShortFirstAttempts>,
}

#[async_trait]
impl object_store::client::HttpService for InstrumentedHttpService {
	async fn call(
		&self,
		request: object_store::client::HttpRequest,
	) -> std::result::Result<object_store::client::HttpResponse, object_store::client::HttpError> {
		let method = billed_method(request.method(), request.uri().query());
		// Requests issued outside a wrapped call, such as list pages and batch deletes polled
		// from a stream, fall back to the path they address.
		let source = SOURCE
			.try_with(|source| *source)
			.unwrap_or_else(|_| ObjectSource::of_uri(request.uri()));
		// Attempts of a request outside a wrapped call have no count, so they never qualify.
		let conditional_put = first_attempt::is_conditional_put(&request);
		let (attempt_index, earlier_cut_off) = ATTEMPTS
			.try_with(|attempts| {
				let cut_off = attempts.cut_off_put.borrow().as_ref() == Some(request.uri());
				(Some(attempts.count.get()), conditional_put && cut_off)
			})
			.unwrap_or((None, false));
		let uri = request.uri().clone();
		let short_deadline = match (&self.short_first_attempts, attempt_index) {
			(Some(budget), Some(0)) => budget
				.deadlines
				.deadline(&request, method)
				.map(|plan| (budget, plan)),
			_ => None,
		};

		let attempt = AttemptTimer::start(method, source);
		let response = match short_deadline {
			Some((budget, (eligible, deadline))) => {
				match budget
					.run(method, deadline, self.inner.execute(request))
					.await
				{
					FirstAttempt::Answered(response) => response,
					FirstAttempt::CutOff => {
						if eligible == first_attempt::Eligible::ConditionalPut {
							let _ = ATTEMPTS.try_with(|attempts| {
								*attempts.cut_off_put.borrow_mut() = Some(uri)
							});
						}
						attempt.finish_label(CUT_OFF);
						return Err(first_attempt::cut_off_error(eligible, deadline));
					}
				}
			}
			None => self.inner.execute(request).await,
		};
		let status = response
			.as_ref()
			.ok()
			.map(|response| response.status().as_u16());
		attempt.finish(status);

		// A precondition failure after a cut-off attempt of the same conditional PUT may be
		// that attempt's own write, so it is reported as unknown.
		match status {
			Some(status) if earlier_cut_off && first_attempt::is_ambiguous_precondition(status) => {
				Err(first_attempt::ambiguous_precondition_error(status))
			}
			_ => response,
		}
	}
}

/// Times one HTTP attempt. Dropped before `finish`, it records the attempt as
/// `abandoned` with its elapsed time, so attempts cut off by a deadline stay visible.
struct AttemptTimer {
	method: &'static str,
	source: ObjectSource,
	started: Instant,
	finished: bool,
}

impl AttemptTimer {
	fn start(method: &'static str, source: ObjectSource) -> Self {
		Self {
			method,
			source,
			started: now(),
			finished: false,
		}
	}

	fn finish(mut self, status: Option<u16>) {
		self.finished = true;
		record_attempt(self.method, self.source, status, self.started);
	}

	fn finish_label(mut self, label: &str) {
		self.finished = true;
		record_attempt_label(self.method, self.source, label, self.started);
	}
}

impl Drop for AttemptTimer {
	fn drop(&mut self) {
		if !self.finished {
			record_attempt_label(self.method, self.source, ABANDONED, self.started);
		}
	}
}

/// A multipart upload that counts its parts, bytes and outcome.
#[derive(Debug)]
struct InstrumentedUpload {
	inner: Box<dyn MultipartUpload>,
	source: ObjectSource,
}

#[async_trait]
impl MultipartUpload for InstrumentedUpload {
	fn put_part(&mut self, data: PutPayload) -> object_store::UploadPart {
		crate::metrics::OBJECT_STORE_MULTIPART_PARTS
			.with_label_values(&[self.source.as_str()])
			.inc();
		crate::metrics::OBJECT_STORE_MULTIPART_BYTES
			.with_label_values(&[self.source.as_str()])
			.inc_by(data.content_length() as u64);
		// The upload may poll its parts on another task, so each part carries its source.
		Box::pin(SOURCE.scope(self.source, self.inner.put_part(data)))
	}

	async fn complete(&mut self) -> Result<PutResult> {
		let result =
			counting_retries("multipart_complete", self.source, self.inner.complete()).await;
		crate::metrics::OBJECT_STORE_MULTIPART_UPLOADS
			.with_label_values(&[self.source.as_str(), "complete", outcome(&result)])
			.inc();
		result
	}

	async fn abort(&mut self) -> Result<()> {
		let result = SOURCE.scope(self.source, self.inner.abort()).await;
		crate::metrics::OBJECT_STORE_MULTIPART_UPLOADS
			.with_label_values(&[self.source.as_str(), "abort", outcome(&result)])
			.inc();
		result
	}
}

/// Runs `future` with every object-store request it makes on this task labelled
/// `purpose`. Requests made by tasks it spawns keep their own label.
pub async fn with_purpose<F: std::future::Future>(
	purpose: ObjectStorePurpose,
	future: F,
) -> F::Output {
	PURPOSE.scope(purpose, future).await
}

/// The purpose of the request being issued on this task.
pub fn current_purpose() -> ObjectStorePurpose {
	PURPOSE
		.try_with(|purpose| *purpose)
		.unwrap_or(ObjectStorePurpose::Background)
}

/// Bounded outcome label for one request.
pub fn outcome<T>(result: &Result<T>) -> &'static str {
	match result {
		Ok(_) => "ok",
		Err(object_store::Error::NotFound { .. }) => "not_found",
		Err(object_store::Error::Precondition { .. }) => "precondition",
		Err(object_store::Error::AlreadyExists { .. }) => "already_exists",
		Err(object_store::Error::NotModified { .. }) => "not_modified",
		Err(_) => "error",
	}
}

/// What a request touches, for telemetry only: SlateDB's compactor and garbage collector
/// by their call tag, objects in a tablet database by their directory in its layout (the
/// `gc` directory's boundary files as `Gc`), and cluster objects by their key kind. A foreground `sst` GET is a block cache miss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectSource {
	Compaction,
	Gc,
	Manifest,
	Compactions,
	Sst,
	Wal,
	/// A worker heartbeat under `depot/cluster/node/{role}`, read by topology discovery,
	/// endpoint lookups, lease checks and heartbeat inspections.
	Heartbeat,
	/// The singleton coordinator lease at `cluster/coordinator`.
	Lease,
	/// The cluster config at `cluster/config`.
	Config,
	/// The tablet coordinator's routing manifest at `depot/manifest`.
	TabletManifest,
	/// Any other object under a tablet database's directory.
	TabletOther,
	Other,
}

impl ObjectSource {
	pub fn as_str(self) -> &'static str {
		match self {
			ObjectSource::Compaction => "compaction",
			ObjectSource::Gc => "gc",
			ObjectSource::Manifest => "manifest",
			ObjectSource::Compactions => "compactions",
			ObjectSource::Sst => "sst",
			ObjectSource::Wal => "wal",
			ObjectSource::Heartbeat => "heartbeat",
			ObjectSource::Lease => "lease",
			ObjectSource::Config => "config",
			ObjectSource::TabletManifest => "tablet_manifest",
			ObjectSource::TabletOther => "tablet_other",
			ObjectSource::Other => "other",
		}
	}

	/// Classifies one request from its SlateDB call tag, when it carries one, and its path.
	pub fn of(location: &Path, extensions: &object_store::Extensions) -> Self {
		use slatedb::object_store_tag::{ObjectStoreCallTag, TableStoreKind};

		if let Some(tag) = extensions.get::<ObjectStoreCallTag>() {
			match tag.kind {
				TableStoreKind::Compactor => return ObjectSource::Compaction,
				TableStoreKind::GC => return ObjectSource::Gc,
				TableStoreKind::Main | TableStoreKind::Reader => {}
			}
		}
		Self::of_path(location)
	}

	fn of_path(location: &Path) -> Self {
		let parts = location
			.parts()
			.map(|part| part.as_ref().to_owned())
			.collect::<Vec<_>>();
		let parts = parts.iter().map(String::as_str).collect::<Vec<_>>();

		// Cluster objects, by the fixed segments of their keys. The routing manifest ends in
		// `depot/manifest`, while SlateDB's manifests are files inside a `manifest` directory.
		if parts.ends_with(&["depot", "manifest"]) {
			return ObjectSource::TabletManifest;
		}
		if parts.ends_with(&["cluster", "coordinator"]) {
			return ObjectSource::Lease;
		}
		if parts.ends_with(&["cluster", "config"]) {
			return ObjectSource::Config;
		}
		if parts.windows(4).any(|w| {
			w[..3] == ["depot", "cluster", "node"] && matches!(w[3], "storage" | "stateless")
		}) {
			return ObjectSource::Heartbeat;
		}

		for part in &parts {
			match *part {
				"gc" => return ObjectSource::Gc,
				"manifest" => return ObjectSource::Manifest,
				"compactions" => return ObjectSource::Compactions,
				"compacted" => return ObjectSource::Sst,
				"wal" => return ObjectSource::Wal,
				_ => {}
			}
		}
		if parts.contains(&"tablets") {
			return ObjectSource::TabletOther;
		}
		ObjectSource::Other
	}

	/// Classifies one HTTP request by the object path it addresses, followed by its `prefix`
	/// query parameter for listings, which address the bucket or container root.
	fn of_uri(uri: &http::Uri) -> Self {
		let prefix = uri.query().and_then(|query| {
			query
				.split('&')
				.find_map(|pair| pair.strip_prefix("prefix="))
		});
		let joined = match prefix {
			Some(prefix) => format!("{}/{prefix}", uri.path()),
			None => uri.path().to_owned(),
		};
		match Path::from_url_path(joined) {
			Ok(path) => Self::of_path(&path),
			Err(_) => ObjectSource::Other,
		}
	}
}

/// A telemetry-only clock reading.
fn now() -> Instant {
	#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
	Instant::now()
}

/// Times one request. `finish` records it with its outcome; dropped before that, because
/// its caller gave up on it (a deadline, a lost `select`), it records `abandoned` with the
/// elapsed time. Without this a request cut off by a timeout leaves no sample at all, which
/// hides exactly the tail a timeout is there for.
struct RequestTimer {
	op: &'static str,
	purpose: ObjectStorePurpose,
	source: ObjectSource,
	started: Instant,
	finished: bool,
}

impl RequestTimer {
	/// Starts timing a request issued on this task, under the task's current purpose.
	fn start(op: &'static str, source: ObjectSource) -> Self {
		Self {
			op,
			purpose: current_purpose(),
			source,
			started: now(),
			finished: false,
		}
	}

	fn finish(mut self, result: &str, bytes: Option<u64>) {
		self.finished = true;
		observe(
			self.op,
			self.purpose,
			self.source,
			result,
			self.started,
			bytes,
		);
	}
}

impl Drop for RequestTimer {
	fn drop(&mut self) {
		if !self.finished {
			observe(
				self.op,
				self.purpose,
				self.source,
				ABANDONED,
				self.started,
				None,
			);
		}
	}
}

fn observe(
	op: &str,
	purpose: ObjectStorePurpose,
	source: ObjectSource,
	result: &str,
	started: Instant,
	bytes: Option<u64>,
) {
	let elapsed = started.elapsed();
	crate::metrics::OBJECT_STORE_REQUEST_DURATION
		.with_label_values(&[op, purpose.as_str(), source.as_str(), result])
		.observe(elapsed.as_secs_f64());
	if op == "get" {
		crate::metrics::record_tablet_cold_get(elapsed);
	}
	if let Some(bytes) = bytes {
		crate::metrics::OBJECT_STORE_REQUEST_BYTES
			.with_label_values(&[op, source.as_str()])
			.observe(bytes as f64);
	}
}

/// Records the whole stream's duration once it ends, or as `abandoned` when the caller
/// drops it first.
fn timed_stream<T: Send + 'static>(
	op: &'static str,
	source: ObjectSource,
	stream: BoxStream<'static, T>,
) -> BoxStream<'static, T> {
	// The timer lives in the final stream element, so dropping the stream early drops it
	// unfinished.
	let timer = RequestTimer::start(op, source);
	stream
		.map(Some)
		.chain(futures_util::stream::once(async move {
			timer.finish("ok", None);
			None
		}))
		.filter_map(futures_util::future::ready)
		.boxed()
}

#[async_trait]
impl ObjectStore for InstrumentedObjectStore {
	async fn put_opts(
		&self,
		location: &Path,
		payload: PutPayload,
		opts: PutOptions,
	) -> Result<PutResult> {
		let op = match opts.mode {
			PutMode::Overwrite => "put",
			PutMode::Create | PutMode::Update(_) => "put_conditional",
		};
		let bytes = payload.content_length() as u64;
		let source = ObjectSource::of(location, &opts.extensions);
		let timer = RequestTimer::start(op, source);
		let result =
			counting_retries(op, source, self.inner.put_opts(location, payload, opts)).await;
		timer.finish(outcome(&result), Some(bytes));
		result
	}

	async fn put_multipart_opts(
		&self,
		location: &Path,
		opts: PutMultipartOptions,
	) -> Result<Box<dyn MultipartUpload>> {
		let source = ObjectSource::of(location, &opts.extensions);
		let result = counting_retries(
			"multipart_start",
			source,
			self.inner.put_multipart_opts(location, opts),
		)
		.await;
		crate::metrics::OBJECT_STORE_MULTIPART_UPLOADS
			.with_label_values(&[source.as_str(), "start", outcome(&result)])
			.inc();
		let upload: Box<dyn MultipartUpload> = Box::new(InstrumentedUpload {
			inner: result?,
			source,
		});
		Ok(upload)
	}

	async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
		let source = ObjectSource::of(location, &options.extensions);
		let op = if options.head { "head" } else { "get" };
		let timer = RequestTimer::start(op, source);
		let result = counting_retries(op, source, self.inner.get_opts(location, options)).await;
		// A head returns metadata only, so its range describes no payload.
		let bytes = result
			.as_ref()
			.ok()
			.filter(|_| op == "get")
			.map(|result| result.range.end.saturating_sub(result.range.start));
		timer.finish(outcome(&result), bytes);
		result
	}

	async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
		let source = ObjectSource::of_path(location);
		let timer = RequestTimer::start("get", source);
		let result = counting_retries("get", source, self.inner.get_ranges(location, ranges)).await;
		let bytes = result
			.as_ref()
			.ok()
			.map(|parts| parts.iter().map(|part| part.len() as u64).sum());
		timer.finish(outcome(&result), bytes);
		result
	}

	async fn delete_if_match(&self, location: &Path, etag: &str) -> Result<()> {
		let source = ObjectSource::of_path(location);
		let timer = RequestTimer::start("delete", source);
		let result =
			counting_retries("delete", source, self.inner.delete_if_match(location, etag)).await;
		timer.finish(outcome(&result), None);
		result
	}

	fn delete_stream(
		&self,
		locations: BoxStream<'static, Result<Path>>,
	) -> BoxStream<'static, Result<Path>> {
		timed_stream(
			"delete",
			ObjectSource::Other,
			self.inner.delete_stream(locations),
		)
	}

	fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
		timed_stream("list", list_source(prefix), self.inner.list(prefix))
	}

	fn list_with_offset(
		&self,
		prefix: Option<&Path>,
		offset: &Path,
	) -> BoxStream<'static, Result<ObjectMeta>> {
		timed_stream(
			"list",
			list_source(prefix),
			self.inner.list_with_offset(prefix, offset),
		)
	}

	async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
		let source = list_source(prefix);
		let timer = RequestTimer::start("list", source);
		let result = counting_retries("list", source, self.inner.list_with_delimiter(prefix)).await;
		timer.finish(outcome(&result), None);
		result
	}

	async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
		let source = ObjectSource::of_path(to);
		let timer = RequestTimer::start("copy", source);
		let result =
			counting_retries("copy", source, self.inner.copy_opts(from, to, options)).await;
		timer.finish(outcome(&result), None);
		result
	}

	async fn rename_opts(&self, from: &Path, to: &Path, options: RenameOptions) -> Result<()> {
		let source = ObjectSource::of_path(to);
		let timer = RequestTimer::start("rename", source);
		let result =
			counting_retries("rename", source, self.inner.rename_opts(from, to, options)).await;
		timer.finish(outcome(&result), None);
		result
	}
}

/// The source of a listing, by the prefix it lists.
fn list_source(prefix: Option<&Path>) -> ObjectSource {
	prefix.map_or(ObjectSource::Other, ObjectSource::of_path)
}


