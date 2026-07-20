use std::collections::BTreeSet;
use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::time::{Instant, SystemTime, UNIX_EPOCH, sleep};

use anyhow::{Context, Result};
use rivet_error::RivetError;
use rivetkit_actor_persist::{generated::v4 as persist_v4, versioned as persist_versioned};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::{Builder, Handle};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::internal_storage;
use crate::actor::messages::ActorEvent;
use crate::actor::persist::{
	decode_latest_with_embedded_version, encode_latest_with_embedded_version,
};
use crate::actor::task_types::UserTaskKind;
use crate::actor::work_registry::ActorWorkKind;
#[cfg(target_arch = "wasm32")]
use crate::error::ActorRuntime;

#[derive(Clone, Debug, Default)]
pub struct QueueNextOpts {
	pub names: Option<Vec<String>>,
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
}

#[derive(Clone, Debug, Default)]
pub struct QueueWaitOpts {
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
}

#[derive(Clone, Debug)]
pub struct QueueNextBatchOpts {
	pub names: Option<Vec<String>>,
	pub count: u32,
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
}

impl Default for QueueNextBatchOpts {
	fn default() -> Self {
		Self {
			names: None,
			count: 1,
			timeout: None,
			signal: None,
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct QueueTryNextOpts {
	pub names: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct QueueTryNextBatchOpts {
	pub names: Option<Vec<String>>,
	pub count: u32,
}

impl Default for QueueTryNextBatchOpts {
	fn default() -> Self {
		Self {
			 names: None,
			count: 1,
		}
	}
}

pub(super) type QueueWaitActivityCallback = Arc<dyn Fn() + Send + Sync>;
pub(super) type QueueInspectorUpdateCallback = Arc<dyn Fn(u32) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct QueueMessage {
	pub id: u64,
	pub receipt_id: String,
	pub name: String,
	pub body: Vec<u8>,
	pub created_at: i64,
	pub attempts: u32,
	pub first_failed_at: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct QueueSendOpts {
	pub dedupe_key: Option<String>,
	pub delay: Option<Duration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueSendReceipt {
	pub id: String,
	pub deduplicated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueMessageStatus {
	Queued {
		attempts: u32,
		created_at: i64,
	},
	Delayed {
		attempts: u32,
		created_at: i64,
		available_at: i64,
	},
	Processing {
		attempts: u32,
		created_at: i64,
		started_at: i64,
	},
	Retrying {
		attempts: u32,
		created_at: i64,
		available_at: i64,
	},
	Succeeded {
		attempts: u32,
		created_at: i64,
		completed_at: i64,
	},
	DeadLettered {
		attempts: u32,
		created_at: i64,
		failed_at: i64,
	},
	Consumed {
		created_at: i64,
		consumed_at: i64,
	},
	Unknown,
}

pub(crate) type QueueMetadata = persist_v4::QueueMetadata;
pub(crate) type PersistedQueueMessage = persist_v4::QueueMessage;

#[cfg(test)]
pub(crate) fn encode_queue_metadata(metadata: &QueueMetadata) -> Result<Vec<u8>> {
	encode_latest_with_embedded_version::<persist_versioned::QueueMetadata>(
		metadata.clone(),
		rivetkit_actor_persist::CURRENT_VERSION,
		"queue metadata",
	)
}

pub(crate) fn decode_queue_metadata(payload: &[u8]) -> Result<QueueMetadata> {
	let metadata = decode_latest_with_embedded_version::<persist_versioned::QueueMetadata>(
		payload,
		"queue metadata",
	)?;
	Ok(metadata)
}

pub(crate) fn encode_queue_message(message: &PersistedQueueMessage) -> Result<Vec<u8>> {
	encode_latest_with_embedded_version::<persist_versioned::QueueMessage>(
		message.clone(),
		rivetkit_actor_persist::CURRENT_VERSION,
		"queue message",
	)
}

pub(crate) fn decode_queue_message(payload: &[u8]) -> Result<PersistedQueueMessage> {
	let message = decode_latest_with_embedded_version::<persist_versioned::QueueMessage>(
		payload,
		"queue message",
	)?;
	Ok(message)
}

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"full",
	"Queue is full",
	"Queue is full. Limit is {limit} messages."
)]
struct QueueFull {
	limit: u32,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"message_too_large",
	"Queue message is too large",
	"Queue message too large ({size} bytes). Limit is {limit} bytes."
)]
struct QueueMessageTooLarge {
	size: usize,
	limit: u32,
}

#[derive(RivetError)]
#[error("actor", "aborted", "Actor aborted")]
struct QueueActorAborted;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"timed_out",
	"Queue wait timed out",
	"Queue wait timed out after {timeout_ms} ms."
)]
struct QueueWaitTimedOut {
	timeout_ms: u64,
}

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"automatic_consumer",
	"Queue is consumed by onMessage",
	"Queue '{name}' is consumed automatically by onMessage and cannot be read with the raw next API."
)]
struct QueueAutomaticConsumer {
	name: String,
}

