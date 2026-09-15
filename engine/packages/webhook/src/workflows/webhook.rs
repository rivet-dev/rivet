use std::time::Duration;

use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universaldb::utils::IsolationLevel::*;
use uuid::Uuid;

use crate::{
	errors, keys,
	types::{DeliveryRecord, DeliveryStatus, WebhookConfig, WebhookEvent, WebhookEventType},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub config: WebhookConfig,
}

#[workflow]
pub async fn webhook(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	tracing::debug!(
		namespace_id = %input.namespace_id,
		name = %input.name,
		"starting webhook workflow"
	);

	let namespace_id = input.namespace_id;
	let name = input.name.clone();

	ctx.loope(input.config.clone(), move |ctx, config| {
		let name = name.clone();
		async move {
			match ctx.listen::<Main>().await? {
				Main::Update(sig) => {
					*config = sig.config;
				}
				Main::Trigger(sig) => {
					if !config.subscriptions.contains(&sig.event.event_type) {
						tracing::debug!(
							event_type = sig.event.event_type.as_str(),
							"dropping trigger for unsubscribed event type"
						);
						return Ok(Loop::Continue);
					}

					let delivery_id = ctx.activity(GenerateDeliveryIdInput {}).await?;

					let outcome = deliver_with_retries(
						ctx,
						namespace_id,
						name.clone(),
						delivery_id,
						config.clone(),
						sig.event,
					)
					.await?;

					if let DeliveryOutcome::Destroyed = outcome {
						return Ok(Loop::Break(()));
					}
				}
				Main::Retry(sig) => {
					let record = ctx
						.activity(GetDeliveryInput {
							namespace_id,
							name: name.clone(),
							delivery_id: sig.delivery_id.clone(),
						})
						.await?;

					let Some(record) = record else {
						tracing::warn!(
							delivery_id = %sig.delivery_id,
							"retry requested for unknown delivery"
						);
						return Ok(Loop::Continue);
					};

					if !matches!(record.status, DeliveryStatus::Failed) {
						tracing::warn!(
							delivery_id = %sig.delivery_id,
							status = ?record.status,
							"retry requested for delivery not in a failed state"
						);
						return Ok(Loop::Continue);
					}

					let data = match serde_json::from_str(&record.payload) {
						Ok(data) => data,
						Err(err) => {
							tracing::warn!(
								delivery_id = %sig.delivery_id,
								?err,
								"stored delivery payload is not valid json"
							);
							return Ok(Loop::Continue);
						}
					};

					let outcome = deliver_with_retries(
						ctx,
						namespace_id,
						name.clone(),
						sig.delivery_id,
						config.clone(),
						WebhookEvent {
							event_type: record.event_type,
							subject: record.subject,
							data,
						},
					)
					.await?;

					if let DeliveryOutcome::Destroyed = outcome {
						return Ok(Loop::Break(()));
					}
				}
				Main::Destroy(_) => {
					return Ok(Loop::Break(()));
				}
			}

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

enum DeliveryOutcome {
	Done,
	Destroyed,
}

// Runs (or re-runs, for a manual `Retry`) the full attempt loop for one delivery: records it as
// `Pending`, attempts delivery with exponential backoff up to `MAX_DELIVERY_ATTEMPTS`, and records
// the terminal `Succeeded`/`Failed` outcome. Shared by `Trigger` (a brand new delivery) and
// `Retry` (re-attempting a stored, already-failed delivery), since both just need to run this
// same loop against a `delivery_id` and `payload`.
async fn deliver_with_retries(
	ctx: &mut WorkflowCtx,
	namespace_id: Id,
	name: String,
	delivery_id: String,
	config: WebhookConfig,
	event: WebhookEvent,
) -> Result<DeliveryOutcome> {
	let WebhookEvent {
		event_type,
		subject,
		data,
	} = event;

	let payload = serde_json::to_string(&data).context("event data is not serializable")?;
	// Also yields the delivery's `created_at`, which the CloudEvents envelope needs. On a manual
	// `Retry` this is the original trigger time, preserved by the activity.
	let created_at = ctx
		.activity(RecordDeliveryInput {
			namespace_id,
			name: name.clone(),
			delivery_id: delivery_id.clone(),
			payload: payload.clone(),
			status: DeliveryStatus::Pending,
			attempt_count: 0,
			last_error: None,
			event_type,
			subject: subject.clone(),
		})
		.await?;

	let mut attempt = 0;

	loop {
		let deliver_res = ctx
			.activity(DeliverInput {
				delivery_id: delivery_id.clone(),
				namespace_id,
				name: name.clone(),
				config: config.clone(),
				data: data.clone(),
				event_type,
				subject: subject.clone(),
				created_at,
			})
			.await?;

		match deliver_res.error {
			None => {
				ctx.activity(RecordDeliveryInput {
					namespace_id,
					name: name.clone(),
					delivery_id: delivery_id.clone(),
					payload,
					status: DeliveryStatus::Succeeded,
					attempt_count: attempt + 1,
					last_error: None,
					event_type,
					subject,
				})
				.await?;

				return Ok(DeliveryOutcome::Done);
			}
			Some(error) if is_retryable(&error) && attempt + 1 < MAX_DELIVERY_ATTEMPTS => {
				attempt += 1;

				let destroy_sig = ctx
					.listen_with_timeout::<Destroy>(next_delivery_delay(
						deliver_res.retry_after_ms,
						attempt,
					))
					.await?;

				if destroy_sig.is_some() {
					return Ok(DeliveryOutcome::Destroyed);
				}
			}
			Some(error) => {
				tracing::warn!(?error, attempt, "webhook delivery failed permanently");

				ctx.activity(RecordDeliveryInput {
					namespace_id,
					name: name.clone(),
					delivery_id: delivery_id.clone(),
					payload,
					status: DeliveryStatus::Failed,
					attempt_count: attempt + 1,
					last_error: Some(error.build().to_string()),
					event_type,
					subject,
				})
				.await?;

				return Ok(DeliveryOutcome::Done);
			}
		}
	}
}

// The maximum number of delivery attempts for a single triggered event before giving up to DLQ.
const MAX_DELIVERY_ATTEMPTS: u32 = 5;

const MIN_RETRY_AFTER: Duration = Duration::from_secs(1);
const MAX_RETRY_AFTER: Duration = Duration::from_secs(300);

fn is_retryable(error: &errors::Webhook) -> bool {
	match error {
		errors::Webhook::RequestFailed { .. } => true,
		errors::Webhook::DeliveryFailed { status } => matches!(status, 429 | 500..=599),
		errors::Webhook::Invalid { .. }
		| errors::Webhook::Conflict
		| errors::Webhook::DestinationBlocked { .. }
		| errors::Webhook::NotFound
		| errors::Webhook::DeliveryNotFound
		| errors::Webhook::DeliveryNotRetryable
		| errors::Webhook::EventTypeNotAllowed { .. } => false,
	}
}

// Exponential backoff starting at 5s, doubling each attempt, capped at 5m.
fn delivery_backoff(attempt: u32) -> Duration {
	Duration::from_secs(5u64.saturating_mul(1u64 << attempt.min(6)).min(300))
}

// How long to wait before the next attempt. A destination that told us how long to wait wins over
// our own backoff, using 429's
fn next_delivery_delay(retry_after_ms: Option<u64>, attempt: u32) -> Duration {
	match retry_after_ms {
		Some(ms) => Duration::from_millis(ms).clamp(MIN_RETRY_AFTER, MAX_RETRY_AFTER),
		None => delivery_backoff(attempt),
	}
}

// `Retry-After` is either delta-seconds or an HTTP-date (RFC 9110). An HTTP-date in the past
// yields a zero delay, which the caller clamps.
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
	let raw = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;

	if let Ok(seconds) = raw.trim().parse::<u64>() {
		return Some(seconds.saturating_mul(1_000));
	}

	let deadline = chrono::DateTime::parse_from_rfc2822(raw.trim()).ok()?;
	let delta = deadline.timestamp_millis() - chrono::Utc::now().timestamp_millis();

	Some(delta.max(0) as u64)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateDeliveryIdInput {}

#[activity(GenerateDeliveryId)]
pub async fn generate_delivery_id(
	_ctx: &ActivityCtx,
	_input: &GenerateDeliveryIdInput,
) -> Result<String> {
	Ok(Uuid::new_v4().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverInput {
	pub delivery_id: String,
	pub namespace_id: Id,
	pub name: String,
	pub config: WebhookConfig,
	pub data: serde_json::Value,
	pub event_type: WebhookEventType,
	pub subject: Option<String>,
	pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeliverOutput {
	pub error: Option<errors::Webhook>,
	pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct CloudEvent<'a> {
	id: String,
	source: &'a str,
	specversion: &'static str,
	#[serde(rename = "type")]
	kind: &'static str,
	time: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	subject: Option<&'a str>,
	datacontenttype: &'static str,
	data: &'a serde_json::Value,
}

#[activity(Deliver)]
pub async fn deliver(ctx: &ActivityCtx, input: &DeliverInput) -> Result<DeliverOutput> {
	let parsed_url =
		url::Url::parse(&input.config.url).context("stored webhook url is not parseable")?;

	let policy = rivet_pools::reqwest::outbound_policy(ctx.config()).await?;
	if let Err(reason) = policy.check_url(&parsed_url) {
		return Ok(DeliverOutput {
			error: Some(errors::Webhook::DestinationBlocked {
				reason: reason.to_string(),
			}),
			retry_after_ms: None,
		});
	}

	// The occurrence time, not the transmission time, so every attempt at one delivery carries
	// the same value. Receivers dedupe on `id` plus `source` and would otherwise see the same
	// event reported as having happened at several different times. Follows Cloudevents standards.
	let occurred_at = chrono::DateTime::from_timestamp_millis(input.created_at)
		.context("delivery created_at is not a valid timestamp")?
		.to_rfc3339();

	let event = CloudEvent {
		id: input.delivery_id.clone(),
		source: &format!("rivet:webhook:{}:{}", input.namespace_id, input.name),
		specversion: "1.0",
		kind: input.event_type.as_cloudevents_type(),
		time: occurred_at,
		subject: input.subject.as_deref(),
		datacontenttype: "application/json",
		data: &input.data,
	};

	let client = rivet_pools::reqwest::guarded_client(ctx.config()).await?;
	let mut req = client
		.post(parsed_url)
		.header("Content-Type", "application/cloudevents+json")
		.json(&event);

	for (k, v) in &input.config.headers {
		req = req.header(k, v);
	}

	match req.send().await {
		Ok(res) if res.status().is_success() => Ok(DeliverOutput {
			error: None,
			retry_after_ms: None,
		}),
		Ok(res) => {
			let status = res.status().as_u16();

			Ok(DeliverOutput {
				error: Some(errors::Webhook::DeliveryFailed { status }),
				retry_after_ms: parse_retry_after(res.headers()),
			})
		}
		Err(err) => {
			let err = anyhow::Error::from(err);

			if let Some(reason) = rivet_outbound_guard::block_reason(&err) {
				return Ok(DeliverOutput {
					error: Some(errors::Webhook::DestinationBlocked {
						reason: reason.to_string(),
					}),
					retry_after_ms: None,
				});
			}

			Ok(DeliverOutput {
				error: Some(errors::Webhook::RequestFailed {
					reason: format!("{err:#}"),
				}),
				retry_after_ms: None,
			})
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDeliveryInput {
	pub namespace_id: Id,
	pub name: String,
	pub delivery_id: String,
	pub payload: String,
	pub status: DeliveryStatus,
	pub attempt_count: u32,
	pub last_error: Option<String>,
	pub event_type: WebhookEventType,
	pub subject: Option<String>,
}

#[activity(RecordDelivery)]
pub async fn record_delivery(ctx: &ActivityCtx, input: &RecordDeliveryInput) -> Result<i64> {
	let namespace_id = input.namespace_id;
	let name = input.name.clone();
	let delivery_id = input.delivery_id.clone();
	let payload = input.payload.clone();
	let status = input.status;
	let attempt_count = input.attempt_count;
	let last_error = input.last_error.clone();
	let event_type = input.event_type;
	let subject = input.subject.clone();
	let now = ctx.ts();

	ctx.udb()?
		.txn("webhook_record_delivery", move |tx| {
			let name = name.clone();
			let delivery_id = delivery_id.clone();
			let payload = payload.clone();
			let last_error = last_error.clone();
			let subject = subject.clone();
			async move {
				let tx = tx.with_subspace(namespace::keys::subspace());
				let key = keys::DeliveryKey::new(namespace_id, name, delivery_id);

				let created_at = match tx.read_opt(&key, Serializable).await? {
					Some(existing) => existing.created_at,
					None => now,
				};

				tx.write(
					&key,
					DeliveryRecord {
						payload,
						status,
						attempt_count,
						last_error,
						created_at,
						event_type,
						subject,
					},
				)?;
				Ok(created_at)
			}
		})
		.await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDeliveryInput {
	pub namespace_id: Id,
	pub name: String,
	pub delivery_id: String,
}

#[activity(GetDelivery)]
pub async fn get_delivery(
	ctx: &ActivityCtx,
	input: &GetDeliveryInput,
) -> Result<Option<DeliveryRecord>> {
	let namespace_id = input.namespace_id;
	let name = input.name.clone();
	let delivery_id = input.delivery_id.clone();

	ctx.udb()?
		.txn("webhook_get_delivery", move |tx| {
			let name = name.clone();
			let delivery_id = delivery_id.clone();
			async move {
				let tx = tx.with_subspace(namespace::keys::subspace());
				tx.read_opt(
					&keys::DeliveryKey::new(namespace_id, name, delivery_id),
					Serializable,
				)
				.await
			}
		})
		.await
}

#[signal("webhook_trigger")]
pub struct Trigger {
	pub event: WebhookEvent,
}

#[signal("webhook_update")]
pub struct Update {
	pub config: WebhookConfig,
}

#[signal("webhook_destroy")]
pub struct Destroy {}

#[signal("webhook_retry")]
pub struct Retry {
	pub delivery_id: String,
}

join_signal!(Main {
	Trigger,
	Update,
	Destroy,
	Retry,
});

#[cfg(test)]
#[path = "../../tests/inline/workflows_webhook.rs"]
mod tests;
