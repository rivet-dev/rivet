//! HTTP response bodies written by the host. Backpressure comes from the bounded
//! channel core reads from: a write completes once core has room for it.

use anyhow::{Result, anyhow};
use parking_lot::Mutex;
use rivetkit_core::{HTTP_BODY_MAX_CHUNK_SIZE, ResponseChunk};
use tokio::sync::mpsc;

/// The writable half of a streaming HTTP response.
pub struct ResponseBody {
	/// Taken by `end` or `error`.
	tx: Mutex<Option<mpsc::Sender<ResponseChunk>>>,
	/// Kept to observe the reader going away after the body ends.
	closed: mpsc::Sender<ResponseChunk>,
}

impl ResponseBody {
	pub(crate) fn new(tx: mpsc::Sender<ResponseChunk>) -> Self {
		Self {
			tx: Mutex::new(Some(tx.clone())),
			closed: tx,
		}
	}

	fn sender(&self) -> Result<mpsc::Sender<ResponseChunk>> {
		self.tx
			.lock()
			.clone()
			.ok_or_else(|| anyhow!("http response body is already closed"))
	}

	/// Writes `data`, split into chunks core accepts. Returns once every chunk is
	/// queued.
	pub async fn write(&self, data: &[u8]) -> Result<()> {
		let tx = self.sender()?;
		for chunk in data.chunks(HTTP_BODY_MAX_CHUNK_SIZE) {
			tx.send(ResponseChunk::Data {
				data: chunk.to_vec(),
				finish: false,
			})
			.await
			.map_err(|_| anyhow!("http response body reader is gone"))?;
		}
		Ok(())
	}

	/// Ends the body.
	pub async fn end(&self) -> Result<()> {
		let tx = self
			.tx
			.lock()
			.take()
			.ok_or_else(|| anyhow!("http response body is already closed"))?;
		tx.send(ResponseChunk::Data {
			data: Vec::new(),
			finish: true,
		})
		.await
		.map_err(|_| anyhow!("http response body reader is gone"))
	}

	/// Aborts the body; the client sees a broken response.
	pub async fn error(&self, message: String) -> Result<()> {
		let Some(tx) = self.tx.lock().take() else {
			return Ok(());
		};
		let _ = tx.send(ResponseChunk::Error(message)).await;
		Ok(())
	}

	/// Resolves when the client stops reading.
	pub async fn cancelled(&self) {
		self.closed.closed().await;
	}
}