impl ActorContext {
	pub(crate) fn start_queue_on_message_loop(&self, generation: u32) {
		let definitions = self
			.config()
			.queues
			.into_iter()
			.filter(|definition| definition.on_message)
			.collect::<Vec<_>>();
		if definitions.is_empty() {
			return;
		}
		let ctx = self.clone();
		self.spawn_work(ActorWorkKind::RegisteredTask, async move {
			let abort = ctx.0.queue_abort_signal.lock().clone();
			loop {
				match ctx
					.run_queue_on_message_loop(definitions.clone(), generation)
					.await
				{
					Ok(()) => return,
					Err(error) => {
						tracing::error!(?error, "queue onMessage loop failed; retrying");
					}
				}
				tokio::select! {
					_ = sleep(Duration::from_millis(100)) => {},
					_ = abort.cancelled() => return,
				}
			}
		});
	}

	async fn run_queue_on_message_loop(
		&self,
		definitions: Vec<crate::actor::config::QueueDefinition>,
		generation: u32,
	) -> Result<()> {
		self.ensure_initialized().await?;
		internal_storage::recover_queue_leases(self.sql(), current_timestamp_ms()?).await?;
		self.sync_queue_alarm_state().await?;
		let abort = self.0.queue_abort_signal.lock().clone();
		let names = definitions
			.iter()
			.map(|definition| definition.name.clone())
			.collect::<Vec<_>>();
		let dead_letter_names = definitions
			.iter()
			.filter(|definition| definition.on_dead_letter)
			.map(|definition| definition.name.clone())
			.collect::<Vec<_>>();
		let mut next_message_index = 0;
		let mut next_dead_letter_index = 0;
		loop {
			let notified = self.0.queue_notify.notified();
			let mut processed = false;
			for offset in 0..definitions.len() {
				let index = (next_dead_letter_index + offset) % definitions.len();
				let definition = &definitions[index];
				if !definition.on_dead_letter {
					continue;
				}
				let row = internal_storage::claim_dead_letter_notification(
					self.sql(),
					&definition.name,
					current_timestamp_ms()?,
					generation,
				)
				.await?;
				let Some(row) = row else {
					continue;
				};
				processed = true;
				next_dead_letter_index = (index + 1) % definitions.len();
				self.dispatch_queue_dead_letter(row, definition, generation)
					.await?;
				break;
			}
			if !processed {
				for offset in 0..definitions.len() {
					let index = (next_message_index + offset) % definitions.len();
					let definition = &definitions[index];
					let now = current_timestamp_ms()?;
					let row = {
						let _receive_guard = self.0.queue_receive_lock.lock().await;
						let transition = internal_storage::dead_letter_exhausted_queue_message(
							self.sql(),
							&definition.name,
							definition.max_attempts,
							now,
						)
						.await?;
						self.record_dead_letter_purge(transition.purged);
						if transition.updated {
							self.decrement_queue_size(1).await;
							self.0.queue_notify.notify_waiters();
							processed = true;
							next_message_index = (index + 1) % definitions.len();
							None
						} else {
							internal_storage::claim_queue_message(
								self.sql(),
								&definition.name,
								now,
								generation,
							)
							.await?
						}
					};
					let Some(row) = row else {
						if processed {
							self.sync_queue_alarm_state().await?;
							break;
						}
						continue;
					};
					processed = true;
					next_message_index = (index + 1) % definitions.len();
					self.dispatch_queue_message(row, definition, generation).await?;
					break;
				}
			}
			if processed {
				continue;
			}

			let next_due = internal_storage::next_queue_due(
				self.sql(),
				&names,
				&dead_letter_names,
			)
			.await?;
			let due_sleep = async {
				if let Some(next_due) = next_due {
					let now = current_timestamp_ms().unwrap_or(next_due);
					let delay_ms = u64::try_from(next_due.saturating_sub(now)).unwrap_or(0);
					sleep(Duration::from_millis(delay_ms)).await;
				} else {
					pending::<()>().await;
				}
			};
			tokio::select! {
				_ = notified => {},
				_ = due_sleep => {},
				_ = abort.cancelled() => return Ok(()),
			}
		}
	}

