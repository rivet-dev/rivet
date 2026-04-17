use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;

use crate::types::ListOpts;

#[derive(Clone, Default)]
pub struct Kv {
	handle: Option<EnvoyHandle>,
	actor_id: String,
}

impl Kv {
	pub fn new(handle: EnvoyHandle, actor_id: impl Into<String>) -> Self {
		Self {
			handle: Some(handle),
			actor_id: actor_id.into(),
		}
	}

	pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		let mut values = self.batch_get(&[key]).await?;
		Ok(values.pop().flatten())
	}

	pub async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
		self.batch_put(&[(key, value)]).await
	}

	pub async fn delete(&self, key: &[u8]) -> Result<()> {
		self.batch_delete(&[key]).await
	}

	pub async fn delete_range(&self, start: &[u8], end: &[u8]) -> Result<()> {
		let handle = self.handle()?;
		handle
			.kv_delete_range(
				self.actor_id.clone(),
				start.to_vec(),
				end.to_vec(),
			)
			.await
	}

	pub async fn list_prefix(&self, prefix: &[u8], opts: ListOpts) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let handle = self.handle()?;
		handle
			.kv_list_prefix(
				self.actor_id.clone(),
				prefix.to_vec(),
				Some(opts.reverse),
				opts.limit.map(u64::from),
			)
			.await
	}

	pub async fn list_range(
		&self,
		start: &[u8],
		end: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let handle = self.handle()?;
		handle
			.kv_list_range(
				self.actor_id.clone(),
				start.to_vec(),
				end.to_vec(),
				false,
				Some(opts.reverse),
				opts.limit.map(u64::from),
			)
			.await
	}

	pub async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>> {
		let handle = self.handle()?;
		handle
			.kv_get(
				self.actor_id.clone(),
				keys.iter().map(|key| key.to_vec()).collect(),
			)
			.await
	}

	pub async fn batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()> {
		let handle = self.handle()?;
		handle
			.kv_put(
				self.actor_id.clone(),
				entries
					.iter()
					.map(|(key, value)| (key.to_vec(), value.to_vec()))
					.collect(),
			)
			.await
	}

	pub async fn batch_delete(&self, keys: &[&[u8]]) -> Result<()> {
		let handle = self.handle()?;
		handle
			.kv_delete(
				self.actor_id.clone(),
				keys.iter().map(|key| key.to_vec()).collect(),
			)
			.await
	}

	fn handle(&self) -> Result<EnvoyHandle> {
		self.handle
			.clone()
			.ok_or_else(|| anyhow!("kv handle is not configured"))
	}
}

impl std::fmt::Debug for Kv {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Kv")
			.field("configured", &self.handle.is_some())
			.field("actor_id", &self.actor_id)
			.finish()
	}
}
