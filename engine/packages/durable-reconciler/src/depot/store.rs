//! Storage used by [`super::DepotStateMachine`].
//!
//! The state machine needs only a point read and an optimistic read-modify-write of one typed
//! record. Keeping that behind this trait lets the reconciler run on any Depot client that
//! offers those two operations with the same conditional-write semantics.

use anyhow::Result;
use bytes::Bytes;
use rivet_keys::keys::codec::{FormalKey, ValueCodec};
use std::future::Future;

/// Durable partitioned key-value storage for reconciler records.
pub trait DurableStore: Clone + Send + Sync + 'static {
	/// Reads the current value of a key, or `None` when absent.
	fn kv_get(
		&self,
		partition: &[u8],
		key: &[u8],
	) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;

	/// Atomically replaces one typed record using a conditional write.
	///
	/// The closure may run once per attempt, so it must be deterministic and free of side
	/// effects. Setting the record to `None` deletes it. Exhausting conflict retries and unknown
	/// write outcomes must both return errors.
	fn kv_mutate<K, R>(
		&self,
		partition: &[u8],
		key: &K,
		mutate: impl Fn(&mut Option<<K::Codec as ValueCodec>::Value>) -> Result<R> + Send + Sync,
	) -> impl Future<Output = Result<R>> + Send
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Input = <K::Codec as ValueCodec>::Value>,
		<K::Codec as ValueCodec>::Value: Send,
		R: Send;
}

impl DurableStore for depot_worker::command::KvHandle {
	async fn kv_get(&self, _partition: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>> {
		Ok(self
			.snapshot()
			.await?
			.get(key)
			.await?
			.map(|value| value.to_vec()))
	}

	async fn kv_mutate<K, R>(
		&self,
		_partition: &[u8],
		key: &K,
		mutate: impl Fn(&mut Option<<K::Codec as ValueCodec>::Value>) -> Result<R> + Send + Sync,
	) -> Result<R>
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Input = <K::Codec as ValueCodec>::Value>,
		<K::Codec as ValueCodec>::Value: Send,
		R: Send,
	{
		loop {
			let mut transaction = self.begin().await?;
			let raw = transaction.get(&key.pack()).await?;
			let mut value = raw.map(|value| key.deserialize(value.into())).transpose()?;
			let result = mutate(&mut value)?;
			match value {
				Some(value) => transaction.put(&key.pack(), &key.serialize(value)?)?,
				None => transaction.delete(&key.pack())?,
			}
			match self.commit(transaction).await {
				Ok(_) => return Ok(result),
				Err(
					depot_client::filesystem::Error::Conflict
					| depot_client::filesystem::Error::TooOld,
				) => continue,
				Err(error) => return Err(error.into()),
			}
		}
	}
}
