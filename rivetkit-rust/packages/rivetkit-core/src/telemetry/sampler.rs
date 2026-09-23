//! Sampling of the runtime's spans.

use opentelemetry::trace::{
	Link, SamplingDecision, SamplingResult, SpanContext, SpanKind, TraceContextExt as _, TraceId,
};
use opentelemetry::{Context, KeyValue, Value};
use opentelemetry_sdk::trace::{Sampler, ShouldSample};

use super::SAMPLE_RATIO_ATTRIBUTE;

/// Applies an actor's sample rate to its invocations and defers every other
/// decision to the process sampler. A rate lowers what is recorded and never
/// raises it.
#[derive(Clone, Debug)]
pub(super) struct ActorSampler {
	process: Box<dyn ShouldSample>,
}

impl ActorSampler {
	pub(super) fn new(process: Box<dyn ShouldSample>) -> Self {
		Self { process }
	}
}

impl ShouldSample for ActorSampler {
	fn should_sample(
		&self,
		parent_context: Option<&Context>,
		trace_id: TraceId,
		name: &str,
		span_kind: &SpanKind,
		attributes: &[KeyValue],
		links: &[Link],
	) -> SamplingResult {
		let parent = parent_context
			.filter(|context| context.has_active_span())
			.map(|context| context.span().span_context().clone())
			.filter(SpanContext::is_valid);
		let actor_ratio = actor_sample_ratio(attributes);

		match (parent, actor_ratio) {
			(Some(parent), Some(_)) if !parent.is_sampled() => follow(&parent),
			(Some(_), Some(ratio)) | (None, Some(ratio)) => Sampler::TraceIdRatioBased(ratio)
				.should_sample(parent_context, trace_id, name, span_kind, attributes, links),
			// A local parent already decided for its children, such as SQLite spans.
			(Some(parent), None) if !parent.is_remote() => follow(&parent),
			(Some(_), None) | (None, None) => self.process.should_sample(
				parent_context,
				trace_id,
				name,
				span_kind,
				attributes,
				links,
			),
		}
	}
}

fn actor_sample_ratio(attributes: &[KeyValue]) -> Option<f64> {
	attributes
		.iter()
		.find(|attribute| attribute.key.as_str() == SAMPLE_RATIO_ATTRIBUTE)
		.and_then(|attribute| match attribute.value {
			Value::F64(ratio) => Some(ratio),
			// `Value` is non-exhaustive.
			_ => None,
		})
}

fn follow(parent: &SpanContext) -> SamplingResult {
	let decision = if parent.is_sampled() {
		SamplingDecision::RecordAndSample
	} else {
		SamplingDecision::Drop
	};
	SamplingResult {
		decision,
		attributes: Vec::new(),
		trace_state: parent.trace_state().clone(),
	}
}
