use std::sync::Mutex;
use std::time::Duration;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::{
	EnqueueAndWaitOpts, Queue as CoreQueue, QueueMessage as CoreQueueMessage,
	QueueNextBatchOpts, QueueNextOpts, QueueTryNextBatchOpts, QueueTryNextOpts,
	QueueWaitOpts,
};

use crate::cancellation_token::CancellationToken;
use crate::napi_anyhow_error;

#[napi(object)]
pub struct JsQueueNextOptions {
	pub names: Option<Vec<String>>,
	pub timeout_ms: Option<i64>,
	pub completable: Option<bool>,
}

#[napi(object)]
pub struct JsQueueNextBatchOptions {
	pub names: Option<Vec<String>>,
	pub count: Option<u32>,
	pub timeout_ms: Option<i64>,
	pub completable: Option<bool>,
}

#[napi(object)]
pub struct JsQueueWaitOptions {
	pub timeout_ms: Option<i64>,
	pub completable: Option<bool>,
}

#[napi(object)]
pub struct JsQueueEnqueueAndWaitOptions {
	pub timeout_ms: Option<i64>,
}

#[napi(object)]
pub struct JsQueueTryNextOptions {
	pub names: Option<Vec<String>>,
	pub completable: Option<bool>,
}

#[napi(object)]
pub struct JsQueueTryNextBatchOptions {
	pub names: Option<Vec<String>>,
	pub count: Option<u32>,
	pub completable: Option<bool>,
}

#[napi]
pub struct Queue {
	inner: CoreQueue,
}

#[napi]
pub struct QueueMessage {
	inner: Mutex<Option<CoreQueueMessage>>,
	id: u64,
	name: String,
	body: Vec<u8>,
	created_at: i64,
	is_completable: bool,
}

impl Queue {
	pub(crate) fn new(inner: CoreQueue) -> Self {
		Self { inner }
	}
}

impl QueueMessage {
	fn from_core(message: CoreQueueMessage) -> Self {
		Self {
			id: message.id,
			name: message.name.clone(),
			body: message.body.clone(),
			created_at: message.created_at,
			is_completable: message.is_completable(),
			inner: Mutex::new(Some(message)),
		}
	}
}

#[napi]
impl Queue {
	#[napi]
	pub async fn send(&self, name: String, body: Buffer) -> napi::Result<QueueMessage> {
		self.inner
			.send(&name, body.as_ref())
			.await
			.map(QueueMessage::from_core)
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
	) -> napi::Result<QueueMessage> {
		self.inner
			.wait_for_names(names, queue_wait_opts(options)?)
			.await
			.map(QueueMessage::from_core)
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn wait_for_names_available(
		&self,
		names: Vec<String>,
		options: Option<JsQueueWaitOptions>,
	) -> napi::Result<()> {
		self.inner
			.wait_for_names_available(names, queue_wait_opts(options)?)
			.await
			.map_err(napi_anyhow_error)
	}

	#[napi]
	pub async fn enqueue_and_wait(
		&self,
		name: String,
		body: Buffer,
		options: Option<JsQueueEnqueueAndWaitOptions>,
		signal: Option<&CancellationToken>,
	) -> napi::Result<Option<Buffer>> {
		self.inner
			.enqueue_and_wait(&name, body.as_ref(), enqueue_and_wait_opts(options, signal)?)
			.await
			.map(|response| response.map(Buffer::from))
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
}

#[napi]
impl QueueMessage {
	#[napi]
	pub fn id(&self) -> u64 {
		self.id
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
	pub fn is_completable(&self) -> bool {
		self.is_completable
	}

	#[napi]
	pub async fn complete(&self, response: Option<Buffer>) -> napi::Result<()> {
		let message = {
			let mut guard = self
				.inner
				.lock()
				.map_err(|_| napi::Error::from_reason("queue message mutex poisoned"))?;
			guard
				.take()
				.ok_or_else(|| napi::Error::from_reason("queue message already completed"))?
		};

		if let Err(error) = message
			.clone()
			.complete(response.map(|response| response.to_vec()))
			.await
		{
			let mut guard = self
				.inner
				.lock()
				.map_err(|_| napi::Error::from_reason("queue message mutex poisoned"))?;
			*guard = Some(message);
			return Err(napi_anyhow_error(error));
		}

		Ok(())
	}
}

fn queue_next_opts(
	options: Option<JsQueueNextOptions>,
	signal: Option<&CancellationToken>,
) -> napi::Result<QueueNextOpts> {
	let options = options.unwrap_or(JsQueueNextOptions {
		names: None,
		timeout_ms: None,
		completable: None,
	});

	Ok(QueueNextOpts {
		names: options.names,
		timeout: timeout_duration(options.timeout_ms)?,
		signal: signal.map(|signal| signal.inner().clone()),
		completable: options.completable.unwrap_or(false),
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
		completable: None,
	});

	Ok(QueueNextBatchOpts {
		names: options.names,
		count: options.count.unwrap_or(1),
		timeout: timeout_duration(options.timeout_ms)?,
		signal: signal.map(|signal| signal.inner().clone()),
		completable: options.completable.unwrap_or(false),
	})
}

fn queue_wait_opts(options: Option<JsQueueWaitOptions>) -> napi::Result<QueueWaitOpts> {
	let options = options.unwrap_or(JsQueueWaitOptions {
		timeout_ms: None,
		completable: None,
	});

	Ok(QueueWaitOpts {
		timeout: timeout_duration(options.timeout_ms)?,
		signal: None,
		completable: options.completable.unwrap_or(false),
	})
}

fn enqueue_and_wait_opts(
	options: Option<JsQueueEnqueueAndWaitOptions>,
	signal: Option<&CancellationToken>,
) -> napi::Result<EnqueueAndWaitOpts> {
	let options = options.unwrap_or(JsQueueEnqueueAndWaitOptions {
		timeout_ms: None,
	});

	Ok(EnqueueAndWaitOpts {
		timeout: timeout_duration(options.timeout_ms)?,
		signal: signal.map(|signal| signal.inner().clone()),
	})
}

fn queue_try_next_opts(options: Option<JsQueueTryNextOptions>) -> QueueTryNextOpts {
	let options = options.unwrap_or(JsQueueTryNextOptions {
		names: None,
		completable: None,
	});

	QueueTryNextOpts {
		names: options.names,
		completable: options.completable.unwrap_or(false),
	}
}

fn queue_try_next_batch_opts(
	options: Option<JsQueueTryNextBatchOptions>,
) -> QueueTryNextBatchOpts {
	let options = options.unwrap_or(JsQueueTryNextBatchOptions {
		names: None,
		count: None,
		completable: None,
	});

	QueueTryNextBatchOpts {
		names: options.names,
		count: options.count.unwrap_or(1),
		completable: options.completable.unwrap_or(false),
	}
}

fn timeout_duration(timeout_ms: Option<i64>) -> napi::Result<Option<Duration>> {
	match timeout_ms {
		Some(timeout_ms) if timeout_ms < 0 => Err(napi::Error::from_reason(
			"queue timeout must be non-negative",
		)),
		Some(timeout_ms) => Ok(Some(Duration::from_millis(
			u64::try_from(timeout_ms)
				.map_err(|_| napi::Error::from_reason("queue timeout exceeds u64 range"))?,
		))),
		None => Ok(None),
	}
}