	async fn dispatch_queue_message(
		&self,
		row: internal_storage::QueueMessageRow,
		definition: &crate::actor::config::QueueDefinition,
		generation: u32,
	) -> Result<()> {
		let message = QueueMessage {
			id: row.id,
			receipt_id: row.receipt_id.clone(),
			name: row.message.name.clone(),
			body: row.message.body.clone(),
			created_at: row.message.created_at,
			attempts: row.attempt_count,
			first_failed_at: row.first_failed_at,
		};
		self.sync_queue_alarm_state().await?;
		let signal = CancellationToken::new();
		let actor_abort = self.0.queue_abort_signal.lock().clone();
		let (reply_tx, reply_rx) = oneshot::channel();
		let dispatch_result = self.try_send_actor_event(
			ActorEvent::QueueMessage {
				message: message.clone(),
				signal: signal.clone(),
				reply: reply_tx.into(),
			},
			"queue_on_message",
		);
		let result = match dispatch_result {
			Ok(()) => {
				self.track_work(ActorWorkKind::InternalKeepAwake, async {
					tokio::select! {
						result = reply_rx => match result {
							Ok(result) => result,
							Err(error) => Err(anyhow::Error::new(error).context("queue onMessage reply dropped")),
						},
						_ = sleep(definition.timeout) => {
							signal.cancel();
							Err(anyhow::anyhow!("queue onMessage timed out"))
						},
						_ = actor_abort.cancelled() => {
							signal.cancel();
							Err(QueueActorAborted.build())
						},
					}
				})
				.await
			}
			Err(error) => {
				signal.cancel();
				Err(error.context("dispatch queue onMessage"))
			}
		};

		let now = current_timestamp_ms()?;
		if result.is_ok() {
			if internal_storage::ack_queue_message(self.sql(), &row, generation, now).await? {
				self.decrement_queue_size(1).await;
			}
			self.sync_queue_alarm_state().await?;
			return Ok(());
		}

		let dead_letter = row.attempt_count >= definition.max_attempts;
		let available_at = (!dead_letter).then(|| {
			now.saturating_add(
				i64::try_from(queue_backoff(definition, row.attempt_count).as_millis())
					.unwrap_or(i64::MAX),
			)
		});
		let transition = internal_storage::nack_queue_message(
			self.sql(),
			&row,
			generation,
			now,
			available_at,
			dead_letter,
		)
		.await?;
		self.record_dead_letter_purge(transition.purged);
		if transition.updated {
			if dead_letter {
				self.decrement_queue_size(1).await;
			}
			self.0.queue_notify.notify_waiters();
		}
		self.sync_queue_alarm_state().await?;
		Ok(())
	}

	async fn dispatch_queue_dead_letter(
		&self,
		row: internal_storage::QueueMessageRow,
		definition: &crate::actor::config::QueueDefinition,
		generation: u32,
	) -> Result<()> {
		let message = QueueMessage {
			id: row.id,
			receipt_id: row.receipt_id.clone(),
			name: row.message.name.clone(),
			body: row.message.body.clone(),
			created_at: row.message.created_at,
			attempts: row.attempt_count,
			first_failed_at: row.first_failed_at,
		};
		let signal = CancellationToken::new();
		let actor_abort = self.0.queue_abort_signal.lock().clone();
		let (reply_tx, reply_rx) = oneshot::channel();
		if let Err(error) = self.try_send_actor_event(
			ActorEvent::QueueDeadLetter {
				message,
				signal: signal.clone(),
				reply: reply_tx.into(),
			},
			"queue_on_dead_letter",
		) {
			tracing::error!(?error, "failed to dispatch queue onDeadLetter callback");
		}
		let succeeded = tokio::select! {
			result = reply_rx => matches!(result, Ok(Ok(()))),
			_ = sleep(definition.timeout) => {
				signal.cancel();
				tracing::error!("queue onDeadLetter callback timed out");
				false
			},
			_ = actor_abort.cancelled() => {
				signal.cancel();
				false
			},
		};
		let now = current_timestamp_ms()?;
		let available_at = (!succeeded).then(|| {
			now.saturating_add(
				i64::try_from(
					queue_backoff(definition, row.dlq_notify_attempt_count).as_millis(),
				)
				.unwrap_or(i64::MAX),
			)
		});
		if internal_storage::finish_dead_letter_notification(
			self.sql(),
			row.id,
			generation,
			now,
			succeeded,
			available_at,
		)
		.await?
		{
			self.0.queue_notify.notify_waiters();
		}
		self.sync_queue_alarm_state().await?;
		Ok(())
	}
	pub async fn send(&self, name: &str, body: &[u8]) -> Result<QueueSendReceipt> {
		self.send_with_opts(name, body, QueueSendOpts::default()).await
	}

