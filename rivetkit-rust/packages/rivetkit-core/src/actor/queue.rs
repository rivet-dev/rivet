use std::collections::BTreeSet;
use std::fmt;
use std::future::pending;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use tokio::runtime::{Builder, Handle};
use tokio::sync::{Mutex, Notify, OnceCell};
use tokio_util::sync::CancellationToken;

use crate::actor::config::ActorConfig;
use crate::actor::metrics::ActorMetrics;
use crate::actor::persist::{
	decode_with_embedded_version, encode_with_embedded_version,
};
use crate::kv::Kv;
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

#[derive(Clone)]
pub struct Queue(Arc<QueueInner>);

struct QueueInner {
	kv: Kv,
	config: StdMutex<ActorConfig>,
	abort_signal: Option<CancellationToken>,
	initialize: OnceCell<()>,
	metadata: Mutex<QueueMetadata>,
	receive_lock: Mutex<()>,
	pending_completable_message_ids: Mutex<BTreeSet<u64>>,
	notify: Notify,
	active_queue_wait_count: AtomicU32,
	wait_activity_callback: StdMutex<Option<Arc<dyn Fn() + Send + Sync>>>,
	metrics: ActorMetrics,
}

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
	queue: Queue,
	message_id: u64,
	completed: std::sync::atomic::AtomicBool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct QueueMetadata {
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
	decode_with_embedded_version(
		payload,
		QUEUE_PAYLOAD_COMPATIBLE_VERSIONS,
		"queue metadata",
	)
}

fn encode_queue_message(message: &PersistedQueueMessage) -> Result<Vec<u8>> {
	encode_with_embedded_version(message, QUEUE_PAYLOAD_VERSION, "queue message")
}

fn decode_queue_message(payload: &[u8]) -> Result<PersistedQueueMessage> {
	decode_with_embedded_version(
		payload,
		QUEUE_PAYLOAD_COMPATIBLE_VERSIONS,
		"queue message",
	)
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
#[error(
	"queue",
	"already_completed",
	"Queue message was already completed"
)]
struct QueueAlreadyCompleted;

#[derive(RivetError)]
#[error(
	"queue",
	"previous_message_not_completed",
	"Previous completable queue message is not completed. Call `message.complete(...)` before receiving the next message."
)]
struct QueuePreviousMessageNotCompleted;

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

impl Queue {
	pub(crate) fn new(
		kv: Kv,
		config: ActorConfig,
		abort_signal: Option<CancellationToken>,
		metrics: ActorMetrics,
	) -> Self {
		Self(Arc::new(QueueInner {
			kv,
			config: StdMutex::new(config),
			abort_signal,
			initialize: OnceCell::new(),
			metadata: Mutex::new(QueueMetadata::default()),
			receive_lock: Mutex::new(()),
			pending_completable_message_ids: Mutex::new(BTreeSet::new()),
			notify: Notify::new(),
			active_queue_wait_count: AtomicU32::new(0),
			wait_activity_callback: StdMutex::new(None),
			metrics,
		}))
	}

	pub async fn send(&self, name: &str, body: &[u8]) -> Result<QueueMessage> {
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
		let encoded_message =
			encode_queue_message(&persisted).context("encode queue message")?;

		let config = self.config();
		if encoded_message.len() > config.max_queue_message_size as usize {
			return Err(QueueMessageTooLarge {
				size: encoded_message.len(),
				limit: config.max_queue_message_size,
			}
			.build()
			.into());
		}

		let mut metadata = self.0.metadata.lock().await;
		if metadata.size >= config.max_queue_size {
			return Err(QueueFull {
				limit: config.max_queue_size,
			}
			.build()
			.into());
		}

		let id = if metadata.next_id == 0 { 1 } else { metadata.next_id };
		metadata.next_id = id.saturating_add(1);
		metadata.size = metadata.size.saturating_add(1);
		let encoded_metadata =
			encode_queue_metadata(&metadata).context("encode queue metadata")?;

		if let Err(error) = self
			.0
			.kv
			.batch_put(&[
				(make_queue_message_key(id).as_slice(), encoded_message.as_slice()),
				(QUEUE_METADATA_KEY.as_slice(), encoded_metadata.as_slice()),
			])
			.await
		{
			metadata.next_id = id;
			metadata.size = metadata.size.saturating_sub(1);
			return Err(error).context("persist queue message");
		}

		drop(metadata);
		self.0.metrics.add_queue_messages_sent(1);
		self
			.0
			.metrics
			.set_queue_depth(self.0.metadata.lock().await.size);
		self.0.notify.notify_waiters();

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

			let remaining_timeout = deadline.map(|deadline| {
				deadline.saturating_duration_since(Instant::now())
			});
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
				WaitOutcome::Aborted => return Err(QueueActorAborted.build().into()),
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

			let remaining_timeout = deadline.map(|deadline| {
				deadline.saturating_duration_since(Instant::now())
			});
			if let Some(timeout) = remaining_timeout {
				if timeout.is_zero() {
					return Err(QueueWaitTimedOut {
						timeout_ms: opts.timeout.map(duration_ms).unwrap_or(0),
					}
					.build()
					.into());
				}
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
					.build()
					.into());
				}
				WaitOutcome::Aborted => return Err(QueueActorAborted.build().into()),
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

