//! In-memory reconciler storage for examples and tests.
//!
//! Depot partition storage is not available in this build, so examples run against a
//! process-local store with the same contract: point reads plus atomic conditional
//! read-modify-write of one record.
use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use bytes::Bytes;
use parking_lot::Mutex;
// Each including module brings `DurableStore` into scope, from the library or the crate under test.
use super::DurableStore;
use rivet_keys::keys::codec::{FormalKey, ValueCodec};

type Records = BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>>;

/// Optimistic conditional writes, like Depot: each attempt reads a snapshot, runs the
/// closure without holding the lock, and writes only if the record is still unchanged.
#[derive(Clone, Default)]
pub struct MemoryStore {
	records: Arc<Mutex<Records>>,
}

/// Attempts per mutation before reporting a conflict, matching Depot's client.
const MUTATE_ATTEMPTS: usize = 3;

impl DurableStore for MemoryStore {
	async fn kv_get(&self, partition: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>> {
		let records = self.records.lock();
		Ok(records.get(&(partition.to_vec(), key.to_vec())).cloned())
	}

	async fn kv_mutate<K, R>(
		&self,
		partition: &[u8],
		key: &K,
		mutate: impl Fn(&mut Option<<K::Codec as ValueCodec>::Value>) -> Result<R> + Send + Sync,
	) -> Result<R>
	where
		K: FormalKey<Packed = Bytes> + Sync,
		K::Codec: ValueCodec<Input = <K::Codec as ValueCodec>::Value>,
		<K::Codec as ValueCodec>::Value: Send,
		R: Send,
	{
		let id = (partition.to_vec(), key.pack().to_vec());
		for _ in 0..MUTATE_ATTEMPTS {
			let snapshot = self.records.lock().get(&id).cloned();
			let mut value = snapshot
				.clone()
				.map(|raw| key.deserialize(Bytes::from(raw)))
				.transpose()?;
			let result = mutate(&mut value)?;
			let encoded = value
				.map(|value| key.serialize(value).map(|bytes| bytes.to_vec()))
				.transpose()?;

			// Even an unchanged value goes through the conditional write.
			let mut records = self.records.lock();
			if records.get(&id) != snapshot.as_ref() {
				continue;
			}
			match encoded {
				Some(encoded) => {
					records.insert(id.clone(), encoded);
				}
				None => {
					records.remove(&id);
				}
			}

			return Ok(result);
		}

		anyhow::bail!("memory store mutation conflicted after {MUTATE_ATTEMPTS} attempts")
	}
}

pub struct TestCluster {
	pub client: MemoryStore,
}

impl TestCluster {
	pub async fn start() -> Result<Self> {
		Ok(Self {
			client: MemoryStore::default(),
		})
	}

	pub async fn close(self) -> Result<()> {
		Ok(())
	}
}