	pub async fn send_with_opts(
		&self,
		name: &str,
		body: &[u8],
		opts: QueueSendOpts,
	) -> Result<QueueSendReceipt> {
		self.enqueue_message(name, body, opts).await
	}

	pub async fn queue_status(&self, receipt_id: &str) -> Result<QueueMessageStatus> {
		self.ensure_initialized().await?;
		let now = current_timestamp_ms()?;
		internal_storage::purge_queue_receipts(
			self.sql(),
			now.saturating_sub(24 * 60 * 60 * 1000),
		)
		.await?;
		let Some(row) = internal_storage::load_queue_status(self.sql(), receipt_id, now).await? else {
			return Ok(QueueMessageStatus::Unknown);
		};
		Ok(match row.state.as_str() {
			"queued" => QueueMessageStatus::Queued {
				attempts: row.attempt_count,
				created_at: row.created_at,
			},
			"delayed" => QueueMessageStatus::Delayed {
				attempts: row.attempt_count,
				created_at: row.created_at,
				available_at: row.available_at.unwrap_or(now),
			},
			"processing" => QueueMessageStatus::Processing {
				attempts: row.attempt_count,
				created_at: row.created_at,
				started_at: row.in_flight_at.unwrap_or(now),
			},
			"retrying" => QueueMessageStatus::Retrying {
				attempts: row.attempt_count,
				created_at: row.created_at,
				available_at: row.available_at.unwrap_or(now),
			},
			"succeeded" => QueueMessageStatus::Succeeded {
				attempts: row.attempt_count,
				created_at: row.created_at,
				completed_at: row.terminal_at.unwrap_or(now),
			},
			"deadLettered" => QueueMessageStatus::DeadLettered {
				attempts: row.attempt_count,
				created_at: row.created_at,
				failed_at: row.dead_at.unwrap_or(now),
			},
			"consumed" => QueueMessageStatus::Consumed {
				created_at: row.created_at,
				consumed_at: row.terminal_at.unwrap_or(now),
			},
			_ => QueueMessageStatus::Unknown,
		})
	}

	async fn enqueue_message(
		&self,
		name: &str,
		body: &[u8],
		opts: QueueSendOpts,
	) -> Result<QueueSendReceipt> {
		self.ensure_initialized().await?;

		let created_at = current_timestamp_ms()?;
		let available_at = opts
			.delay
			.map(|delay| {
				i64::try_from(delay.as_millis())
					.context("queue delay exceeds supported range")
					.map(|delay_ms| created_at.saturating_add(delay_ms))
			})
			.transpose()?;
		let persisted = PersistedQueueMessage {
			name: name.to_owned(),
			body: body.to_vec(),
			created_at,
			failure_count: None,
			available_at: None,
			in_flight: None,
			in_flight_at: None,
		};
		let encoded_message = encode_queue_message(&persisted).context("encode queue message")?;

		let config = self.config();
		if encoded_message.len() > config.max_queue_message_size as usize {
			return Err(QueueMessageTooLarge {
				size: encoded_message.len(),
				limit: config.max_queue_message_size,
			}
			.build());
		}

		let mut metadata = self.0.queue_metadata.lock().await;
		if let Some(dedupe_key) = opts.dedupe_key.as_deref() {
			let oldest_accepted_at = created_at.saturating_sub(5 * 60 * 1000);
			if let Some(receipt_id) = internal_storage::find_queue_dedupe(
				self.sql(),
				name,
				dedupe_key,
				oldest_accepted_at,
			)
			.await?
			{
				return Ok(QueueSendReceipt {
					id: receipt_id,
					deduplicated: true,
				});
			}
		}
		if metadata.size >= config.max_queue_size {
			return Err(QueueFull {
				limit: config.max_queue_size,
			}
			.build());
		}

		let id = if metadata.next_id == 0 {
			1
		} else {
			metadata.next_id
		};
		metadata.next_id = id.saturating_add(1);
		metadata.size = metadata.size.saturating_add(1);
		let receipt_id = uuid::Uuid::new_v4().to_string();
		let persist_result = internal_storage::persist_queue_message(
			self.sql(),
			id,
			metadata.next_id,
			&receipt_id,
			opts.dedupe_key.as_deref(),
			created_at,
			available_at,
			&persisted,
		)
		.await;

		if let Err(error) = persist_result {
			metadata.next_id = id;
			metadata.size = metadata.size.saturating_sub(1);
			return Err(error).context("persist queue message");
		}

		let queue_size = metadata.size;
		drop(metadata);
		self.0.metrics.add_queue_messages_sent(1);
		self.0
			.metrics
			.set_queue_depth(self.0.queue_metadata.lock().await.size);
		self.notify_inspector_update(queue_size);
		self.0.queue_notify.notify_waiters();
		self.sync_queue_alarm_state().await?;

		Ok(QueueSendReceipt {
			id: receipt_id,
			deduplicated: false,
		})
	}