	pub(crate) fn active_queue_wait_count(&self) -> u32 {
		self.0.active_queue_wait_count.load(Ordering::SeqCst)
	}

	#[allow(dead_code)]
	pub(crate) fn configure_sleep(&self, config: ActorConfig) {
		*self.0.config.lock().expect("queue config lock poisoned") = config;
	}

	pub(crate) fn set_wait_activity_callback(
		&self,
		callback: Option<Arc<dyn Fn() + Send + Sync>>,
	) {
		*self
			.0
			.wait_activity_callback
			.lock()
			.expect("queue wait activity callback lock poisoned") = callback;
	}

	async fn ensure_initialized(&self) -> Result<()> {
		self.0
			.initialize
			.get_or_try_init(|| async {
				let metadata = self.load_or_create_metadata().await?;
				let mut state = self.0.metadata.lock().await;
				*state = metadata;
				self.0.metrics.set_queue_depth(state.size);
				Ok(())
			})
			.await
			.map(|_| ())
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
					&encode_queue_metadata(&metadata)
						.context("encode default queue metadata")?,
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
		let next_id = messages
			.last()
			.map(|message| message.id.saturating_add(1))
			.unwrap_or(1);
		let metadata = QueueMetadata {
			next_id,
			size: messages.len().try_into().unwrap_or(u32::MAX),
		};
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
			.context("persist queue metadata")
	}

	async fn try_receive_batch(
		&self,
		names: Option<&BTreeSet<String>>,
		count: u32,
		completable: bool,
	) -> Result<Vec<QueueMessage>> {
		let _receive_guard = self.0.receive_lock.lock().await;

		if !self
			.0
			.pending_completable_message_ids
			.lock()
			.await
			.is_empty()
		{
			return Err(QueuePreviousMessageNotCompleted.build().into());
		}

		let messages = self.list_messages().await?;
		let mut selected = Vec::new();
		for message in messages {
			if let Some(names) = names {
				if !names.contains(&message.name) {
					continue;
				}
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
			let mut pending = self.0.pending_completable_message_ids.lock().await;
			pending.extend(selected.iter().map(|message| message.id));
			self
				.0
				.metrics
				.add_queue_messages_received(selected.len().try_into().unwrap_or(u64::MAX));
			return Ok(selected
				.into_iter()
				.map(|message| self.attach_completion(message))
				.collect());
		}

		self
			.remove_messages(selected.iter().map(|message| message.id).collect())
			.await?;
		self
			.0
			.metrics
			.add_queue_messages_received(selected.len().try_into().unwrap_or(u64::MAX));

		Ok(selected)
	}

	async fn list_messages(&self) -> Result<Vec<QueueMessage>> {
		let entries = self
			.0
			.kv
			.list_prefix(
				&QUEUE_MESSAGES_PREFIX,
				ListOpts {
					reverse: false,
					limit: None,
				},
			)
			.await
			.context("list queue messages")?;

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
					tracing::warn!(?error, queue_message_id = id, "failed to decode queue message");
				}
			}
		}

		messages.sort_by_key(|message| message.id);

		let actual_size = messages.len().try_into().unwrap_or(u32::MAX);
		let mut metadata = self.0.metadata.lock().await;
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
			let mut metadata = self.0.metadata.lock().await;
			metadata.size = metadata.size.saturating_sub(key_refs.len() as u32);
			encode_queue_metadata(&metadata)
				.context("encode queue metadata after delete")?
		};

		self.0
			.kv
			.put(&QUEUE_METADATA_KEY, &encoded_metadata)
			.await
			.context("persist queue metadata after delete")?;
		self
			.0
			.metrics
			.set_queue_depth(self.0.metadata.lock().await.size);
		Ok(())
	}

	async fn complete_message_by_id(
		&self,
		message_id: u64,
		_response: Option<Vec<u8>>,
	) -> Result<()> {
		self.remove_messages(vec![message_id]).await?;
		self
			.0
			.pending_completable_message_ids
			.lock()
			.await
			.remove(&message_id);
		Ok(())
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
			.abort_signal
			.as_ref()
			.is_some_and(CancellationToken::is_cancelled)
		{
			return WaitOutcome::Aborted;
		}

		let notified = self.0.notify.notified();
		let actor_aborted = async {
			if let Some(signal) = &self.0.abort_signal {
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
		self.0
			.config
			.lock()
			.expect("queue config lock poisoned")
			.clone()
	}

	fn notify_wait_activity(&self) {
		if let Some(callback) = self
			.0
			.wait_activity_callback
			.lock()
			.expect("queue wait activity callback lock poisoned")
			.clone()
		{
			callback();
		}
	}
}

impl Default for Queue {
	fn default() -> Self {
		Self::new(
			Kv::default(),
			ActorConfig::default(),
			None,
			ActorMetrics::default(),
		)
	}
}

impl fmt::Debug for Queue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Queue")
			.field("configured", &true)
			.field("active_queue_wait_count", &self.active_queue_wait_count())
			.finish()
	}
}

