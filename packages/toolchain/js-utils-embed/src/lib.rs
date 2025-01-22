use anyhow::*;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;
use tokio::fs;

const JS_UTILS_DIST_PATH: Dir = include_dir!("$JS_UTILS_DIST_PATH");
const JS_UTILS_DIST_HASH: &'static str = env!("JS_UTILS_DIST_HASH");

/// Return a path for the source dir. If one does not exist, the source dir will automatically be
/// extracted and executables will be set.
pub async fn dist_path(data_dir: &PathBuf) -> Result<PathBuf> {
	// Create path to src based on hash
	let src_dir = data_dir.join("js-utils").join(JS_UTILS_DIST_HASH);

	// Write js-utils if does not exist
	if !src_dir.exists() {
		fs::create_dir_all(&src_dir).await?;
		tokio::task::block_in_place(|| JS_UTILS_DIST_PATH.extract(&src_dir))?;
	}

	Ok(src_dir)
}

