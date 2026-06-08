use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
	time::Duration,
};

use prometheus::{HistogramOpts, HistogramVec};
use rivet_perf::{perf_abandon, perf_finish, perf_start};
use tracing::{Dispatch, Event, Level, Subscriber, field};
use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};

#[derive(Clone, Debug, Default)]
struct SpanFields(BTreeMap<String, String>);

#[derive(Clone, Debug)]
struct EventRecord {
	level: Level,
	fields: BTreeMap<String, String>,
	span_fields: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
struct RecordingLayer {
	events: Arc<Mutex<Vec<EventRecord>>>,
}

impl RecordingLayer {
	fn events(&self) -> Vec<EventRecord> {
		self.events.lock().unwrap().clone()
	}
}

impl<S> Layer<S> for RecordingLayer
where
	S: Subscriber,
	S: for<'lookup> LookupSpan<'lookup>,
{
	fn on_new_span(
		&self,
		attrs: &tracing::span::Attributes<'_>,
		id: &tracing::span::Id,
		ctx: Context<'_, S>,
	) {
		let mut visitor = FieldVisitor::default();
		attrs.record(&mut visitor);

		if let Some(span) = ctx.span(id) {
			span.extensions_mut().insert(SpanFields(visitor.fields));
		}
	}

	fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
		let mut visitor = FieldVisitor::default();
		event.record(&mut visitor);

		let mut span_fields = BTreeMap::new();
		if let Some(scope) = ctx.event_scope(event) {
			for span in scope.from_root() {
				if let Some(fields) = span.extensions().get::<SpanFields>() {
					span_fields.extend(fields.0.clone());
				}
			}
		}

		self.events.lock().unwrap().push(EventRecord {
			level: *event.metadata().level(),
			fields: visitor.fields,
			span_fields,
		});
	}
}

#[derive(Default)]
struct FieldVisitor {
	fields: BTreeMap<String, String>,
}

impl field::Visit for FieldVisitor {
	fn record_str(&mut self, field: &field::Field, value: &str) {
		self.fields
			.insert(field.name().to_string(), value.to_string());
	}

	fn record_u64(&mut self, field: &field::Field, value: u64) {
		self.fields
			.insert(field.name().to_string(), value.to_string());
	}

	fn record_i64(&mut self, field: &field::Field, value: i64) {
		self.fields
			.insert(field.name().to_string(), value.to_string());
	}

	fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
		self.fields
			.insert(field.name().to_string(), format!("{value:?}"));
	}
}

fn metric(labels: &[&str]) -> &'static HistogramVec {
	Box::leak(Box::new(
		HistogramVec::new(HistogramOpts::new("test_duration_seconds", "test"), labels).unwrap(),
	))
}

fn sample_count(histogram: &HistogramVec, labels: &[&str]) -> u64 {
	histogram.with_label_values(labels).get_sample_count()
}

#[test]
fn perf_start_and_finish_records_metric() {
	let histogram = metric(&["namespace_id"]);

	let m = perf_start!(
		histogram,
		slow_ms = 10_000,
		"test_operation",
		labels: { namespace_id = %"ns-a" },
		fields: { actor_id = %"actor-a" },
	);
	std::thread::sleep(Duration::from_millis(1));
	perf_finish!(m);

	assert_eq!(1, sample_count(histogram, &["ns-a"]));
}

#[test]
fn perf_finish_below_threshold_does_not_warn() {
	let histogram = metric(&["namespace_id"]);
	let layer = RecordingLayer::default();
	let subscriber = tracing_subscriber::registry().with(layer.clone());

	tracing::subscriber::with_default(subscriber, || {
		let m = perf_start!(
			histogram,
			slow_ms = 10_000,
			"test_operation",
			labels: { namespace_id = %"ns-a" },
			fields: { actor_id = %"actor-a" },
		);
		perf_finish!(m);
	});

	assert!(layer.events().is_empty());
}