	pub async fn next(&self, opts: QueueNextOpts) -> Result<Option<QueueMessage>> {
		let mut messages = self
			.next_batch(QueueNextBatchOpts {
				names: opts.names,
				count: 1,
				timeout: opts.timeout,
				signal: opts.signal,
			})
			.await?;
		Ok(messages.pop())
	}

	pub async fn next_batch(&self, opts: QueueNextBatchOpts) -> Result<Vec<QueueMessage>> {
		self.ensure_initialized().await?;

		let count = opts.count.max(1);
		let deadline = opts.timeout.map(|timeout| Instant::now() + timeout);
		let names = self.raw_queue_names(opts.names)?;

		loop {
			let messages = self.try_receive_batch(names.as_ref(), count).await?;
			if !messages.is_empty() {
				return Ok(messages);
			}

			let remaining_timeout =
				deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
			if matches!(remaining_timeout, Some(timeout) if timeout.is_zero()) {
				return Ok(Vec::new());
			}

			let wait_guard = ActiveQueueWaitGuard::new(self);
			let result = self
				.wait_for_message(remaining_timeout, opts.signal.as_ref())
				.await;
			drop(wait_guard);

			match result {
				WaitOutcome::Notified => continue,
				WaitOutcome::TimedOut => return Ok(Vec::new()),
				WaitOutcome::Aborted => return Err(QueueActorAborted.build()),
			}
		}
	}

	pub async fn wait_for_names(
		&self,
		names: Vec<String>,
		opts: QueueWaitOpts,
	) -> Result<QueueMessage> {
		self.ensure_initialized().await?;

		let deadline = opts.timeout.map(|timeout| Instant::now() + timeout);
		let names = self.raw_queue_names(Some(names))?;

		loop {
			if let Some(message) = self
				.try_receive_batch(names.as_ref(), 1)
				.await?
				.into_iter()
				.next()
			{
				return Ok(message);
			}

			let remaining_timeout =
				deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
			if let Some(timeout) = remaining_timeout
				&& timeout.is_zero()
			{
				return Err(QueueWaitTimedOut {
					timeout_ms: opts.timeout.map(duration_ms).unwrap_or(0),
				}
				.build());
			}

			let wait_guard = ActiveQueueWaitGuard::new(self);
			let result = self
				.wait_for_message(remaining_timeout, opts.signal.as_ref())
				.await;
			drop(wait_guard);

			match result {
				WaitOutcome::Notified => continue,
				WaitOutcome::TimedOut => {
					return Err(QueueWaitTimedOut {
						timeout_ms: opts.timeout.map(duration_ms).unwrap_or(0),
					}
					.build());
				}
				WaitOutcome::Aborted => return Err(QueueActorAborted.build()),
			}
		}
	}

	pub async fn wait_for_names_available(
		&self,
		names: Vec<String>,
		opts: QueueWaitOpts,
	) -> Result<()> {
		self.ensure_initialized().await?;

		let deadline = opts.timeout.map(|timeout| Instant::now() + timeout);
		let names = self.raw_queue_names(Some(names))?;

		loop {
			if !internal_storage::load_available_queue_messages(
				self.sql(),
				names.as_ref(),
				1,
				current_timestamp_ms()?,
			)
			.await?
			.is_empty()
			{
				return Ok(());
			}

			let remaining_timeout =
				deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
			if let Some(timeout) = remaining_timeout
				&& timeout.is_zero()
			{
				return Err(QueueWaitTimedOut {
					timeout_ms: opts.timeout.map(duration_ms).unwrap_or(0),
				}
				.build());
			}

			let wait_guard = ActiveQueueWaitGuard::new(self);
			let result = self
				.wait_for_message(remaining_timeout, opts.signal.as_ref())
				.await;
			drop(wait_guard);

			match result {
				WaitOutcome::Notified => continue,
				WaitOutcome::TimedOut => {
					return Err(QueueWaitTimedOut {
						timeout_ms: opts.timeout.map(duration_ms).unwrap_or(0),
					}
					.build());
				}
				WaitOutcome::Aborted => return Err(QueueActorAborted.build()),
			}
		}
	}

