use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;

use crate::vfs::{NativeDatabase, SqliteVfs, VfsConfig};

pub type NativeDatabaseHandle = NativeDatabase;

pub fn open_database_from_envoy(
	handle: EnvoyHandle,
	actor_id: String,
	startup_data: Option<protocol::SqliteStartupData>,
	rt_handle: Handle,
) -> Result<NativeDatabaseHandle> {
	let startup = startup_data
		.ok_or_else(|| anyhow!("missing sqlite startup data for actor {actor_id}"))?;
	let vfs_name = format!("envoy-sqlite-{actor_id}");
	let vfs = SqliteVfs::register(
		&vfs_name,
		handle,
		actor_id.clone(),
		rt_handle,
		startup,
		VfsConfig::default(),
	)
	.map_err(|e| anyhow!("failed to register sqlite VFS: {e}"))?;

	crate::vfs::open_database(vfs, &actor_id)
		.map_err(|e| anyhow!("failed to open sqlite database: {e}"))
}
