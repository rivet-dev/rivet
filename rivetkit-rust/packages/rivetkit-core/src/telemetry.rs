//! Internal OpenTelemetry spans owned by the actor runtime.

#[cfg(feature = "native-runtime")]
pub mod export;

use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use opentelemetry::trace::{
	SpanContext, SpanId, TraceContextExt as _, TraceFlags, TraceId, TraceState,
};
use parking_lot::Mutex;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

use crate::ActorContext;

/// Correlation fields accepted at an invocation boundary.
#[derive(Debug, Default)]
pub struct IncomingInvocationContext {
	pub(crate) ray_id: Option<String>,
	remote_parent: Option<SpanContext>,
}

/// Header carrying the caller's ray into an actor.
pub(crate) const HEADER_RIVETKIT_RAY_ID: &str = "x-rivetkit-ray-id";

impl IncomingInvocationContext {
	pub(crate) fn from_headers(
		ray_id: Option<String>,
		traceparent: Option<&str>,
		tracestate: Option<&str>,
	) -> Self {
		Self {
			ray_id,
			remote_parent: parse_remote_parent(traceparent, tracestate),
		}
	}

	/// Reads the ray ID and W3C trace context an HTTP request carries. Every
	/// HTTP entry point into an actor reads them through here, so they all
	/// apply the same bounds.
	pub(crate) fn from_http_headers(headers: &http::HeaderMap) -> Self {
		Self::from_headers(
			invocation_ray_id(headers),
			headers.get("traceparent").and_then(|value| value.to_str().ok()),
			headers.get("tracestate").and_then(|value| value.to_str().ok()),
		)
	}
}

/// Reads the caller's ray id. The header is untrusted, so it is bounded to
/// 128 characters of `[A-Za-z0-9_-]`; anything else counts as absent and the
/// invocation mints a fresh ray instead.
fn invocation_ray_id(headers: &http::HeaderMap) -> Option<String> {
	headers
		.get(HEADER_RIVETKIT_RAY_ID)?
		.to_str()
		.ok()
		.filter(|value| {
			!value.is_empty()
				&& value.len() <= 128
				&& value
					.bytes()
					.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
		})
		.map(str::to_owned)
}

/// The single root span for one client action invocation.
#[derive(Debug)]
pub(crate) struct ActionInvocationSpan {
	telemetry: ActorInvocationTelemetry,
}

/// Opaque invocation context carried across foreign-runtime adapters.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ActorInvocationTelemetry(Arc<InvocationInner>);

/// Identity fields that do not change while an actor is alive. Built once per
/// actor and shared by every invocation, so starting one does not re-allocate
/// them.
#[derive(Debug)]
pub(crate) struct ActorTelemetryIdentity {
	pub(crate) actor_id: String,
	pub(crate) actor_name: String,
	pub(crate) actor_key: String,
}

/// Shared invocation state. `finished` lets exactly one of the finish and
/// drop paths record the terminal status, and marks the invocation closed even
/// when tracing is off and there is no span. The span slot is emptied, which
/// is what exports the span, once the status is recorded and no work handed
/// to `wait_until` from this invocation is still running. `pending_work`
/// counts that work, so the invocation stays active for it after the reply.
#[derive(Debug)]
struct InvocationInner {
	ray_id: Option<String>,
	// Forced-sync: the slot is emptied from `Drop` paths and sync accessors,
	// and the guard is never held across an await.
	span: Mutex<Option<tracing::Span>>,
	finished: AtomicBool,
	pending_work: AtomicUsize,
	identity: Arc<ActorTelemetryIdentity>,
}

const OPERATION_ABANDONED_ERROR_TYPE: &str = "actor.operation_abandoned";

/// Keeps an invocation open while one piece of `wait_until` work runs. The
/// span is released when the last guard drops after the terminal status has
/// been recorded, so work that settles before the reply changes nothing.
pub(crate) struct InvocationWorkGuard(ActorInvocationTelemetry);

/// Active actor invocation fields exposed to foreign-runtime adapters.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ActorInvocationTraceContext {
	pub ray_id: Option<String>,
	/// Present only while the invocation runs inside a valid span.
	pub span: Option<ActorInvocationSpanContext>,
}

/// W3C span context of the current invocation span. A span context is either
/// complete or absent, so these fields are never optional individually.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ActorInvocationSpanContext {
	pub trace_id: String,
	pub span_id: String,
	pub trace_flags: u8,
	pub traceparent: String,
	pub tracestate: Option<String>,
}

/// The closed set of SQLite operations that get a span.
///
/// Both names are `&'static str`, so starting one of these spans allocates
/// nothing. Adding an operation is a compile error here rather than a silently
/// wrong span name.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SqliteOperation {
	Exec,
	Execute,
	ExecuteBatch,
	Query,
	Run,
	TransactionBegin,
	TransactionExec,
	TransactionExecute,
	TransactionCommit,
	TransactionRollback,
}