	pub fn try_next(&self, opts: QueueTryNextOpts) -> Result<Option<QueueMessage>> {
		let mut messages = self.try_next_batch(QueueTryNextBatchOpts {
			names: opts.names,
			count: 1,
		})?;
		Ok(messages.pop())
	}

	pub fn try_next_batch(&self, opts: QueueTryNextBatchOpts) -> Result<Vec<QueueMessage>> {
		self.block_on(async {
			self.ensure_initialized().await?;
			let names = self.raw_queue_names(opts.names)?;
			self.try_receive_batch(names.as_ref(), opts.count.max(1))
				.await
		})
	}

	fn raw_queue_names(&self, names: Option<Vec<String>>) -> Result<Option<BTreeSet<String>>> {
		let definitions = self.config().queues;
		if definitions.is_empty() {
			return Ok(normalize_names(names));
		}

		let automatic = definitions
			.iter()
			.filter(|definition| definition.on_message)
			.map(|definition| definition.name.as_str())
			.collect::<BTreeSet<_>>();
		if let Some(requested) = normalize_names(names) {
			if let Some(name) = requested.iter().find(|name| automatic.contains(name.as_str())) {
				return Err(QueueAutomaticConsumer { name: name.clone() }.build());
			}
			return Ok(Some(requested));
		}

		Ok(Some(
			definitions
				.into_iter()
				.filter(|definition| !definition.on_message)
				.map(|definition| definition.name)
				.collect(),
		))
	}

	pub async fn inspect_messages(&self) -> Result<Vec<QueueMessage>> {
		self.ensure_initialized().await?;
		self.list_messages().await
	}

	pub fn max_size(&self) -> u32 {
		self.config().max_queue_size
	}

	async fn sync_queue_alarm_state(&self) -> Result<()> {
		let definitions = self.config().queues;
		let names = definitions
			.iter()
			.map(|definition| definition.name.clone())
			.collect::<Vec<_>>();
		let dead_letter_names = definitions
			.iter()
			.filter(|definition| definition.on_dead_letter)
			.map(|definition| definition.name.clone())
			.collect::<Vec<_>>();
		let now = current_timestamp_ms()?;
		let next_due = internal_storage::next_queue_due(
			self.sql(),
			&names,
			&dead_letter_names,
		)
		.await?
		.filter(|timestamp| *timestamp > now);
		self.update_queue_alarm(next_due).await;
		Ok(())
	}

	/// Removes all messages from the queue and resets the size counter.
	pub async fn reset(&self) -> Result<()> {
		self.ensure_initialized().await?;

		// Serialize against receivers before touching metadata. Lock order matches
		// try_receive_batch (receive lock then metadata) so there is no deadlock.
		let _receive_guard = self.0.queue_receive_lock.lock().await;

		let mut metadata = self.0.queue_metadata.lock().await;

		internal_storage::reset_queue(self.sql())
			.await
			.context("delete all sqlite queue messages")?;

		metadata.size = 0;

		drop(metadata);

		self.0.metrics.set_queue_depth(0);
		self.notify_inspector_update(0);
		self.0.queue_notify.notify_waiters();
		self.sync_queue_alarm_state().await?;

		Ok(())
	}

	pub(crate) fn configure_queue(&self, config: ActorConfig) {
		*self.0.queue_config.lock() = config;
	}

	pub(crate) fn set_wait_activity_callback(&self, callback: Option<Arc<dyn Fn() + Send + Sync>>) {
		*self.0.queue_wait_activity_callback.lock() = callback;
	}

	pub(crate) fn set_inspector_update_callback(
		&self,
		callback: Option<Arc<dyn Fn(u32) + Send + Sync>>,
	) {
		*self.0.queue_inspector_update_callback.lock() = callback;
	}

	async fn ensure_initialized(&self) -> Result<()> {
		self.0
			.queue_initialize
			.get_or_try_init(|| async {
				let metadata = internal_storage::load_queue_metadata(self.sql())
					.await
					.context("load queue metadata from sqlite")?;
				let mut state = self.0.queue_metadata.lock().await;
				*state = metadata;
				self.0.metrics.set_queue_depth(state.size);
				Ok(())
			})
			.await
			.map(|_| ())
	}