impl QueueMessage {
	pub async fn complete(self, response: Option<Vec<u8>>) -> Result<()> {
		let completable = self.into_completable()?;
		completable.complete(response).await
	}

	pub fn into_completable(self) -> Result<CompletableQueueMessage> {
		let completion = self
			.completion
			.clone()
			.ok_or_else(|| QueueCompleteNotConfigured {
				name: self.name.clone(),
			}
			.build())?;

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
	fn new(queue: Queue, message_id: u64) -> Self {
		Self(Arc::new(CompletionHandleInner {
			queue,
			message_id,
			completed: std::sync::atomic::AtomicBool::new(false),
		}))
	}

	async fn complete(&self, response: Option<Vec<u8>>) -> Result<()> {
		if self.0.completed.swap(true, Ordering::SeqCst) {
			return Err(QueueAlreadyCompleted.build().into());
		}

		if let Err(error) = self
			.0
			.queue
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
	queue: &'a Queue,
}

impl<'a> ActiveQueueWaitGuard<'a> {
	fn new(queue: &'a Queue) -> Self {
		queue
			.0
			.active_queue_wait_count
			.fetch_add(1, Ordering::SeqCst);
		queue.notify_wait_activity();
		Self { queue }
	}
}

impl Drop for ActiveQueueWaitGuard<'_> {
	fn drop(&mut self) {
		let previous = self
			.queue
			.0
			.active_queue_wait_count
			.fetch_sub(1, Ordering::SeqCst);
		if previous == 0 {
			self.queue.0.active_queue_wait_count.store(0, Ordering::SeqCst);
		}
		self.queue.notify_wait_activity();
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

fn make_queue_message_key(id: u64) -> Vec<u8> {
	let mut key = Vec::with_capacity(QUEUE_MESSAGES_PREFIX.len() + 8);
	key.extend_from_slice(&QUEUE_MESSAGES_PREFIX);
	key.extend_from_slice(&id.to_be_bytes());
	key
}

fn decode_queue_message_key(key: &[u8]) -> Result<u64> {
	if key.len() != QUEUE_MESSAGES_PREFIX.len() + 8 {
		return Err(anyhow!("queue message key has invalid length"));
	}
	if !key.starts_with(&QUEUE_MESSAGES_PREFIX) {
		return Err(anyhow!("queue message key has invalid prefix"));
	}

	let bytes: [u8; 8] = key[QUEUE_MESSAGES_PREFIX.len()..]
		.try_into()
		.map_err(|_| anyhow!("queue message key has invalid id bytes"))?;
	Ok(u64::from_be_bytes(bytes))
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
#[path = "../../tests/modules/queue.rs"]
pub(crate) mod tests;