impl SqliteOperation {
	fn as_str(self) -> &'static str {
		match self {
			Self::Exec => "exec",
			Self::Execute => "execute",
			Self::ExecuteBatch => "execute_batch",
			Self::Query => "query",
			Self::Run => "run",
			Self::TransactionBegin => "transaction.begin",
			Self::TransactionExec => "transaction.exec",
			Self::TransactionExecute => "transaction.execute",
			Self::TransactionCommit => "transaction.commit",
			Self::TransactionRollback => "transaction.rollback",
		}
	}

	fn span_name(self) -> &'static str {
		match self {
			Self::Exec => "rivet.sqlite.exec",
			Self::Execute => "rivet.sqlite.execute",
			Self::ExecuteBatch => "rivet.sqlite.execute_batch",
			Self::Query => "rivet.sqlite.query",
			Self::Run => "rivet.sqlite.run",
			Self::TransactionBegin => "rivet.sqlite.transaction.begin",
			Self::TransactionExec => "rivet.sqlite.transaction.exec",
			Self::TransactionExecute => "rivet.sqlite.transaction.execute",
			Self::TransactionCommit => "rivet.sqlite.transaction.commit",
			Self::TransactionRollback => "rivet.sqlite.transaction.rollback",
		}
	}
}

pub(crate) struct SqliteOperationSpan {
	span: Option<tracing::Span>,
}

impl ActionInvocationSpan {
	pub(crate) fn start(
		ctx: &ActorContext,
		action_name: &str,
		incoming: IncomingInvocationContext,
	) -> Self {
		let identity = ctx.telemetry_identity();
		let ray_id = incoming.ray_id;
		let span = if tracing::enabled!(target: "rivetkit::telemetry", tracing::Level::INFO) {
				let span = tracing::info_span!(
					target: "rivetkit::telemetry",
					parent: None,
					"rivet.actor.invoke",
					otel.kind = "server",
					rivet.invocation.type = "action",
					rivet.actor.id = %identity.actor_id,
					rivet.actor.name = %identity.actor_name,
					rivet.actor.key = %identity.actor_key,
					rivet.action.name = %action_name,
					rivet.ray.id = tracing::field::Empty,
					otel.status_code = tracing::field::Empty,
					error.type = tracing::field::Empty,
				);
				span.record("rivet.ray.id", ray_id.as_deref());
				if let Some(parent) = incoming.remote_parent {
					span.set_parent(opentelemetry::Context::new().with_remote_span_context(parent));
				}
				Some(span)
		} else {
			None
		};

		Self {
			telemetry: ActorInvocationTelemetry::new(ray_id, span, identity),
		}
	}

	pub(crate) fn telemetry(&self) -> ActorInvocationTelemetry {
		self.telemetry.clone()
	}

	pub(crate) fn finish(self, error: Option<&anyhow::Error>) {
		self.telemetry.finish(error);
	}
}

impl Drop for ActionInvocationSpan {
	fn drop(&mut self) {
		self.telemetry.finish_dropped();
	}
}

impl ActorInvocationTelemetry {
	fn new(
		ray_id: Option<String>,
		span: Option<tracing::Span>,
		identity: Arc<ActorTelemetryIdentity>,
	) -> Self {
		Self(Arc::new(InvocationInner {
			ray_id,
			span: Mutex::new(span),
			finished: AtomicBool::new(false),
			pending_work: AtomicUsize::new(0),
			identity,
		}))
	}

	/// Registers work that outlives the reply, so the invocation span stays
	/// open and keeps parenting operations until the returned guard drops.
	pub(crate) fn hold_open(&self) -> InvocationWorkGuard {
		self.0.pending_work.fetch_add(1, Ordering::SeqCst);
		InvocationWorkGuard(self.clone())
	}

	/// Returns correlation fields only while this actor invocation is active.
	#[doc(hidden)]
	pub fn trace_context(&self) -> Option<ActorInvocationTraceContext> {
		let active = self.active()?;
		let span = active.span.lock().clone().and_then(|span| {
			let context = span.context();
			let context_span = context.span();
			let span_context = context_span.span_context();
			if !span_context.is_valid() {
				return None;
			}
			let tracestate = span_context.trace_state().header();
			Some(ActorInvocationSpanContext {
				trace_id: span_context.trace_id().to_string(),
				span_id: span_context.span_id().to_string(),
				trace_flags: span_context.trace_flags().to_u8(),
				traceparent: format!(
					"00-{}-{}-{:02x}",
					span_context.trace_id(),
					span_context.span_id(),
					span_context.trace_flags().to_u8(),
				),
				tracestate: if tracestate.is_empty() {
					None
				} else {
					Some(tracestate)
				},
			})
		});

		Some(ActorInvocationTraceContext {
			ray_id: active.ray_id.clone(),
			span,
		})
	}

