use anyhow::*;
use sha1::{Digest, Sha1};
use std::{env, path::PathBuf};

/// Root of the current project.
pub fn project_root() -> Result<PathBuf> {
	Ok(env::current_dir()?)
}

/// Returns a unique hash to the current project's path.
pub fn project_path_hash() -> Result<String> {
	let project_root = project_root()?;

	// Build clean file name
	let file_name = project_root
		.file_name()
		.map(|name| name.to_string_lossy().to_lowercase())
		.unwrap_or_default()
		.replace(|c: char| !c.is_alphanumeric(), "_");

	// Hash the full path to ensure it's unique
	let mut hasher = Sha1::new();
	hasher.update(project_root.to_string_lossy().as_bytes());
	let hash = format!("{:.16x}", hasher.finalize());

	// Return a human-readable name
	Ok(format!("{}_{}", file_name, hash))
}

/// Where all data gets stored globally.
pub fn data_dir() -> Result<PathBuf> {
	Ok(dirs::data_dir().context("dirs::data_dir()")?.join("rivet"))
}

/// Global config data.
pub fn user_config_dir(base_data_dir: &PathBuf) -> Result<PathBuf> {
	Ok(base_data_dir.join("config"))
}

/// Directory specific to this project.
///
/// This is not stored within the project itself since it causes problems with version control &
/// bugs in WSL.
pub fn project_data_dir(base_data_dir: &PathBuf) -> Result<PathBuf> {
	Ok(base_data_dir.join("projects").join(project_path_hash()?))
}

/// Stores all meta.
pub fn meta_config_file(base_data_dir: &PathBuf) -> Result<PathBuf> {
	Ok(project_data_dir(base_data_dir)?.join("meta.json"))
}