	pub(crate) async fn refresh_queue_metadata(&self) -> Result<()> {
		let metadata = internal_storage::load_queue_metadata(self.sql())
			.await
			.context("refresh queue metadata from sqlite")?;
		let queue_size = metadata.size;
		*self.0.queue_metadata.lock().await = metadata;
		self.0.metrics.set_queue_depth(queue_size);
		self.0.queue_initialize.get_or_init(|| async {}).await;
		Ok(())
	}

	async fn try_receive_batch(
		&self,
		names: Option<&BTreeSet<String>>,
		count: u32,
	) -> Result<Vec<QueueMessage>> {
		let _receive_guard = self.0.queue_receive_lock.lock().await;

		let selected = internal_storage::load_available_queue_messages(
			self.sql(),
			names,
			count,
			current_timestamp_ms()?,
		)
		.await?
		.into_iter()
		.map(|row| QueueMessage {
			id: row.id,
			receipt_id: row.receipt_id,
			name: row.message.name,
			body: row.message.body,
			created_at: row.message.created_at,
			attempts: row.attempt_count,
			first_failed_at: row.first_failed_at,
		})
		.collect::<Vec<_>>();

		if selected.is_empty() {
			return Ok(Vec::new());
		}

		internal_storage::persist_consumed_queue_messages(
			self.sql(),
			&selected
				.iter()
				.map(|message| {
					(
						message.id,
						message.receipt_id.clone(),
						message.name.clone(),
						message.created_at,
						message.attempts,
					)
				})
				.collect::<Vec<_>>(),
			current_timestamp_ms()?,
		)
		.await?;
		self.decrement_queue_size(selected.len()).await;
		self.sync_queue_alarm_state().await?;
		self.0
			.metrics
			.add_queue_messages_received(selected.len().try_into().unwrap_or(u64::MAX));

		Ok(selected)
	}

	async fn list_messages(&self) -> Result<Vec<QueueMessage>> {
		let now = current_timestamp_ms()?;
		let messages: Vec<QueueMessage> = internal_storage::load_queue_messages(self.sql())
			.await
			.context("list sqlite queue messages")?
			.into_iter()
			.filter(|row| {
				row.dead_at.is_none()
					&& row.in_flight_at.is_none()
					&& row.available_at.is_none_or(|available_at| available_at <= now)
			})
			.map(|row| QueueMessage {
				id: row.id,
				receipt_id: row.receipt_id,
				name: row.message.name,
				body: row.message.body,
				created_at: row.message.created_at,
				attempts: row.attempt_count,
				first_failed_at: row.first_failed_at,
			})
			.collect();

		let mut metadata = self.0.queue_metadata.lock().await;
		if metadata.next_id == 0 {
			metadata.next_id = messages
				.last()
				.map(|message| message.id.saturating_add(1))
				.unwrap_or(1);
		}

		Ok(messages)
	}

	async fn decrement_queue_size(&self, count: usize) {
		let queue_size = {
			let mut metadata = self.0.queue_metadata.lock().await;
			metadata.size = metadata.size.saturating_sub(count as u32);
			metadata.size
		};
		self.0.metrics.set_queue_depth(queue_size);
		self.notify_inspector_update(queue_size);
	}

	async fn wait_for_message(
		&self,
		timeout: Option<Duration>,
		signal: Option<&CancellationToken>,
	) -> WaitOutcome {
		let actor_abort_signal = self.0.queue_abort_signal.lock().clone();
		if signal.is_some_and(CancellationToken::is_cancelled) {
			return WaitOutcome::Aborted;
		}
		if actor_abort_signal.is_cancelled() {
			return WaitOutcome::Aborted;
		}

		let notified = self.0.queue_notify.notified();
		let actor_aborted = async {
			actor_abort_signal.cancelled().await;
		};
		let external_aborted = async {
			if let Some(signal) = signal {
				signal.cancelled().await;
			} else {
				pending::<()>().await;
			}
		};

		match timeout {
			Some(timeout) => {
				tokio::select! {
					_ = notified => WaitOutcome::Notified,
					_ = actor_aborted => WaitOutcome::Aborted,
					_ = external_aborted => WaitOutcome::Aborted,
					_ = sleep(timeout) => WaitOutcome::TimedOut,
				}
			}
			None => {
				tokio::select! {
					_ = notified => WaitOutcome::Notified,
					_ = actor_aborted => WaitOutcome::Aborted,
					_ = external_aborted => WaitOutcome::Aborted,
				}
			}
		}
	}

