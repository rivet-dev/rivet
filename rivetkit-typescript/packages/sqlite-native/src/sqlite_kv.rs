//! Transport-agnostic KV trait for the native SQLite VFS.
//!
//! Implementations provide the backing KV storage that the native SQLite VFS
//! reads and writes chunks through. The trait is object-safe and async so it
//! can be implemented over any transport (WebSocket channel, in-process engine,
//! etc.).

use std::fmt;

use async_trait::async_trait;

// MARK: Error

/// Error type for SqliteKv operations.
#[derive(Debug)]
pub struct SqliteKvError {
	message: String,
}

impl SqliteKvError {
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}
}

impl fmt::Display for SqliteKvError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl std::error::Error for SqliteKvError {}

impl From<String> for SqliteKvError {
	fn from(message: String) -> Self {
		Self { message }
	}
}

impl From<&str> for SqliteKvError {
	fn from(message: &str) -> Self {
		Self {
			message: message.to_string(),
		}
	}
}

// MARK: Get result

/// Result of a batch get operation.
///
/// `keys` and `values` are parallel lists. Only keys that exist in the store
/// are returned; missing keys are omitted.
#[derive(Debug)]
pub struct KvGetResult {
	pub keys: Vec<Vec<u8>>,
	pub values: Vec<Vec<u8>>,
}

// MARK: Trait

/// Transport-agnostic KV trait consumed by the native SQLite VFS.
///
/// All methods receive an `actor_id` to scope operations to a specific actor's
/// KV namespace. Implementations are free to ignore it if scoping is handled
/// at a higher level.
#[async_trait]
pub trait SqliteKv: Send + Sync {
	/// Called when a KV operation fails inside a VFS callback before the
	/// original error is collapsed into a generic SQLite IO error code.
	fn on_error(&self, _actor_id: &str, _error: &SqliteKvError) {}

	/// Called when an actor's database is opened.
	async fn on_open(&self, _actor_id: &str) -> Result<(), SqliteKvError> {
		Ok(())
	}

	/// Called when an actor's database is closed.
	async fn on_close(&self, _actor_id: &str) -> Result<(), SqliteKvError> {
		Ok(())
	}

	/// Fetch multiple keys in one batch.
	///
	/// Only existing keys are returned in the result. Missing keys are omitted.
	async fn batch_get(
		&self,
		actor_id: &str,
		keys: Vec<Vec<u8>>,
	) -> Result<KvGetResult, SqliteKvError>;

	/// Write multiple key-value pairs in one batch.
	///
	/// `keys` and `values` must have the same length.
	async fn batch_put(
		&self,
		actor_id: &str,
		keys: Vec<Vec<u8>>,
		values: Vec<Vec<u8>>,
	) -> Result<(), SqliteKvError>;

	/// Delete multiple keys in one batch.
	async fn batch_delete(&self, actor_id: &str, keys: Vec<Vec<u8>>) -> Result<(), SqliteKvError>;

	/// Delete all keys in the half-open range `[start, end)`.
	async fn delete_range(
		&self,
		actor_id: &str,
		start: Vec<u8>,
		end: Vec<u8>,
	) -> Result<(), SqliteKvError>;
}
