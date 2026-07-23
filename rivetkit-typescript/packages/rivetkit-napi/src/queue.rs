use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::{
	ActorContext as CoreActorContext, QueueMessage as CoreQueueMessage, QueueMessageStatus,
	QueueNextBatchOpts, QueueNextOpts, QueueSendOpts, QueueTryNextBatchOpts, QueueTryNextOpts,
	QueueWaitOpts,
};

use crate::cancellation_token::CancellationToken;
use crate::{NapiInvalidArgument, napi_anyhow_error};

#[napi(object)]
pub struct JsQueueNextOptions {
	pub names: Option<Vec<String>>,
	pub timeout_ms: Option<i64>,
}

#[napi(object)]
pub struct JsQueueNextBatchOptions {
	pub names: Option<Vec<String>>,
	pub count: Option<u32>,
	pub timeout_ms: Option<i64>,
}

#[napi(object)]
pub struct JsQueueWaitOptions {
	pub timeout_ms: Option<i64>,
}

#[napi(object)]
pub struct JsQueueSendOptions {
	pub dedupe_key: Option<String>,
	pub delay_ms: Option<i64>,
}

#[napi(object)]
pub struct JsQueueTryNextOptions {
	pub names: Option<Vec<String>>,
}

#[napi(object)]
pub struct JsQueueTryNextBatchOptions {
	pub names: Option<Vec<String>>,
	pub count: Option<u32>,
}

#[napi(object)]
pub struct JsQueueSendReceipt {
	pub id: String,
	pub deduplicated: bool,
}

#[napi(object)]
pub struct JsQueueStatus {
	pub state: String,
	pub attempts: Option<u32>,
	pub created_at_ms: Option<i64>,
	pub available_at_ms: Option<i64>,
	pub started_at_ms: Option<i64>,
	pub completed_at_ms: Option<i64>,
	pub failed_at_ms: Option<i64>,
	pub consumed_at_ms: Option<i64>,
}

#[napi]
pub struct Queue {
	inner: CoreActorContext,
}

#[napi]
pub struct QueueMessage {
	id: String,
	name: String,
	body: Vec<u8>,
	created_at: i64,
	attempts: u32,
	first_failed_at: Option<i64>,
}

impl Queue {
	pub(crate) fn new(inner: CoreActorContext) -> Self {
		Self { inner }
	}
}

impl QueueMessage {
	fn from_core(message: CoreQueueMessage) -> Self {
		tracing::debug!(
			class = "QueueMessage",
			message_id = %message.receipt_id,
			name = %message.name,
			body_bytes = message.body.len(),
			"constructed napi class"
		);
		Self {
			id: message.receipt_id,
			name: message.name.clone(),
			body: message.body.clone(),
			created_at: message.created_at,
			attempts: message.attempts,
			first_failed_at: message.first_failed_at,
		}
	}
}

impl Drop for QueueMessage {
	fn drop(&mut self) {
		tracing::debug!(
			class = "QueueMessage",
			message_id = %self.id,
			name = %self.name,
			"dropped napi class"
		);
	}
}

