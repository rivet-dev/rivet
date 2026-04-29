use std::sync::Arc;

use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;

use crate::vfs::{NativeDatabase, SqliteVfs, SqliteVfsMetrics, VfsConfig};

pub type NativeDatabaseHandle = NativeDatabase;

pub fn vfs_name_for_actor_database(actor_id: &str, generation: u64) -> String {
	format!("envoy-sqlite-{actor_id}-g{generation}")
}

pub fn open_database_from_envoy(
	handle: EnvoyHandle,
	actor_id: String,
	startup_data: Option<protocol::SqliteStartupData>,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	let startup =
		startup_data.ok_or_else(|| anyhow!("missing sqlite startup data for actor {actor_id}"))?;
	let vfs_name = vfs_name_for_actor_database(&actor_id, startup.generation);
	let vfs = SqliteVfs::register(
		&vfs_name,
		handle,
		actor_id.clone(),
		rt_handle,
		startup,
		VfsConfig::default(),
		metrics,
	)
	.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?;

	crate::vfs::open_database(vfs, &actor_id)
		.map_err(|e| anyhow!("failed to open sqlite database: {e}"))
}

#[cfg(test)]
mod tests {
	use super::vfs_name_for_actor_database;

	#[test]
	fn vfs_name_includes_actor_and_generation() {
		assert_eq!(
			vfs_name_for_actor_database("actor-123", 42),
			"envoy-sqlite-actor-123-g42"
		);
	}
}
