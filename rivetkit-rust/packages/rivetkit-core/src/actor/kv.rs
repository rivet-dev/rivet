



use anyhow::Result;

use rivet_envoy_client::handle::EnvoyHandle;

use crate::types::ListOpts;

/// Narrow access to the actor's pre-SQLite KV namespace.
///
/// Production code may use this only for the one-time SQLite importer and the
/// temporary inspector-token mirror. It intentionally has no default or
/// unconfigured state. The in-memory backend exists only in unit tests.
#[derive(Clone)]
pub(crate) struct LegacyActorKv {
	backend: LegacyActorKvBackend,
}

#[derive(Clone)]
enum LegacyActorKvBackend {
	Envoy {
		handle: EnvoyHandle,
		actor_id: String,
	},
	
}



impl LegacyActorKv {
	pub(crate) fn new(handle: EnvoyHandle, actor_id: impl Into<String>) -> Self {
		Self {
			backend: LegacyActorKvBackend::Envoy {
				handle,
				actor_id: actor_id.into(),
			},
		}
	}

	

	

	pub(crate) async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
		self.batch_put(&[(key, value)]).await
	}

	pub(crate) async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_get(
						actor_id.clone(),
						keys.iter().map(|key| key.to_vec()).collect(),
					)
					.await
			}
			
		}
	}

	pub(crate) async fn batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_put(
						actor_id.clone(),
						entries
							.iter()
							.map(|(key, value)| (key.to_vec(), value.to_vec()))
							.collect(),
					)
					.await
			}
			
		}
	}

	pub(crate) async fn list_prefix(
		&self,
		prefix: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_list_prefix(
						actor_id.clone(),
						prefix.to_vec(),
						Some(opts.reverse),
						opts.limit.map(u64::from),
					)
					.await
			}
			
		}
	}

	pub(crate) async fn list_range(
		&self,
		start: &[u8],
		end: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_list_range(
						actor_id.clone(),
						start.to_vec(),
						end.to_vec(),
						true,
						Some(opts.reverse),
						opts.limit.map(u64::from),
					)
					.await
			}
			
		}
	}

	

	

	

	

	

	

	

	

	

	
}








