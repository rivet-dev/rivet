//! Internal OpenTelemetry spans owned by the actor runtime.

#[cfg(feature = "native-runtime")]
pub mod export;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use opentelemetry::Context;
use opentelemetry::propagation::{Extractor, TextMapPropagator as _};
use opentelemetry::trace::{SpanContext, TraceContextExt as _};
use opentelemetry_http::{HeaderExtractor, HeaderInjector};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use parking_lot::Mutex;
use rivetkit_client_protocol::ray_id::RayId;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

use crate::ActorContext;
use crate::actor::metrics::{ActorMetrics, InvocationStatus, InvocationType};
use crate::actor::queue::QueueMessage;
use crate::time::Instant;

/// Correlation fields accepted at an invocation boundary.
#[derive(Debug, Default)]
pub struct IncomingInvocationContext {
	pub(crate) ray_id: Option<String>,
	remote_parent: Option<SpanContext>,
}

pub(crate) use rivetkit_client_protocol::ray_id::HEADER_RIVET_RAY_ID;

const HEADER_TRACEPARENT: &str = "traceparent";
const HEADER_TRACESTATE: &str = "tracestate";

impl IncomingInvocationContext {
	/// Reads the ray ID and W3C trace context an HTTP request carries. Every
	/// HTTP entry point into an actor reads them through here, so they all
	/// apply the same bounds.
	pub(crate) fn from_http_headers(headers: &http::HeaderMap) -> Self {
		Self {
			ray_id: invocation_ray_id(headers),
			remote_parent: if headers.contains_key(HEADER_TRACEPARENT) {
				extract_remote_parent(&HeaderExtractor(headers))
			} else {
				None
			},
		}
	}
}

/// Reads the caller's ray ID. The header is untrusted, so it is bounded by
/// the rule shared with the clients that send it; anything else counts as
/// absent.
fn invocation_ray_id(headers: &http::HeaderMap) -> Option<String> {
	let value = headers.get(HEADER_RIVET_RAY_ID)?.to_str().ok()?;
	RayId::parse(value.to_owned()).ok().map(RayId::into_string)
}

/// Name a request invocation is reported under, in place of an action name.
/// It cannot collide with an action, because the metric and span carry the
/// invocation type beside it.
const REQUEST_INVOCATION_NAME: &str = "onRequest";

/// Name a queue send invocation is reported under. The queue itself is an
/// attribute, so one series covers every queue of an actor.
const QUEUE_SEND_INVOCATION_NAME: &str = "queue.send";

