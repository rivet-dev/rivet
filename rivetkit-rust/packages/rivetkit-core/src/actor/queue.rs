use std::collections::BTreeSet;
use std::fmt;
use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use tokio::runtime::{Builder, Handle};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::actor::config::ActorConfig;
use crate::actor::context::ActorContext;
use crate::actor::persist::{decode_with_embedded_version, encode_with_embedded_version};
use crate::actor::preload::PreloadedKv;
use crate::actor::task_types::UserTaskKind;
use crate::types::ListOpts;

const QUEUE_STORAGE_VERSION: u8 = 1;
const QUEUE_METADATA_KEY: [u8; 3] = [5, QUEUE_STORAGE_VERSION, 1];
const QUEUE_MESSAGES_PREFIX: [u8; 3] = [5, QUEUE_STORAGE_VERSION, 2];
const QUEUE_PAYLOAD_VERSION: u16 = 4;
const QUEUE_PAYLOAD_COMPATIBLE_VERSIONS: &[u16] = &[4];

#[derive(Clone, Debug, Default)]
pub struct QueueNextOpts {
	pub names: Option<Vec<String>>,
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
	pub completable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct QueueWaitOpts {
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
	pub completable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct EnqueueAndWaitOpts {
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
}

#[derive(Clone, Debug)]
pub struct QueueNextBatchOpts {
	pub names: Option<Vec<String>>,
	pub count: u32,
	pub timeout: Option<Duration>,
	pub signal: Option<CancellationToken>,
	pub completable: bool,
}

impl Default for QueueNextBatchOpts {
	fn default() -> Self {
		Self {
			names: None,
			count: 1,
			timeout: None,
			signal: None,
			completable: false,
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct QueueTryNextOpts {
	pub names: Option<Vec<String>>,
	pub completable: bool,
}

#[derive(Clone, Debug)]
pub struct QueueTryNextBatchOpts {
	pub names: Option<Vec<String>>,
	pub count: u32,
	pub completable: bool,
}

impl Default for QueueTryNextBatchOpts {
	fn default() -> Self {
		Self {
			names: None,
			count: 1,
			completable: false,
		}
	}
}

pub(super) type QueueWaitActivityCallback = Arc<dyn Fn() + Send + Sync>;
pub(super) type QueueInspectorUpdateCallback = Arc<dyn Fn(u32) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct QueueMessage {
	pub id: u64,
	pub name: String,
	pub body: Vec<u8>,
	pub created_at: i64,
	completion: Option<CompletionHandle>,
}

#[derive(Clone, Debug)]
pub struct CompletableQueueMessage {
	pub id: u64,
	pub name: String,
	pub body: Vec<u8>,
	pub created_at: i64,
	completion: CompletionHandle,
}

#[derive(Clone)]
struct CompletionHandle(Arc<CompletionHandleInner>);

struct CompletionHandleInner {
	ctx: ActorContext,
	message_id: u64,
	completed: std::sync::atomic::AtomicBool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct QueueMetadata {
	next_id: u64,
	size: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedQueueMessage {
	name: String,
	body: Vec<u8>,
	created_at: i64,
	failure_count: Option<u32>,
	available_at: Option<i64>,
	in_flight: Option<bool>,
	in_flight_at: Option<i64>,
}

fn encode_queue_metadata(metadata: &QueueMetadata) -> Result<Vec<u8>> {
	encode_with_embedded_version(metadata, QUEUE_PAYLOAD_VERSION, "queue metadata")
}

fn decode_queue_metadata(payload: &[u8]) -> Result<QueueMetadata> {
	decode_with_embedded_version(payload, QUEUE_PAYLOAD_COMPATIBLE_VERSIONS, "queue metadata")
}

fn encode_queue_message(message: &PersistedQueueMessage) -> Result<Vec<u8>> {
	encode_with_embedded_version(message, QUEUE_PAYLOAD_VERSION, "queue message")
}

fn decode_queue_message(payload: &[u8]) -> Result<PersistedQueueMessage> {
	decode_with_embedded_version(payload, QUEUE_PAYLOAD_COMPATIBLE_VERSIONS, "queue message")
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
#[error("queue", "already_completed", "Queue message was already completed")]
struct QueueAlreadyCompleted;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"complete_not_configured",
	"Queue message does not support completion",
	"Queue '{name}' does not support completion responses."
)]
struct QueueCompleteNotConfigured {
	name: String,
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
	"completion_waiter_conflict",
	"Queue completion waiter conflict",
	"Queue completion waiter is already registered for message {message_id}."
)]
struct QueueCompletionWaiterConflict {
	message_id: u64,
}

#[derive(RivetError)]
#[error(
	"queue",
	"completion_waiter_dropped",
	"Queue completion waiter dropped before response"
)]
struct QueueCompletionWaiterDropped;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"invalid_message_key",
	"Queue message key is invalid",
	"Queue message key is invalid: {reason}"
)]
struct QueueInvalidMessageKey {
	reason: String,
}

impl ActorContext {
	pub async fn send(&self, name: &str, body: &[u8]) -> Result<QueueMessage> {
		self.enqueue_message(name, body, None).await
	}

