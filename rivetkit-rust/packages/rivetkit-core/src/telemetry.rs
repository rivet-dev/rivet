//! Internal OpenTelemetry spans owned by the actor runtime.

#[cfg(feature = "native-runtime")]
pub mod export;

use std::str::FromStr as _;

use opentelemetry::trace::{
	SpanContext, SpanId, TraceContextExt as _, TraceFlags, TraceId, TraceState,
};
use tracing_opentelemetry::OpenTelemetrySpanExt as _;

use crate::{ActorContext, format_actor_key};

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
	span: Option<tracing::Span>,
}

impl ActionInvocationSpan {
	pub(crate) fn start(
		ctx: &ActorContext,
		action_name: &str,
		incoming: IncomingInvocationContext,
	) -> Self {
		if !tracing::enabled!(target: "rivetkit::telemetry", tracing::Level::INFO) {
			return Self { span: None };
		}

		let span = tracing::info_span!(
			target: "rivetkit::telemetry",
			parent: None,
			"rivet.actor.invoke",
			otel.kind = "server",
			rivet.invocation.type = "action",
			rivet.actor.id = %ctx.actor_id(),
			rivet.actor.name = %ctx.name(),
			rivet.actor.key = %format_actor_key(ctx.key()),
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

		Self { span: Some(span) }
	}

	pub(crate) fn finish(mut self, error: Option<&anyhow::Error>) {
		let Some(span) = self.span.take() else {
			return;
		};
		span.record(
			"otel.status_code",
			if error.is_none() { "OK" } else { "ERROR" },
		);
		if let Some(error) = error {
			let error = rivet_error::RivetError::extract(error);
			span.record("error.type", format!("{}.{}", error.group(), error.code()));
		}
	}
}

impl Drop for ActionInvocationSpan {
	fn drop(&mut self) {
		let Some(span) = self.span.take() else {
			return;
		};
		span.record("otel.status_code", "ERROR");
		span.record("error.type", "actor.dropped_reply");
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
