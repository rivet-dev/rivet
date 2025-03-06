// TODO: Use concrete error types
use anyhow::*;
use foundationdb::future::FdbValue;

pub mod signal;
pub mod wake;
pub mod workflow;

pub trait FormalKey {
	type Value;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value>;

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>>;
}

pub trait FormalChunkedKey {
	type Value;
	type ChunkKey;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey;

	/// Assumes chunks are in order.
	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value>;

	// fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>>;
}
