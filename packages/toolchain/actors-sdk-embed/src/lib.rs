use anyhow::*;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;
use tokio::fs;

const ACTOR_SDK_DIR: Dir = include_dir!("$ACTOR_SDK_PATH");
const ACTOR_SDK_HASH: &'static str = env!("ACTOR_SDK_HASH");

/// Return a path for the source dir. If one does not exist, the source dir will automatically be
/// extracted and executables will be set.
pub async fn src_path(data_dir: &PathBuf) -> Result<PathBuf> {
	// Create path to src based on hash
	let src_dir = data_dir.join("actor-sdk").join(ACTOR_SDK_HASH);

	// Write actor-sdk if does not exist
	if !src_dir.exists() {
		fs::create_dir_all(&src_dir).await?;
		tokio::task::block_in_place(|| ACTOR_SDK_DIR.extract(&src_dir))?;
	}

	Ok(src_dir)
}
