use anyhow::*;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;
use tokio::fs;

const ACTOR_SDK_DIST_DIR: Dir = include_dir!("$ACTOR_SDK_DIST_PATH");
const ACTOR_SDK_DIST_HASH: &'static str = env!("ACTOR_SDK_DIST_HASH");

/// Return a path for the source dir. If one does not exist, the source dir will automatically be
/// extracted and executables will be set.
pub async fn dist_path(data_dir: &PathBuf) -> Result<PathBuf> {
	// Create path to dist based on hash
	let dist_dir = data_dir.join("actor-sdk").join(ACTOR_SDK_DIST_HASH);

	// Write actor-sdk if does not exist
	if !dist_dir.exists() {
		fs::create_dir_all(&dist_dir).await?;
		tokio::task::block_in_place(|| ACTOR_SDK_DIST_DIR.extract(&dist_dir))?;
	}

	Ok(dist_dir)
}