/// What an invocation ran, which decides its name and the attributes that
/// identify it on the span.
#[derive(Clone, Copy, Debug)]
enum InvocationSubject<'a> {
	Action(&'a str),
	Request { method: &'a str },
	QueueSend { queue: &'a str },
}

/// Owns the complete lifecycle of one actor invocation.
#[derive(Debug)]
pub(crate) struct ActorInvocation {
	telemetry: ActorInvocationTelemetry,
	metrics: ActorMetrics,
	action_name: String,
	invocation_type: InvocationType,
	started_at: Instant,
}

/// Opaque invocation context carried across foreign-runtime adapters.
///
/// The second field is the application span the host runtime had active when
/// it resolved this handle. Core cannot see the host's span stack, so a span
/// Core opens through this handle parents there when it is set and to the
/// invocation span otherwise. Every clone of a handle shares one invocation.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ActorInvocationTelemetry(Arc<InvocationInner>, Option<SpanContext>);

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

/// Where later work came from: the ray ID of the invocation that caused it and
/// the span that was active there. Persisted beside schedules and queue
/// messages so the work they cause can link back to its origin.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceOrigin {
	pub ray_id: Option<String>,
	pub traceparent: Option<String>,
	pub tracestate: Option<String>,
}

impl TraceOrigin {
	/// True when there is nothing to persist: the work was caused outside any
	/// traced invocation.
	pub fn is_empty(&self) -> bool {
		self.ray_id.is_none() && self.traceparent.is_none() && self.tracestate.is_none()
	}
}

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

/// One call from this invocation out to another actor, held open across a
/// foreign-runtime boundary.
///
/// The call is made by the host runtime's client, so it is opened and closed by
/// two separate calls rather than by one Rust scope. Dropping this without
/// finishing records the call as cancelled, matching how a dropped SQLite span
/// is treated.
#[doc(hidden)]
pub struct OutboundCallInvocation {
	span: Option<tracing::Span>,
	context: Option<ActorInvocationSpanContext>,
}

impl ActorInvocation {
	pub(crate) fn start_action(
		ctx: &ActorContext,
		action_name: &str,
		incoming: IncomingInvocationContext,
	) -> Self {
		Self::start(
			ctx,
			InvocationSubject::Action(action_name),
			InvocationType::Action,
			incoming.ray_id,
			incoming.remote_parent,
			None,
		)
	}

	pub(crate) fn start_scheduled(
		ctx: &ActorContext,
		action_name: &str,
		origin: TraceOrigin,
	) -> Self {
		let origin_parent =
			parse_remote_parent(origin.traceparent.as_deref(), origin.tracestate.as_deref());
		Self::start(
			ctx,
			InvocationSubject::Action(action_name),
			InvocationType::Scheduled,
			origin.ray_id,
			None,
			origin_parent,
		)
	}

	/// Starts the invocation for one message sent into `queue_name` from
	/// outside the actor. It ends when the send is acknowledged, or when the
	/// sender's wait for a completion ends.
	pub(crate) fn start_queue_send(
		ctx: &ActorContext,
		queue_name: &str,
		incoming: IncomingInvocationContext,
	) -> Self {
		Self::start(
			ctx,
			InvocationSubject::QueueSend { queue: queue_name },
			InvocationType::QueueSend,
			incoming.ray_id,
			incoming.remote_parent,
			None,
		)
	}

	/// Starts the invocation for one raw HTTP request served by `onRequest`.
	/// The span is named after the handler rather than the path, because a
	/// path is caller-supplied and would make the name a cardinality surface.
	pub(crate) fn start_request(
		ctx: &ActorContext,
		request: &crate::actor::messages::Request,
		incoming: IncomingInvocationContext,
	) -> Self {
		Self::start(
			ctx,
			InvocationSubject::Request {
				method: request.method().as_str(),
			},
			InvocationType::Request,
			incoming.ray_id,
			incoming.remote_parent,
			None,
		)
	}

	fn start(
		ctx: &ActorContext,
		subject: InvocationSubject<'_>,
		invocation_type: InvocationType,
		ray_id: Option<String>,
		parent: Option<SpanContext>,
		link: Option<SpanContext>,
	) -> Self {
		let identity = ctx.telemetry_identity();
		// Use bounded names for both spans and metrics.
		let (action_name, http_method, queue_name) = match subject {
			InvocationSubject::Action(name) => (ctx.metrics().label_action_name(name), None, None),
			InvocationSubject::Request { method } => (REQUEST_INVOCATION_NAME, Some(method), None),
			InvocationSubject::QueueSend { queue } => (
				QUEUE_SEND_INVOCATION_NAME,
				None,
				Some(ctx.metrics().label_queue_name(queue)),
			),
		};
		let action_name = action_name.to_owned();
		let span = if tracing::enabled!(target: "rivetkit::telemetry", tracing::Level::INFO) {
			let span = tracing::info_span!(
				target: "rivetkit::telemetry",
				parent: None,
				"rivet.actor.invoke",
				otel.name = %format!("{}/{}", identity.actor_name, action_name),
				otel.kind = invocation_type.otel_kind(),
				rivet.invocation.type = invocation_type.as_label(),
				rivet.actor.id = %identity.actor_id,
				rivet.actor.name = %identity.actor_name,
				rivet.actor.key = %identity.actor_key,
				rivet.action.name = tracing::field::Empty,
				rivet.ray.id = tracing::field::Empty,
				http.request.method = tracing::field::Empty,
				http.response.status_code = tracing::field::Empty,
				rivet.queue.name = tracing::field::Empty,
				otel.status_code = tracing::field::Empty,
				error.type = tracing::field::Empty,
			);
			span.record("rivet.ray.id", ray_id.as_deref());
			match (http_method, queue_name) {
				(Some(method), _) => span.record("http.request.method", method),
				(None, Some(queue)) => span.record("rivet.queue.name", queue),
				(None, None) => span.record("rivet.action.name", &action_name),
			};
			if let Some(parent) = parent {
				span.set_parent(opentelemetry::Context::new().with_remote_span_context(parent));
			}
			if let Some(link) = link {
				span.add_link(link);
			}
			Some(span)
		} else {
			None
		};

		Self {
			telemetry: ActorInvocationTelemetry::new(ray_id, span, identity),
			metrics: ctx.metrics().clone(),
			action_name,
			invocation_type,
			started_at: Instant::now(),
		}
	}

	pub(crate) fn telemetry(&self) -> ActorInvocationTelemetry {
		self.telemetry.clone()
	}

	pub(crate) fn finish(mut self, error: Option<&anyhow::Error>) {
		self.finish_with_status(
			error.map_or(InvocationStatus::Ok, InvocationStatus::from_error),
			error,
		);
	}

	/// Finishes a request invocation with the HTTP status the handler
	/// answered, which is recorded on the span beside the outcome. A 5xx
	/// answer counts as a failed invocation with the status as its error
	/// identity, following the HTTP server span convention. An error means no
	/// response was produced, so only the error identity is recorded.
	pub(crate) fn finish_request(
		mut self,
		response: std::result::Result<&crate::actor::messages::ActorHttpResponse, &anyhow::Error>,
	) {
		match response {
			Ok(response) => {
				let status = response.status();
				self.telemetry.record_http_status(status);
				if status >= 500 {
					self.finish_with_failure(InvocationFailure::HttpStatus(status));
				} else {
					self.finish_with_status(InvocationStatus::Ok, None);
				}
			}
			Err(error) => self.finish(Some(error)),
		}
	}

	fn finish_with_status(&mut self, status: InvocationStatus, error: Option<&anyhow::Error>) {
		let Some(span) = self.telemetry.claim_terminal() else {
			return;
		};
		self.record_finished(span, status, error.map(InvocationFailure::Error));
	}

	fn finish_with_failure(&mut self, failure: InvocationFailure<'_>) {
		let Some(span) = self.telemetry.claim_terminal() else {
			return;
		};
		self.record_finished(span, InvocationStatus::Error, Some(failure));
	}

	/// Records the terminal metric and span status of an invocation whose
	/// completion the caller has already claimed through `claim_terminal`.
	/// The metric measures what the caller waited for, so it is recorded here
	/// even when `wait_until` work keeps the span open past this point.
	fn record_finished(
		&self,
		span: Option<tracing::Span>,
		status: InvocationStatus,
		failure: Option<InvocationFailure<'_>>,
	) {
		self.metrics.record_invocation(
			&self.action_name,
			self.invocation_type,
			status,
			self.started_at.elapsed(),
		);
		if let Some(span) = span {
			match failure {
				Some(InvocationFailure::HttpStatus(status)) => {
					span.record("otel.status_code", "ERROR");
					span.record("error.type", status.to_string());
				}
				Some(InvocationFailure::Error(error)) => record_outcome(&span, Some(error)),
				None => record_outcome(&span, None),
			}
			self.telemetry.mark_reply_sent(&span);
		}
		self.telemetry.release_span_if_settled();
	}
}

/// Why an invocation is recorded as failed: an error crossing the runtime
/// boundary, or a request the handler answered with a server error status.
enum InvocationFailure<'a> {
	Error(&'a anyhow::Error),
	HttpStatus(u16),
}

impl Drop for ActorInvocation {
	fn drop(&mut self) {
		// `finish` consumes the invocation, so this runs on the completed path
		// too. Claim the terminal record first, so the dropped-reply error is
		// only built for an invocation that really was dropped.
		let Some(span) = self.telemetry.claim_terminal() else {
			return;
		};
		let error = crate::error::ActorLifecycle::DroppedReply.build();
		self.record_finished(
			span,
			InvocationStatus::Dropped,
			Some(InvocationFailure::Error(&error)),
		);
	}
}

impl ActorInvocationTelemetry {
	fn new(
		ray_id: Option<String>,
		span: Option<tracing::Span>,
		identity: Arc<ActorTelemetryIdentity>,
	) -> Self {
		Self(
			Arc::new(InvocationInner {
				ray_id,
				span: Mutex::new(span),
				finished: AtomicBool::new(false),
				pending_work: AtomicUsize::new(0),
				identity,
			}),
			None,
		)
	}

	/// Returns a handle for the same invocation whose spans parent to the
	/// application span identified by `traceparent` and `tracestate`. Invalid
	/// or absent context yields a handle that parents to the invocation span.
	pub(crate) fn with_application_span(
		&self,
		traceparent: Option<&str>,
		tracestate: Option<&str>,
	) -> Self {
		Self(self.0.clone(), parse_remote_parent(traceparent, tracestate))
	}

	/// Records the status a request invocation answered with.
	fn record_http_status(&self, status: u16) {
		if let Some(span) = self.0.span.lock().as_ref() {
			span.record("http.response.status_code", status);
		}
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
		let span = active
			.span
			.lock()
			.clone()
			.and_then(|span| span_context_of(&span));

		Some(ActorInvocationTraceContext {
			ray_id: active.ray_id.clone(),
			span,
		})
	}

	/// Trace origin work caused by this invocation records: the invocation's
	/// ray ID, and the application span active in the host runtime at that
	/// moment, or the invocation span when there was none. Work that links
	/// back to it then points at the code that caused it rather than at the
	/// whole invocation around that code.
	pub(crate) fn trace_origin(&self) -> TraceOrigin {
		let Some(active) = self.active() else {
			return TraceOrigin::default();
		};
		let span = self
			.1
			.clone()
			.or_else(|| active.span.lock().as_ref().and_then(otel_span_context_of));
		let (traceparent, tracestate) = span
			.as_ref()
			.and_then(propagation_headers)
			.map_or((None, None), |(traceparent, tracestate)| {
				(Some(traceparent), tracestate)
			});
		TraceOrigin {
			ray_id: active.ray_id.clone(),
			traceparent,
			tracestate,
		}
	}

	/// Opens the span covering one call out to another actor.
	///
	/// The callee parents to this span rather than to the invocation making the
	/// call, so the time spent reaching it, which includes routing and waking a
	/// sleeping actor, is attributed to the call instead of falling in the gap
	/// between the two invocations.
	/// When this handle carries the application span active in the host
	/// runtime, the call span parents there instead, which is what puts a
	/// callee under the application span that issued the call rather than
	/// beside it.
	pub(crate) fn start_outbound_call(
		&self,
		actor_name: &str,
		action_name: &str,
	) -> Option<OutboundCallInvocation> {
		let invocation_span = self.active()?.span.lock().clone()?;
		let span = tracing::info_span!(
			target: "rivetkit::telemetry",
			parent: &invocation_span,
			"rivet.actor.call",
			otel.name = %format!("{actor_name}/{action_name}"),
			otel.kind = "client",
			// Omit rivet.invocation.type: this measures the caller waiting, not the callee running.
			rivet.actor.name = %actor_name,
			rivet.action.name = %action_name,
			rivet.ray.id = self.0.ray_id.as_deref(),
			otel.status_code = tracing::field::Empty,
			error.type = tracing::field::Empty,
		);
		if let Some(application_span) = &self.1 {
			span.set_parent(
				opentelemetry::Context::new().with_remote_span_context(application_span.clone()),
			);
		}
		let context = span_context_of(&span);
		Some(OutboundCallInvocation {
			span: Some(span),
			context,
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
		if let Some(application_span) = &self.1 {
			span.set_parent(
				opentelemetry::Context::new().with_remote_span_context(application_span.clone()),
			);
		}
		Some(SqliteOperationSpan { span: Some(span) })
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
	fn claim_terminal(&self) -> Option<Option<tracing::Span>> {
		if self.0.finished.swap(true, Ordering::SeqCst) {
			return None;
		}
		Some(self.0.span.lock().clone())
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

impl OutboundCallInvocation {
	/// W3C context of this call's span, to send to the callee so it parents
	/// here. Absent when tracing is disabled.
	pub fn span_context(&self) -> Option<ActorInvocationSpanContext> {
		self.context.clone()
	}

	/// Records the call's outcome. `error` is the failure the callee returned,
	/// and its group and code become the span's `error.type`.
	pub fn finish(mut self, error: Option<&anyhow::Error>) {
		let Some(span) = self.span.take() else {
			return;
		};
		record_outcome(&span, error);
	}
}

impl Drop for OutboundCallInvocation {
	fn drop(&mut self) {
		let Some(span) = self.span.take() else {
			return;
		};
		span.record("otel.status_code", "ERROR");
		span.record("error.type", OPERATION_ABANDONED_ERROR_TYPE);
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

/// W3C fields of a span context, or nothing when it is not valid and so
/// carries nothing worth propagating.
fn span_context_fields(span_context: &SpanContext) -> Option<ActorInvocationSpanContext> {
	if !span_context.is_valid() {
		return None;
	}
	let tracestate = span_context.trace_state().header();
	Some(ActorInvocationSpanContext {
		trace_id: span_context.trace_id().to_string(),
		span_id: span_context.span_id().to_string(),
		trace_flags: span_context.trace_flags().to_u8(),
		tracestate: (!tracestate.is_empty()).then_some(tracestate),
	})
}

/// Reads a span's W3C context, or nothing when it carries no valid context to
/// propagate.
fn span_context_of(span: &tracing::Span) -> Option<ActorInvocationSpanContext> {
	otel_span_context_of(span)
		.as_ref()
		.and_then(span_context_fields)
}

fn otel_span_context_of(span: &tracing::Span) -> Option<SpanContext> {
	let context = span.context();
	let context_span = context.span();
	let span_context = context_span.span_context();
	span_context.is_valid().then(|| span_context.clone())
}

/// Opens the span covering the moment one queue message is handed to the
/// actor. It links to the span that sent the message, which is what connects
/// the consumer's trace to the sender's. Inside an invocation it sits under
/// that invocation's application span, or the invocation span, and carries its
/// ray ID. Outside one, as from the run handler, it is a root span carrying the
/// ray ID the message was sent under. It closes when the caller drops it, which
/// `try_receive_batch` does as it hands the message back.
pub(crate) fn start_queue_receive(ctx: &ActorContext, message: &QueueMessage) -> tracing::Span {
	if !tracing::enabled!(target: "rivetkit::telemetry", tracing::Level::INFO) {
		return tracing::Span::none();
	}
	let identity = ctx.telemetry_identity();
	let invocation = ctx
		.1
		.as_ref()
		.and_then(|telemetry| telemetry.active().map(|inner| (telemetry, inner)));
	let span = tracing::info_span!(
		target: "rivetkit::telemetry",
		parent: None,
		"rivet.queue.receive",
		otel.name = %format!("{}/queue.receive", identity.actor_name),
		otel.kind = "consumer",
		rivet.actor.id = %identity.actor_id,
		rivet.actor.name = %identity.actor_name,
		rivet.actor.key = %identity.actor_key,
		rivet.queue.name = ctx.metrics().label_queue_name(&message.name),
		rivet.ray.id = tracing::field::Empty,
	);
	match invocation {
		Some((telemetry, inner)) => {
			span.record("rivet.ray.id", inner.ray_id.as_deref());
			let parent = match &telemetry.1 {
				Some(application_span) => Some(
					opentelemetry::Context::new()
						.with_remote_span_context(application_span.clone()),
				),
				None => inner.span.lock().as_ref().map(|span| span.context()),
			};
			if let Some(parent) = parent {
				span.set_parent(parent);
			}
		}
		None => {
			if let Some(ray_id) = &message.trace_origin.ray_id {
				span.record("rivet.ray.id", ray_id);
			}
		}
	}
	if let Some(link) = parse_remote_parent(
		message.trace_origin.traceparent.as_deref(),
		message.trace_origin.tracestate.as_deref(),
	) {
		span.add_link(link);
	}
	span
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
	traceparent?;
	extract_remote_parent(&TraceHeaders {
		traceparent,
		tracestate,
	})
}

fn extract_remote_parent(extractor: &dyn Extractor) -> Option<SpanContext> {
	let context = TraceContextPropagator::new().extract_with_context(&Context::new(), extractor);
	let span = context.span();
	let span_context = span.span_context();
	span_context.is_valid().then(|| span_context.clone())
}

fn propagation_headers(span_context: &SpanContext) -> Option<(String, Option<String>)> {
	let context = Context::new().with_remote_span_context(span_context.clone());
	let mut headers = http::HeaderMap::new();
	TraceContextPropagator::new().inject_context(&context, &mut HeaderInjector(&mut headers));
	let traceparent = headers.get(HEADER_TRACEPARENT)?.to_str().ok()?.to_owned();
	let tracestate = headers
		.get(HEADER_TRACESTATE)
		.and_then(|value| value.to_str().ok())
		.filter(|value| !value.is_empty())
		.map(str::to_owned);
	Some((traceparent, tracestate))
}

struct TraceHeaders<'a> {
	traceparent: Option<&'a str>,
	tracestate: Option<&'a str>,
}

impl Extractor for TraceHeaders<'_> {
	fn get(&self, key: &str) -> Option<&str> {
		match key {
			key if key.eq_ignore_ascii_case(HEADER_TRACEPARENT) => self.traceparent,
			key if key.eq_ignore_ascii_case(HEADER_TRACESTATE) => self.tracestate,
			_ => None,
		}
	}

	fn keys(&self) -> Vec<&str> {
		let mut keys = Vec::with_capacity(2);
		if self.traceparent.is_some() {
			keys.push(HEADER_TRACEPARENT);
		}
		if self.tracestate.is_some() {
			keys.push(HEADER_TRACESTATE);
		}
		keys
	}
}
