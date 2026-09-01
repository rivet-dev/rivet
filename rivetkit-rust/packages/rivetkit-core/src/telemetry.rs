//! Internal OpenTelemetry spans owned by the actor runtime.

#[cfg(feature = "native-runtime")]
pub mod export;

use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use opentelemetry::trace::{
	SpanContext, SpanId, TraceContextExt as _, TraceFlags, TraceId, TraceState,
};
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

use crate::ActorContext;

/// Correlation fields accepted at an invocation boundary.
#[derive(Debug, Default)]
pub struct IncomingInvocationContext {
	pub(crate) ray_id: Option<String>,
	remote_parent: Option<SpanContext>,
}

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

/// Shared invocation state. The span never changes once the invocation starts,
/// so only the terminal record needs guarding: `finished` lets exactly one of
/// the explicit finish path and the drop path record a status.
#[derive(Debug)]
struct InvocationInner {
	span: Option<tracing::Span>,
	finished: AtomicBool,
	identity: Arc<ActorTelemetryIdentity>,
}

const OPERATION_ABANDONED_ERROR_TYPE: &str = "actor.operation_abandoned";

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
		let span = tracing::enabled!(target: "rivetkit::telemetry", tracing::Level::INFO).then(|| {
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
			if let Some(ray_id) = incoming.ray_id.as_deref() {
				span.record("rivet.ray.id", ray_id);
			}
			if let Some(parent) = incoming.remote_parent {
				span.set_parent(opentelemetry::Context::new().with_remote_span_context(parent));
			}
			span
		});

		Self {
			telemetry: ActorInvocationTelemetry::new(span, identity),
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
		span: Option<tracing::Span>,
		identity: Arc<ActorTelemetryIdentity>,
	) -> Self {
		Self(Arc::new(InvocationInner {
			span,
			finished: AtomicBool::new(false),
			identity,
		}))
	}

	pub(crate) fn start_sqlite(&self, operation: SqliteOperation) -> Option<SqliteOperationSpan> {
		let parent = self.0.span.as_ref()?;
		let span = tracing::info_span!(
			target: "rivetkit::telemetry",
			parent: parent,
			"rivet.sqlite.operation",
			otel.name = operation.span_name(),
			otel.kind = "internal",
			rivet.operation.system = "sqlite",
			rivet.operation.name = operation.as_str(),
			rivet.actor.id = %self.0.identity.actor_id,
			rivet.actor.name = %self.0.identity.actor_name,
			rivet.actor.key = %self.0.identity.actor_key,
			otel.status_code = tracing::field::Empty,
			error.type = tracing::field::Empty,
		);
		Some(SqliteOperationSpan { span: Some(span) })
	}

	fn finish(&self, error: Option<&anyhow::Error>) {
		let Some(span) = self.take_span() else {
			return;
		};
		record_outcome(span, error);
	}

	fn finish_dropped(&self) {
		let Some(span) = self.take_span() else {
			return;
		};
		span.record("otel.status_code", "ERROR");
		span.record("error.type", "actor.dropped_reply");
	}

	/// Claims the terminal record, so the finish and drop paths cannot both
	/// record a status for the same invocation.
	fn take_span(&self) -> Option<&tracing::Span> {
		if self.0.finished.swap(true, Ordering::AcqRel) {
			return None;
		}
		self.0.span.as_ref()
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
