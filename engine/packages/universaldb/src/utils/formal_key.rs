use anyhow::Result;

use crate::value::Value;

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
	fn combine(&self, chunks: Vec<Value>) -> Result<Self::Value>;

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>>;
}
