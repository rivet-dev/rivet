use anyhow::*;
use universaldb::{self as udb, future::FdbValue};

pub trait FormalKey {
	type Value;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value>;

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>>;

	fn read(&self, value: &[u8]) -> std::result::Result<Self::Value, udb::FdbBindingError> {
		self.deserialize(value)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))
	}
}

pub trait FormalChunkedKey {
	type Value;
	type ChunkKey;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey;

	/// Assumes chunks are in order.
	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value>;

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>>;
}