#[napi]
impl Queue {
	#[napi]
	pub async fn send(
		&self,
		name: String,
		body: Buffer,
		options: Option<JsQueueSendOptions>,
	) -> napi::Result<JsQueueSendReceipt> {
		let options = options.unwrap_or(JsQueueSendOptions {
			dedupe_key: None,
			delay_ms: None,
		});
		self.inner
			.send_with_opts(
				&name,
				body.as_ref(),
				QueueSendOpts {
					dedupe_key: options.dedupe_key,
					delay: timeout_duration(options.delay_ms)?,
				},
			)
			.await
			.map(|receipt| JsQueueSendReceipt {
				id: receipt.id,
				deduplicated: receipt.deduplicated,
			})
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn status(&self, receipt_id: String) -> napi::Result<JsQueueStatus> {
		self.inner
			.queue_status(&receipt_id)
			.await
			.map(queue_status_to_js)
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn next(
		&self,
		options: Option<JsQueueNextOptions>,
		signal: Option<&CancellationToken>,
	) -> napi::Result<Option<QueueMessage>> {
		self.inner
			.next(queue_next_opts(options, signal)?)
			.await
			.map(|message| message.map(QueueMessage::from_core))
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn next_batch(
		&self,
		options: Option<JsQueueNextBatchOptions>,
		signal: Option<&CancellationToken>,
	) -> napi::Result<Vec<QueueMessage>> {
		self.inner
			.next_batch(queue_next_batch_opts(options, signal)?)
			.await
			.map(|messages| messages.into_iter().map(QueueMessage::from_core).collect())
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn wait_for_names(
		&self,
		names: Vec<String>,
		options: Option<JsQueueWaitOptions>,
		signal: Option<&CancellationToken>,
	) -> napi::Result<()> {
		self.inner
			.wait_for_names_available(names, queue_wait_opts(options, signal)?)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn wait_for_names_available(
		&self,
		names: Vec<String>,
		options: Option<JsQueueWaitOptions>,
		signal: Option<&CancellationToken>,
	) -> napi::Result<()> {
		self.inner
			.wait_for_names_available(names, queue_wait_opts(options, signal)?)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn try_next(
		&self,
		options: Option<JsQueueTryNextOptions>,
	) -> napi::Result<Option<QueueMessage>> {
		self.inner
			.try_next(queue_try_next_opts(options))
			.map(|message| message.map(QueueMessage::from_core))
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn try_next_batch(
		&self,
		options: Option<JsQueueTryNextBatchOptions>,
	) -> napi::Result<Vec<QueueMessage>> {
		self.inner
			.try_next_batch(queue_try_next_batch_opts(options))
			.map(|messages| messages.into_iter().map(QueueMessage::from_core).collect())
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub fn max_size(&self) -> u32 {
		self.inner.max_size()
	}

	#[napi]
	pub async fn reset(&self) -> napi::Result<()> {
		self.inner.reset().await.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn inspect_messages(&self) -> napi::Result<Vec<JsQueueInspectMessage>> {
		self.inner
			.inspect_messages()
			.await
			.map(|messages| {
				messages
					.into_iter()
					.map(|m| JsQueueInspectMessage {
						id: u64_to_i64(m.id),
						name: m.name,
						created_at_ms: m.created_at,
					})
					.collect()
			})
			.map_err(napi_anyhow_error)
	}
}

#[napi(object)]
pub struct JsQueueInspectMessage {
	/// Queue message id. Stored as the raw u64 reinterpreted as i64 so JS
	/// sees a plain number; ids are monotonic and fit comfortably in i64.
	pub id: i64,
	pub name: String,
	pub created_at_ms: i64,
}

fn u64_to_i64(value: u64) -> i64 {
	i64::try_from(value).unwrap_or(i64::MAX)
}

#[napi]
impl QueueMessage {
	#[napi]
	pub fn id(&self) -> String {
		self.id.clone()
	}

	#[napi]
	pub fn name(&self) -> String {
		self.name.clone()
	}

	#[napi]
	pub fn body(&self) -> Buffer {
		Buffer::from(self.body.clone())
	}

	#[napi]
	pub fn created_at(&self) -> i64 {
		self.created_at
	}

	#[napi]
	pub fn attempts(&self) -> u32 {
		self.attempts
	}

	#[napi]
	pub fn first_failed_at(&self) -> Option<i64> {
		self.first_failed_at
	}
}

fn queue_next_opts(
	options: Option<JsQueueNextOptions>,
	signal: Option<&CancellationToken>,
) -> napi::Result<QueueNextOpts> {
	let options = options.unwrap_or(JsQueueNextOptions {
		names: None,
		timeout_ms: None,
	});

	Ok(QueueNextOpts {
		names: options.names,
		timeout: timeout_duration(options.timeout_ms)?,
		signal: signal.map(|signal| signal.inner().clone()),
	})
}

fn queue_next_batch_opts(
	options: Option<JsQueueNextBatchOptions>,
	signal: Option<&CancellationToken>,
) -> napi::Result<QueueNextBatchOpts> {
	let options = options.unwrap_or(JsQueueNextBatchOptions {
		names: None,
		count: None,
		timeout_ms: None,
	});

	Ok(QueueNextBatchOpts {
		names: options.names,
		count: options.count.unwrap_or(1),
		timeout: timeout_duration(options.timeout_ms)?,
		signal: signal.map(|signal| signal.inner().clone()),
	})
}

fn queue_wait_opts(
	options: Option<JsQueueWaitOptions>,
	signal: Option<&CancellationToken>,
) -> napi::Result<QueueWaitOpts> {
	let options = options.unwrap_or(JsQueueWaitOptions {
		timeout_ms: None,
	});

	Ok(QueueWaitOpts {
		timeout: timeout_duration(options.timeout_ms)?,
		signal: signal.map(|signal| signal.inner().clone()),
	})
}

fn queue_try_next_opts(options: Option<JsQueueTryNextOptions>) -> QueueTryNextOpts {
	let options = options.unwrap_or(JsQueueTryNextOptions {
		names: None,
	});

	QueueTryNextOpts {
		names: options.names,
	}
}

fn queue_try_next_batch_opts(options: Option<JsQueueTryNextBatchOptions>) -> QueueTryNextBatchOpts {
	let options = options.unwrap_or(JsQueueTryNextBatchOptions {
		names: None,
		count: None,
	});

	QueueTryNextBatchOpts {
		names: options.names,
		count: options.count.unwrap_or(1),
	}
}

fn queue_status_to_js(status: QueueMessageStatus) -> JsQueueStatus {
	let mut output = JsQueueStatus {
		state: "unknown".to_owned(),
		attempts: None,
		created_at_ms: None,
		available_at_ms: None,
		started_at_ms: None,
		completed_at_ms: None,
		failed_at_ms: None,
		consumed_at_ms: None,
	};
	match status {
		QueueMessageStatus::Queued { attempts, created_at } => {
			output.state = "queued".to_owned();
			output.attempts = Some(attempts);
			output.created_at_ms = Some(created_at);
		}
		QueueMessageStatus::Delayed { attempts, created_at, available_at } => {
			output.state = "delayed".to_owned();
			output.attempts = Some(attempts);
			output.created_at_ms = Some(created_at);
			output.available_at_ms = Some(available_at);
		}
		QueueMessageStatus::Processing { attempts, created_at, started_at } => {
			output.state = "processing".to_owned();
			output.attempts = Some(attempts);
			output.created_at_ms = Some(created_at);
			output.started_at_ms = Some(started_at);
		}
		QueueMessageStatus::Retrying { attempts, created_at, available_at } => {
			output.state = "retrying".to_owned();
			output.attempts = Some(attempts);
			output.created_at_ms = Some(created_at);
			output.available_at_ms = Some(available_at);
		}
		QueueMessageStatus::Succeeded { attempts, created_at, completed_at } => {
			output.state = "succeeded".to_owned();
			output.attempts = Some(attempts);
			output.created_at_ms = Some(created_at);
			output.completed_at_ms = Some(completed_at);
		}
		QueueMessageStatus::DeadLettered { attempts, created_at, failed_at } => {
			output.state = "deadLettered".to_owned();
			output.attempts = Some(attempts);
			output.created_at_ms = Some(created_at);
			output.failed_at_ms = Some(failed_at);
		}
		QueueMessageStatus::Consumed { created_at, consumed_at } => {
			output.state = "consumed".to_owned();
			output.created_at_ms = Some(created_at);
			output.consumed_at_ms = Some(consumed_at);
		}
		QueueMessageStatus::Unknown => {}
	}
	output
}

fn timeout_duration(timeout_ms: Option<i64>) -> napi::Result<Option<Duration>> {
	match timeout_ms {
		Some(timeout_ms) if timeout_ms < 0 => Err(napi_anyhow_error(
			NapiInvalidArgument {
				argument: "timeoutMs".to_owned(),
				reason: "must be non-negative".to_owned(),
			}
			.build(),
		)),
		Some(timeout_ms) => Ok(Some(Duration::from_millis(
			u64::try_from(timeout_ms).map_err(|_| {
				napi_anyhow_error(
					NapiInvalidArgument {
						argument: "timeoutMs".to_owned(),
						reason: "exceeds u64 range".to_owned(),
					}
					.build(),
				)
			})?,
		))),
		None => Ok(None),
	}
}