	fn block_on<T>(&self, future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
		#[cfg(not(target_arch = "wasm32"))]
		{
			if let Ok(handle) = Handle::try_current() {
				tokio::task::block_in_place(|| handle.block_on(future))
			} else {
				Builder::new_current_thread()
					.enable_all()
					.build()
					.context("build temporary runtime for queue operation")?
					.block_on(future)
			}
		}

		#[cfg(target_arch = "wasm32")]
		{
			drop(future);
			Err(ActorRuntime::InvalidOperation {
				operation: "queue.try_next_batch".to_owned(),
				reason: "synchronous queue receive requires native runtime support".to_owned(),
			}
			.build())
		}
	}

	fn config(&self) -> ActorConfig {
		self.0.queue_config.lock().clone()
	}

	#[cfg(test)]
	pub(crate) fn queue_config_for_tests(&self) -> ActorConfig {
		self.config()
	}

	fn notify_wait_activity(&self) {
		if let Some(callback) = self.0.queue_wait_activity_callback.lock().clone() {
			callback();
		}
	}

	fn notify_inspector_update(&self, queue_size: u32) {
		if let Some(callback) = self.0.queue_inspector_update_callback.lock().clone() {
			callback(queue_size);
		}
	}

	fn record_dead_letter_purge(&self, purge: internal_storage::DeadLetterPurge) {
		for (reason, count) in [
			("retention", purge.retention),
			("capacity", purge.capacity),
		] {
			if count == 0 {
				continue;
			}
			tracing::warn!(
				actor_id = %self.actor_id(),
				reason,
				count,
				"evicted durable queue dead-letter entries"
			);
			self.0.metrics.add_queue_dead_letters_evicted(reason, count);
		}
	}
}

struct ActiveQueueWaitGuard<'a> {
	ctx: &'a ActorContext,
	started_at: Instant,
}

impl<'a> ActiveQueueWaitGuard<'a> {
	fn new(ctx: &'a ActorContext) -> Self {
		ctx.0.active_queue_wait_count.fetch_add(1, Ordering::SeqCst);
		ctx.0.metrics.begin_user_task(UserTaskKind::QueueWait);
		ctx.notify_wait_activity();
		Self {
			ctx,
			started_at: Instant::now(),
		}
	}
}

impl Drop for ActiveQueueWaitGuard<'_> {
	fn drop(&mut self) {
		self.ctx
			.0
			.metrics
			.end_user_task(UserTaskKind::QueueWait, self.started_at.elapsed());
		let previous = self
			.ctx
			.0
			.active_queue_wait_count
			.fetch_sub(1, Ordering::SeqCst);
		if previous == 0 {
			self.ctx
				.0
				.active_queue_wait_count
				.store(0, Ordering::SeqCst);
		}
		self.ctx.notify_wait_activity();
	}
}

enum WaitOutcome {
	Notified,
	TimedOut,
	Aborted,
}

fn normalize_names(names: Option<Vec<String>>) -> Option<BTreeSet<String>> {
	names.and_then(|names| {
		let normalized = names.into_iter().collect::<BTreeSet<_>>();
		if normalized.is_empty() {
			None
		} else {
			Some(normalized)
		}
	})
}

fn current_timestamp_ms() -> Result<i64> {
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("current time is before unix epoch")?;
	i64::try_from(now.as_millis()).context("queue timestamp exceeds i64")
}

fn duration_ms(duration: Duration) -> u64 {
	duration.as_millis().try_into().unwrap_or(u64::MAX)
}

fn queue_backoff(
	definition: &crate::actor::config::QueueDefinition,
	attempt: u32,
) -> Duration {
	let jitter_multiplier = if definition.backoff_jitter {
		0.5 + rand::random::<f64>()
	} else {
		1.0
	};
	queue_backoff_with_jitter(definition, attempt, jitter_multiplier)
}

fn queue_backoff_with_jitter(
	definition: &crate::actor::config::QueueDefinition,
	attempt: u32,
	jitter_multiplier: f64,
) -> Duration {
	let exponent = i32::try_from(attempt.saturating_sub(1)).unwrap_or(i32::MAX);
	let base_ms = definition.backoff_initial.as_secs_f64() * 1000.0;
	let max_ms = definition.backoff_max.as_secs_f64() * 1000.0;
	let delay_ms = ((base_ms * definition.backoff_factor.powi(exponent)).min(max_ms)
		* jitter_multiplier)
		.min(max_ms);
	Duration::from_secs_f64((delay_ms.max(0.0)) / 1000.0)
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/queue.rs"]
mod tests;