#[test]
fn perf_finish_above_threshold_warns_with_start_and_end_fields() {
	let histogram = metric(&["namespace_id"]);
	let layer = RecordingLayer::default();
	let subscriber = tracing_subscriber::registry().with(layer.clone());

	tracing::subscriber::with_default(subscriber, || {
		let m = perf_start!(
			histogram,
			slow_ms = 0,
			"test_operation",
			labels: { namespace_id = %"ns-a" },
			fields: { actor_id = %"actor-a" },
		);
		perf_finish!(m, fields: {
			bytes_sent = 123,
			result_code = %"ok",
		});
	});

	let events = layer.events();
	assert_eq!(1, events.len());
	assert_eq!(Level::WARN, events[0].level);
	assert_eq!(
		Some(&"ns-a".to_string()),
		events[0].span_fields.get("namespace_id")
	);
	assert_eq!(
		Some(&"actor-a".to_string()),
		events[0].span_fields.get("actor_id")
	);
	assert_eq!(Some(&"123".to_string()), events[0].fields.get("bytes_sent"));
	assert_eq!(Some(&"ok".to_string()), events[0].fields.get("result_code"));
	assert_eq!(1, sample_count(histogram, &["ns-a"]));
}

#[test]
fn perf_drop_without_finish_warns_and_does_not_record() {
	let histogram = metric(&["namespace_id"]);
	let layer = RecordingLayer::default();
	let subscriber = tracing_subscriber::registry().with(layer.clone());

	tracing::subscriber::with_default(subscriber, || {
		let _m = perf_start!(
			histogram,
			slow_ms = 10_000,
			"test_operation",
			labels: { namespace_id = %"ns-a" },
			fields: { actor_id = %"actor-a" },
		);
	});

	let events = layer.events();
	assert_eq!(1, events.len());
	assert_eq!(Level::WARN, events[0].level);
	assert_eq!(
		Some(&"test_operation".to_string()),
		events[0].fields.get("name")
	);
	assert_eq!(
		Some(&"ns-a".to_string()),
		events[0].span_fields.get("namespace_id")
	);
	assert_eq!(0, sample_count(histogram, &["ns-a"]));
}

#[test]
fn perf_abandon_silent_and_does_not_record() {
	let histogram = metric(&["namespace_id"]);
	let layer = RecordingLayer::default();
	let subscriber = tracing_subscriber::registry().with(layer.clone());

	tracing::subscriber::with_default(subscriber, || {
		let m = perf_start!(
			histogram,
			slow_ms = 0,
			"test_operation",
			labels: { namespace_id = %"ns-a" },
			fields: { actor_id = %"actor-a" },
		);
		perf_abandon!(m);
	});

	assert!(layer.events().is_empty());
	assert_eq!(0, sample_count(histogram, &["ns-a"]));
}

#[tokio::test]
async fn perf_async_holds_across_await() {
	let histogram = metric(&["namespace_id"]);
	let layer = RecordingLayer::default();
	let dispatch = Dispatch::new(tracing_subscriber::registry().with(layer.clone()));

	let m = tracing::dispatcher::with_default(&dispatch, || {
		perf_start!(
			histogram,
			slow_ms = 0,
			"test_operation",
			labels: { namespace_id = %"ns-a" },
			fields: { actor_id = %"actor-a" },
		)
	});

	tokio::task::yield_now().await;

	tracing::dispatcher::with_default(&dispatch, || {
		perf_finish!(m, fields: { result_code = %"ok" });
	});

	let events = layer.events();
	assert_eq!(1, events.len());
	assert_eq!(
		Some(&"ns-a".to_string()),
		events[0].span_fields.get("namespace_id")
	);
	assert_eq!(
		Some(&"actor-a".to_string()),
		events[0].span_fields.get("actor_id")
	);
	assert_eq!(1, sample_count(histogram, &["ns-a"]));
}

#[test]
#[should_panic(expected = "PerfMeasure label order must match HistogramVec registration")]
fn perf_label_order_must_match_registration() {
	let histogram = metric(&["a", "b"]);

	let m = perf_start!(
		histogram,
		slow_ms = 10_000,
		"test_operation",
		labels: {
			b = %"b-value",
			a = %"a-value",
		},
		fields: {},
	);
	perf_finish!(m);
}
