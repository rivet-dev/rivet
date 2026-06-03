mod common;

use anyhow::{Result, anyhow};
use gas::prelude::*;
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};
use tracing::{
	Event, Level, Subscriber,
	field::{Field, Visit},
};
use tracing_subscriber::{Layer, Registry, layer::Context, prelude::*};

#[tokio::test]
async fn hash_scan_circuit_breaker_warn_log_includes_structured_fields() -> Result<()> {
	let capture = CaptureLayer::default();
	let events = Arc::clone(&capture.events);
	let subscriber = Registry::default().with(capture);
	tracing::subscriber::set_global_default(subscriber)
		.map_err(|_| anyhow!("failed to install capture subscriber"))?;

	let test_deps = common::setup_deps().await?;
	let namespace_id = Id::new_v1(test_deps.config().dc_label());
	let pool_name = common::unique_pool_name("hash-scan-breaker-log");

	for i in 1..=8 {
		common::write_hash_envoy(
			&test_deps,
			namespace_id,
			&pool_name,
			&format!("stale-{i}"),
			common::stale_ping_ts(),
			vec![common::hash_pos(i)],
			Some(0),
		)
		.await?;
	}

	let (allocation, _) = common::allocate_hash(
		&test_deps,
		namespace_id,
		&pool_name,
		1,
		8,
		vec![common::hash_pos(0)],
		0,
	)
	.await?;
	assert_eq!(allocation, None);

	let events = events
		.lock()
		.map_err(|_| anyhow!("capture lock poisoned"))?;
	let event = events
		.iter()
		.find(|event| {
			event
				.fields
				.get("message")
				.is_some_and(|message| message.contains("exhausted shared max_scan budget"))
		})
		.ok_or_else(|| anyhow!("expected scan circuit breaker warn log"))?;

	for field in ["namespace_id", "pool_name", "version", "stale_count"] {
		assert!(
			event.fields.contains_key(field),
			"warn log should include {field}"
		);
	}
	assert_eq!(
		event.fields.get("pool_name").map(String::as_str),
		Some(pool_name.as_str())
	);
	assert_eq!(event.fields.get("version").map(String::as_str), Some("7"));
	assert_eq!(
		event.fields.get("stale_count").map(String::as_str),
		Some("8")
	);

	Ok(())
}

#[derive(Debug, Clone)]
struct RecordedEvent {
	fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct CaptureLayer {
	events: Arc<Mutex<Vec<RecordedEvent>>>,
}

impl<S> Layer<S> for CaptureLayer
where
	S: Subscriber,
{
	fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
		if *event.metadata().level() != Level::WARN {
			return;
		}

		let mut visitor = FieldVisitor {
			fields: BTreeMap::new(),
		};
		event.record(&mut visitor);
		if let Ok(mut events) = self.events.lock() {
			events.push(RecordedEvent {
				fields: visitor.fields,
			});
		}
	}
}

struct FieldVisitor {
	fields: BTreeMap<String, String>,
}

impl FieldVisitor {
	fn record(&mut self, field: &Field, value: String) {
		self.fields.insert(field.name().to_string(), value);
	}
}

impl Visit for FieldVisitor {
	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		self.record(field, format!("{value:?}"));
	}

	fn record_str(&mut self, field: &Field, value: &str) {
		self.record(field, value.to_string());
	}

	fn record_i64(&mut self, field: &Field, value: i64) {
		self.record(field, value.to_string());
	}

	fn record_u64(&mut self, field: &Field, value: u64) {
		self.record(field, value.to_string());
	}
}