	pub(crate) fn start_sqlite(&self, operation: SqliteOperation) -> Option<SqliteOperationSpan> {
		let parent = self.active()?.span.lock().clone()?;
		let span = tracing::info_span!(
			target: "rivetkit::telemetry",
			parent: &parent,
			"rivet.sqlite.operation",
			otel.name = operation.span_name(),
			otel.kind = "internal",
			rivet.operation.system = "sqlite",
			rivet.operation.name = operation.as_str(),
			rivet.ray.id = self.0.ray_id.as_deref(),
			rivet.actor.id = %self.0.identity.actor_id,
			rivet.actor.name = %self.0.identity.actor_name,
			rivet.actor.key = %self.0.identity.actor_key,
			otel.status_code = tracing::field::Empty,
			error.type = tracing::field::Empty,
		);
		Some(SqliteOperationSpan { span: Some(span) })
	}

	fn finish(&self, error: Option<&anyhow::Error>) {
		let Some(span) = self.claim_terminal() else {
			return;
		};
		record_outcome(&span, error);
		self.mark_reply_sent(&span);
		self.release_span_if_settled();
	}

	fn finish_dropped(&self) {
		let Some(span) = self.claim_terminal() else {
			return;
		};
		span.record("otel.status_code", "ERROR");
		span.record("error.type", "actor.dropped_reply");
		self.mark_reply_sent(&span);
		self.release_span_if_settled();
	}

	/// Borrows the invocation while it is still open: before its status is
	/// recorded, or after it while `wait_until` work from it still runs. A
	/// settled invocation yields nothing, so late SQLite work and retained
	/// handles cannot attach to a span that has already ended.
	fn active(&self) -> Option<&InvocationInner> {
		let open = !self.0.finished.load(Ordering::SeqCst)
			|| self.0.pending_work.load(Ordering::SeqCst) > 0;
		if open { Some(&*self.0) } else { None }
	}

	/// Claims the terminal record, so the finish and drop paths cannot both
	/// record a status for the same invocation. The span stays in its slot
	/// until `release_span_if_settled` empties it.
	fn claim_terminal(&self) -> Option<tracing::Span> {
		if self.0.finished.swap(true, Ordering::SeqCst) {
			return None;
		}
		self.0.span.lock().clone()
	}

	/// Marks the moment the caller got its answer when the span will outlive
	/// it, so the reply point stays visible inside a span that is still
	/// running `wait_until` work.
	fn mark_reply_sent(&self, span: &tracing::Span) {
		if self.0.pending_work.load(Ordering::SeqCst) > 0 {
			tracing::info!(target: "rivetkit::telemetry", parent: span, "reply sent");
		}
	}

	/// Ends the span, which exports it, unless `wait_until` work is still
	/// holding the invocation open. The last guard to drop ends it instead.
	/// Both sides set their flag before reading the other's, so the two
	/// cannot each see the other as still pending and leave the span behind.
	fn release_span_if_settled(&self) {
		if self.0.pending_work.load(Ordering::SeqCst) == 0 {
			self.0.span.lock().take();
		}
	}
}

impl Drop for InvocationWorkGuard {
	fn drop(&mut self) {
		let inner = &self.0.0;
		let was_last = inner.pending_work.fetch_sub(1, Ordering::SeqCst) == 1;
		if was_last && inner.finished.load(Ordering::SeqCst) {
			inner.span.lock().take();
		}
	}
}

impl SqliteOperationSpan {
	pub(crate) fn span(&self) -> tracing::Span {
		self.span.as_ref().expect("sqlite span is present").clone()
	}

	pub(crate) fn finish(&mut self, error: Option<&anyhow::Error>) {
		let Some(span) = self.span.take() else {
			return;
		};
		record_outcome(&span, error);
	}
}

impl Drop for SqliteOperationSpan {
	fn drop(&mut self) {
		let Some(span) = self.span.take() else {
			return;
		};
		span.record("otel.status_code", "ERROR");
		span.record("error.type", OPERATION_ABANDONED_ERROR_TYPE);
	}
}

/// Records the terminal status and error identity of a finished span.
fn record_outcome(span: &tracing::Span, error: Option<&anyhow::Error>) {
	span.record(
		"otel.status_code",
		if error.is_none() { "OK" } else { "ERROR" },
	);
	if let Some(error) = error {
		let error = rivet_error::RivetError::extract(error);
		span.record("error.type", format!("{}.{}", error.group(), error.code()));
	}
}

fn parse_remote_parent(traceparent: Option<&str>, tracestate: Option<&str>) -> Option<SpanContext> {
	let mut fields = traceparent?.split('-');
	let version = fields.next()?;
	let trace_id = fields.next()?;
	let span_id = fields.next()?;
	let flags = fields.next()?;
	if fields.next().is_some()
		|| version.len() != 2
		|| version.eq_ignore_ascii_case("ff")
		|| trace_id.len() != 32
		|| span_id.len() != 16
		|| flags.len() != 2
	{
		return None;
	}

	let trace_id = TraceId::from_hex(trace_id).ok()?;
	let span_id = SpanId::from_hex(span_id).ok()?;
	let flags = u8::from_str_radix(flags, 16).ok()?;
	let trace_state = tracestate
		.and_then(|value| TraceState::from_str(value).ok())
		.unwrap_or_default();
	let context = SpanContext::new(trace_id, span_id, TraceFlags::new(flags), true, trace_state);
	if context.is_valid() {
		Some(context)
	} else {
		None
	}
}