	pub async fn enqueue_and_wait(
		&self,
		name: &str,
		body: &[u8],
		opts: EnqueueAndWaitOpts,
	) -> Result<Option<Vec<u8>>> {
		let (sender, receiver) = oneshot::channel();
		let message = self.enqueue_message(name, body, Some(sender)).await?;
		let result = self
			.wait_for_completion_response(message.id, receiver, opts.timeout, opts.signal.as_ref())
			.await;
		self.remove_completion_waiter(message.id).await;
		result
	}

	async fn enqueue_message(
		&self,
		name: &str,
		body: &[u8],
		completion_waiter: Option<oneshot::Sender<Option<Vec<u8>>>>,
	) -> Result<QueueMessage> {
		self.ensure_initialized().await?;

		let created_at = current_timestamp_ms()?;
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
		self.clear_preloaded_messages();

		let config = self.config();
		if encoded_message.len() > config.max_queue_message_size as usize {
			return Err(QueueMessageTooLarge {
				size: encoded_message.len(),
				limit: config.max_queue_message_size,
			}
			.build());
		}

		let mut metadata = self.0.queue_metadata.lock().await;
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
		let encoded_metadata = encode_queue_metadata(&metadata).context("encode queue metadata")?;

		let registered_completion_waiter = if let Some(waiter) = completion_waiter {
			if self
				.0
				.queue_completion_waiters
				.insert_async(id, waiter)
				.await
				.is_err()
			{
				metadata.next_id = id;
				metadata.size = metadata.size.saturating_sub(1);
				return Err(QueueCompletionWaiterConflict { message_id: id }.build());
			}
			true
		} else {
			false
		};

		if let Err(error) = self
			.0
			.kv
			.batch_put(&[
				(
					make_queue_message_key(id).as_slice(),
					encoded_message.as_slice(),
				),
				(QUEUE_METADATA_KEY.as_slice(), encoded_metadata.as_slice()),
			])
			.await
		{
			metadata.next_id = id;
			metadata.size = metadata.size.saturating_sub(1);
			if registered_completion_waiter {
				self.remove_completion_waiter(id).await;
			}
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

		Ok(QueueMessage {
			id,
			name: name.to_owned(),
			body: body.to_vec(),
			created_at,
			completion: None,
		})
	}

	pub async fn next(&self, opts: QueueNextOpts) -> Result<Option<QueueMessage>> {
		let mut messages = self
			.next_batch(QueueNextBatchOpts {
				names: opts.names,
				count: 1,
				timeout: opts.timeout,
				signal: opts.signal,
				completable: opts.completable,
			})
			.await?;
		Ok(messages.pop())
	}

	pub async fn next_batch(&self, opts: QueueNextBatchOpts) -> Result<Vec<QueueMessage>> {
		self.ensure_initialized().await?;

		let count = opts.count.max(1);
		let deadline = opts.timeout.map(|timeout| Instant::now() + timeout);
		let names = normalize_names(opts.names);

		loop {
			let messages = self
				.try_receive_batch(names.as_ref(), count, opts.completable)
				.await?;
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
		let names = normalize_names(Some(names));

		loop {
			if let Some(message) = self
				.try_receive_batch(names.as_ref(), 1, opts.completable)
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
		let names = normalize_names(Some(names));

		loop {
			let messages = self.list_messages().await?;
			let has_match = if let Some(names) = names.as_ref() {
				messages
					.into_iter()
					.any(|message| names.contains(&message.name))
			} else {
				!messages.is_empty()
			};
			if has_match {
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
			completable: opts.completable,
		})?;
		Ok(messages.pop())
	}

	pub fn try_next_batch(&self, opts: QueueTryNextBatchOpts) -> Result<Vec<QueueMessage>> {
		self.block_on(async {
			self.ensure_initialized().await?;
			self.try_receive_batch(
				normalize_names(opts.names).as_ref(),
				opts.count.max(1),
				opts.completable,
			)
			.await
		})
	}

	pub async fn inspect_messages(&self) -> Result<Vec<QueueMessage>> {
		self.ensure_initialized().await?;
		self.list_messages().await
	}

	pub fn max_size(&self) -> u32 {
		self.config().max_queue_size
	}

	pub(crate) fn configure_queue(&self, config: ActorConfig) {
		*self.0.queue_config.lock() = config;
	}

	pub(crate) fn configure_preload(&self, preloaded_kv: Option<PreloadedKv>) {
		*self.0.queue_preloaded_kv.lock() = preloaded_kv;
		*self.0.queue_preloaded_message_entries.lock() = None;
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
				let preload = self.0.queue_preloaded_kv.lock().take();
				let metadata = if let Some(preloaded) = preload.as_ref() {
					self.configure_preloaded_messages(preloaded);
					if let Some(metadata) = self.load_metadata_from_preload(preloaded).await? {
						metadata
					} else {
						self.load_or_create_metadata().await?
					}
				} else {
					self.load_or_create_metadata().await?
				};
				let mut state = self.0.queue_metadata.lock().await;
				*state = metadata;
				self.0.metrics.set_queue_depth(state.size);
				Ok(())
			})
			.await
			.map(|_| ())
	}

	fn configure_preloaded_messages(&self, preloaded: &PreloadedKv) {
		if let Some(entries) = preloaded.prefix_entries(&QUEUE_MESSAGES_PREFIX) {
			*self.0.queue_preloaded_message_entries.lock() = Some(entries);
		}
	}

	async fn load_metadata_from_preload(
		&self,
		preloaded: &PreloadedKv,
	) -> Result<Option<QueueMetadata>> {
		match preloaded.key_entry(&QUEUE_METADATA_KEY) {
			Some(Some(encoded)) => match decode_queue_metadata(&encoded) {
				Ok(metadata) => Ok(Some(metadata)),
				Err(error) => {
					tracing::warn!(
						?error,
						"failed to decode preloaded queue metadata, rebuilding"
					);
					Ok(self.metadata_from_preloaded_messages())
				}
			},
			Some(None) => Ok(self.metadata_from_preloaded_messages()),
			None => Ok(None),
		}
	}

	fn metadata_from_preloaded_messages(&self) -> Option<QueueMetadata> {
		let entries = self.0.queue_preloaded_message_entries.lock().clone()?;
		Some(metadata_from_queue_messages(decode_queue_message_entries(
			entries,
		)))
	}

	async fn load_or_create_metadata(&self) -> Result<QueueMetadata> {
		let Some(encoded) = self.0.kv.get(&QUEUE_METADATA_KEY).await? else {
			let metadata = QueueMetadata {
				next_id: 1,
				size: 0,
			};
			self.0
				.kv
				.put(
					&QUEUE_METADATA_KEY,
					&encode_queue_metadata(&metadata).context("encode default queue metadata")?,
				)
				.await
				.context("persist default queue metadata")?;
			return Ok(metadata);
		};

		match decode_queue_metadata(&encoded) {
			Ok(metadata) => Ok(metadata),
			Err(error) => {
				tracing::warn!(?error, "failed to decode queue metadata, rebuilding");
				self.rebuild_metadata().await
			}
		}
	}

	async fn rebuild_metadata(&self) -> Result<QueueMetadata> {
		let messages = self.list_messages().await?;
		let metadata = metadata_from_queue_messages(messages);
		self.persist_metadata(&metadata)
			.await
			.context("persist rebuilt queue metadata")?;
		Ok(metadata)
	}

	async fn persist_metadata(&self, metadata: &QueueMetadata) -> Result<()> {
		let encoded = encode_queue_metadata(metadata).context("encode queue metadata")?;
		self.0
			.kv
			.put(&QUEUE_METADATA_KEY, &encoded)
			.await
			.context("persist queue metadata")?;
		self.notify_inspector_update(metadata.size);
		Ok(())
	}

	async fn try_receive_batch(
		&self,
		names: Option<&BTreeSet<String>>,
		count: u32,
		completable: bool,
	) -> Result<Vec<QueueMessage>> {
		let _receive_guard = self.0.queue_receive_lock.lock().await;

		let messages = self.list_messages().await?;
		let mut selected = Vec::new();
		for message in messages {
			if let Some(names) = names
				&& !names.contains(&message.name)
			{
				continue;
			}

			selected.push(message);
			if selected.len() >= count as usize {
				break;
			}
		}

		if selected.is_empty() {
			return Ok(Vec::new());
		}

		if completable {
			let queue_size = self.0.queue_metadata.lock().await.size;
			self.0
				.metrics
				.add_queue_messages_received(selected.len().try_into().unwrap_or(u64::MAX));
			self.notify_inspector_update(queue_size);
			return Ok(selected
				.into_iter()
				.map(|message| self.attach_completion(message))
				.collect());
		}

		self.remove_messages(selected.iter().map(|message| message.id).collect())
			.await?;
		self.0
			.metrics
			.add_queue_messages_received(selected.len().try_into().unwrap_or(u64::MAX));

		Ok(selected)
	}

	async fn list_messages(&self) -> Result<Vec<QueueMessage>> {
		let messages = decode_queue_message_entries(self.list_message_entries().await?);

		let actual_size = messages.len().try_into().unwrap_or(u32::MAX);
		let mut metadata = self.0.queue_metadata.lock().await;
		if metadata.size != actual_size {
			metadata.size = actual_size;
		}
		if metadata.next_id == 0 {
			metadata.next_id = messages
				.last()
				.map(|message| message.id.saturating_add(1))
				.unwrap_or(1);
		}

		Ok(messages)
	}

	async fn list_message_entries(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		if let Some(entries) = self.0.queue_preloaded_message_entries.lock().take() {
			return Ok(entries);
		}

		self.0
			.kv
			.list_prefix(
				&QUEUE_MESSAGES_PREFIX,
				ListOpts {
					reverse: false,
					limit: None,
				},
			)
			.await
			.context("list queue messages")
	}

	fn clear_preloaded_messages(&self) {
		self.0.queue_preloaded_message_entries.lock().take();
	}

	fn attach_completion(&self, mut message: QueueMessage) -> QueueMessage {
		message.completion = Some(CompletionHandle::new(self.clone(), message.id));
		message
	}

	async fn remove_messages(&self, message_ids: Vec<u64>) -> Result<()> {
		if message_ids.is_empty() {
			return Ok(());
		}

		let keys: Vec<Vec<u8>> = message_ids
			.into_iter()
			.map(make_queue_message_key)
			.collect();
		let key_refs: Vec<&[u8]> = keys.iter().map(Vec::as_slice).collect();

		self.0
			.kv
			.batch_delete(&key_refs)
			.await
			.context("delete queue messages")?;

		let encoded_metadata = {
			let mut metadata = self.0.queue_metadata.lock().await;
			metadata.size = metadata.size.saturating_sub(key_refs.len() as u32);
			let queue_size = metadata.size;
			encode_queue_metadata(&metadata)
				.context("encode queue metadata after delete")
				.map(|encoded| (encoded, queue_size))?
		};
		let (encoded_metadata, queue_size) = encoded_metadata;

		self.0
			.kv
			.put(&QUEUE_METADATA_KEY, &encoded_metadata)
			.await
			.context("persist queue metadata after delete")?;
		self.0
			.metrics
			.set_queue_depth(self.0.queue_metadata.lock().await.size);
		self.notify_inspector_update(queue_size);
		Ok(())
	}

	async fn complete_message_by_id(
		&self,
		message_id: u64,
		response: Option<Vec<u8>>,
	) -> Result<()> {
		self.remove_messages(vec![message_id]).await?;
		if let Some(waiter) = self.remove_completion_waiter(message_id).await {
			let _ = waiter.send(response);
		}
		Ok(())
	}

	async fn remove_completion_waiter(
		&self,
		message_id: u64,
	) -> Option<oneshot::Sender<Option<Vec<u8>>>> {
		self.0
			.queue_completion_waiters
			.remove_async(&message_id)
			.await
			.map(|(_, waiter)| waiter)
	}

	async fn wait_for_message(
		&self,
		timeout: Option<Duration>,
		signal: Option<&CancellationToken>,
	) -> WaitOutcome {
		if signal.is_some_and(CancellationToken::is_cancelled) {
			return WaitOutcome::Aborted;
		}
		if self
			.0
			.queue_abort_signal
			.as_ref()
			.is_some_and(CancellationToken::is_cancelled)
		{
			return WaitOutcome::Aborted;
		}

		let notified = self.0.queue_notify.notified();
		let actor_aborted = async {
			if let Some(signal) = &self.0.queue_abort_signal {
				signal.cancelled().await;
			} else {
				pending::<()>().await;
			}
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
					_ = tokio::time::sleep(timeout) => WaitOutcome::TimedOut,
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

	/// TS parity: queue-manager.ts keeps `enqueueAndWait` completion waits
	/// alive across actor aborts; the surrounding tracked user task owns
	/// shutdown cancellation.
	async fn wait_for_completion_response(
		&self,
		message_id: u64,
		mut receiver: oneshot::Receiver<Option<Vec<u8>>>,
		timeout: Option<Duration>,
		signal: Option<&CancellationToken>,
	) -> Result<Option<Vec<u8>>> {
		if signal.is_some_and(CancellationToken::is_cancelled) {
			return Err(QueueActorAborted.build());
		}

		let external_aborted = async {
			if let Some(signal) = signal {
				signal.cancelled().await;
			} else {
				pending::<()>().await;
			}
		};

		let wait_result = match timeout {
			Some(timeout) => {
				tokio::select! {
					response = &mut receiver => CompletionWaitOutcome::Response(response),
					_ = external_aborted => CompletionWaitOutcome::Aborted,
					_ = tokio::time::sleep(timeout) => CompletionWaitOutcome::TimedOut,
				}
			}
			None => {
				tokio::select! {
					response = &mut receiver => CompletionWaitOutcome::Response(response),
					_ = external_aborted => CompletionWaitOutcome::Aborted,
				}
			}
		};

		match wait_result {
			CompletionWaitOutcome::Response(Ok(response)) => Ok(response),
			CompletionWaitOutcome::Response(Err(_)) => Err(QueueCompletionWaiterDropped.build())
				.context(format!("wait for queue completion on message {message_id}")),
			CompletionWaitOutcome::TimedOut => Err(QueueWaitTimedOut {
				timeout_ms: timeout.map(duration_ms).unwrap_or(0),
			}
			.build()),
			CompletionWaitOutcome::Aborted => Err(QueueActorAborted.build()),
		}
	}

	fn block_on<T>(&self, future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
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
}

impl QueueMessage {
	pub async fn complete(self, response: Option<Vec<u8>>) -> Result<()> {
		let completable = self.into_completable()?;
		completable.complete(response).await
	}

	pub fn into_completable(self) -> Result<CompletableQueueMessage> {
		let completion = self.completion.clone().ok_or_else(|| {
			QueueCompleteNotConfigured {
				name: self.name.clone(),
			}
			.build()
		})?;

		Ok(CompletableQueueMessage {
			id: self.id,
			name: self.name,
			body: self.body,
			created_at: self.created_at,
			completion,
		})
	}

	pub fn is_completable(&self) -> bool {
		self.completion.is_some()
	}
}

impl CompletableQueueMessage {
	pub async fn complete(self, response: Option<Vec<u8>>) -> Result<()> {
		self.completion.complete(response).await
	}

	pub fn into_message(self) -> QueueMessage {
		QueueMessage {
			id: self.id,
			name: self.name,
			body: self.body,
			created_at: self.created_at,
			completion: Some(self.completion),
		}
	}
}

impl CompletionHandle {
	fn new(ctx: ActorContext, message_id: u64) -> Self {
		Self(Arc::new(CompletionHandleInner {
			ctx,
			message_id,
			completed: std::sync::atomic::AtomicBool::new(false),
		}))
	}

	async fn complete(&self, response: Option<Vec<u8>>) -> Result<()> {
		if self.0.completed.swap(true, Ordering::SeqCst) {
			return Err(QueueAlreadyCompleted.build());
		}

		if let Err(error) = self
			.0
			.ctx
			.complete_message_by_id(self.0.message_id, response)
			.await
		{
			self.0.completed.store(false, Ordering::SeqCst);
			return Err(error);
		}

		Ok(())
	}
}

impl fmt::Debug for CompletionHandle {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("CompletionHandle")
			.field("message_id", &self.0.message_id)
			.field("completed", &self.0.completed.load(Ordering::SeqCst))
			.finish()
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

enum CompletionWaitOutcome {
	Response(Result<Option<Vec<u8>>, oneshot::error::RecvError>),
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

fn make_queue_message_key(id: u64) -> Vec<u8> {
	let mut key = Vec::with_capacity(QUEUE_MESSAGES_PREFIX.len() + 8);
	key.extend_from_slice(&QUEUE_MESSAGES_PREFIX);
	key.extend_from_slice(&id.to_be_bytes());
	key
}

fn decode_queue_message_key(key: &[u8]) -> Result<u64> {
	if key.len() != QUEUE_MESSAGES_PREFIX.len() + 8 {
		return Err(invalid_queue_key("invalid length"));
	}
	if !key.starts_with(&QUEUE_MESSAGES_PREFIX) {
		return Err(invalid_queue_key("invalid prefix"));
	}

	let bytes: [u8; 8] = key[QUEUE_MESSAGES_PREFIX.len()..]
		.try_into()
		.map_err(|_| invalid_queue_key("invalid id bytes"))?;
	Ok(u64::from_be_bytes(bytes))
}

fn invalid_queue_key(reason: &str) -> anyhow::Error {
	QueueInvalidMessageKey {
		reason: reason.to_owned(),
	}
	.build()
}

fn decode_queue_message_entries(entries: Vec<(Vec<u8>, Vec<u8>)>) -> Vec<QueueMessage> {
	let mut messages = Vec::with_capacity(entries.len());
	for (key, value) in entries {
		let id = match decode_queue_message_key(&key) {
			Ok(id) => id,
			Err(error) => {
				tracing::warn!(?error, "failed to decode queue message key");
				continue;
			}
		};

		match decode_queue_message(&value) {
			Ok(message) => messages.push(QueueMessage {
				id,
				name: message.name,
				body: message.body,
				created_at: message.created_at,
				completion: None,
			}),
			Err(error) => {
				tracing::warn!(
					?error,
					queue_message_id = id,
					"failed to decode queue message"
				);
			}
		}
	}

	messages.sort_by_key(|message| message.id);
	messages
}

fn metadata_from_queue_messages(messages: Vec<QueueMessage>) -> QueueMetadata {
	let next_id = messages
		.last()
		.map(|message| message.id.saturating_add(1))
		.unwrap_or(1);
	QueueMetadata {
		next_id,
		size: messages.len().try_into().unwrap_or(u32::MAX),
	}
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

#[cfg(test)]
mod tests {
	use super::{
		PersistedQueueMessage, QUEUE_MESSAGES_PREFIX, QUEUE_METADATA_KEY, QueueMetadata,
		QueueNextOpts, QueueWaitOpts, encode_queue_message, encode_queue_metadata,
		make_queue_message_key,
	};

	use crate::actor::context::ActorContext;
	use crate::actor::preload::PreloadedKv;
	use crate::kv::Kv;
	use std::time::Duration;
	use tokio::task::yield_now;
	use tokio_util::sync::CancellationToken;

	fn test_queue() -> ActorContext {
		ActorContext::new_with_kv(
			"actor-queue",
			"queue-test",
			Vec::new(),
			"local",
			Kv::new_in_memory(),
		)
	}

	fn assert_actor_aborted(error: anyhow::Error) {
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "aborted");
	}

	#[tokio::test]
	async fn inspect_messages_uses_preloaded_queue_entries_when_present() {
		let queue = ActorContext::new_with_kv(
			"actor-queue",
			"queue-preload",
			Vec::new(),
			"local",
			Kv::default(),
		);
		let metadata = QueueMetadata {
			next_id: 8,
			size: 1,
		};
		let persisted = PersistedQueueMessage {
			name: "preloaded".to_owned(),
			body: b"body".to_vec(),
			created_at: 42,
			failure_count: None,
			available_at: None,
			in_flight: None,
			in_flight_at: None,
		};
		queue.configure_preload(Some(PreloadedKv::new_with_requested_get_keys(
			[
				(
					QUEUE_METADATA_KEY.to_vec(),
					encode_queue_metadata(&metadata).expect("metadata should encode"),
				),
				(
					make_queue_message_key(7),
					encode_queue_message(&persisted).expect("message should encode"),
				),
			],
			vec![QUEUE_METADATA_KEY.to_vec()],
			vec![QUEUE_MESSAGES_PREFIX.to_vec()],
		)));

		let messages = queue
			.inspect_messages()
			.await
			.expect("queue should initialize from preload without touching kv");

		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].id, 7);
		assert_eq!(messages[0].name, "preloaded");
		assert_eq!(messages[0].body, b"body");
		assert_eq!(*queue.0.queue_metadata.lock().await, metadata);
	}

	#[tokio::test]
	async fn wait_for_names_returns_aborted_when_signal_is_already_cancelled() {
		let queue = test_queue();
		let signal = CancellationToken::new();
		signal.cancel();

		let error = queue
			.wait_for_names(
				vec!["missing".to_owned()],
				QueueWaitOpts {
					signal: Some(signal),
					..Default::default()
				},
			)
			.await
			.expect_err("already-cancelled waits should abort immediately");

		assert_actor_aborted(error);
	}

	#[tokio::test(start_paused = true)]
	async fn wait_for_names_returns_aborted_when_signal_cancels_during_wait() {
		let queue = test_queue();
		let signal = CancellationToken::new();
		let wait_signal = signal.clone();
		let wait_queue = queue.clone();

		let wait = tokio::spawn(async move {
			wait_queue
				.wait_for_names(
					vec!["missing".to_owned()],
					QueueWaitOpts {
						timeout: Some(Duration::from_secs(60)),
						signal: Some(wait_signal),
						..Default::default()
					},
				)
				.await
		});

		yield_now().await;
		signal.cancel();

		let error = wait
			.await
			.expect("wait task should join")
			.expect_err("cancelled waits should abort");

		assert_actor_aborted(error);
	}

	#[tokio::test(start_paused = true)]
	async fn next_returns_aborted_when_actor_signal_cancels_during_wait() {
		let queue = test_queue();

		let wait = tokio::spawn({
			let queue = queue.clone();
			async move {
				queue
					.next(QueueNextOpts {
						names: Some(vec!["missing".to_owned()]),
						timeout: Some(Duration::from_secs(60)),
						..Default::default()
					})
					.await
			}
		});

		yield_now().await;
		queue.mark_destroy_requested();

		let error = wait
			.await
			.expect("wait task should join")
			.expect_err("cancelled actor waits should abort");

		assert_actor_aborted(error);
	}
}
