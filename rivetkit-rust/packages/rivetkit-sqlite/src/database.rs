use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;

use crate::vfs::{NativeDatabase, SqliteVfs, VfsConfig};

pub type NativeDatabaseHandle = NativeDatabase;

pub fn open_database_from_envoy(
	handle: EnvoyHandle,
	actor_id: String,
	rt_handle: Handle,
) -> Result<NativeDatabaseHandle> {
	let vfs_name = format!("envoy-sqlite-{actor_id}");
	let vfs = SqliteVfs::register(
		&vfs_name,
		handle,
		actor_id.clone(),
		rt_handle,
		VfsConfig::default(),
	)
	.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?;

	crate::vfs::open_database(vfs, &actor_id)
		.map_err(|e| anyhow!("failed to open sqlite database: {e}"))
}
