use anyhow::*;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;
use tokio::fs;

const JS_UTILS_DIR: Dir = include_dir!("$JS_UTILS_PATH");
const JS_UTILS_HASH: &'static str = env!("JS_UTILS_HASH");

/// Return a path for the source dir. If one does not exist, the source dir will automatically be
/// extracted and executables will be set.
pub async fn src_path(data_dir: &PathBuf) -> Result<PathBuf> {
	// Create path to src based on hash
	let src_dir = data_dir.join("js-utils").join(JS_UTILS_HASH);

	// Write js-utils if does not exist
	if !src_dir.exists() {
		fs::create_dir_all(&src_dir).await?;
		tokio::task::block_in_place(|| JS_UTILS_DIR.extract(&src_dir))?;

		// Update executables
		#[cfg(unix)]
		set_executables(&JS_UTILS_DIR, &src_dir).await?;
	}

	Ok(src_dir)
}

/// HACK: Make all binaries in `bin` folders executable. This is because
/// bundling the vendored folders strips permissions, so executables can't be ran.
#[cfg(unix)]
async fn set_executables(dir: &Dir<'_>, fs_path: &PathBuf) -> Result<()> {
	use include_dir::DirEntry;
	use std::os::unix::fs::PermissionsExt;

	for entry in dir.entries() {
		match entry {
			DirEntry::Dir(subdir) => {
				let file_name = subdir.path().file_name().unwrap_or_default();
				if file_name == "bin" || file_name == ".bin" {
					for file_entry in subdir.files() {
						let file_path = fs_path.join(file_entry.path());
						let metadata = fs::metadata(&file_path).await?;
						let mut perms = metadata.permissions();
						perms.set_mode(perms.mode() | 0o111);
						fs::set_permissions(file_path, perms).await?;
					}
				}

				Box::pin(set_executables(subdir, &fs_path)).await?;
			}
			DirEntry::File(_) => {} // Skip files at this level
		}
	}
	Ok(())
}
