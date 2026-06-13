use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Credentials {
	pub rivet_cloud_token: String,
}

/// Resolves the Rivet Cloud token from, in order: the flag, the
/// `RIVET_CLOUD_TOKEN` env var, then `~/.rivet/credentials`.
pub fn resolve_token(flag: Option<&str>) -> Result<String> {
	if let Some(token) = flag {
		return Ok(token.to_string());
	}
	if let Ok(token) = env::var("RIVET_CLOUD_TOKEN") {
		if !token.trim().is_empty() {
			return Ok(token);
		}
	}
	let path = credentials_path()?;
	if path.exists() {
		let credentials: Credentials = serde_json::from_str(&fs::read_to_string(&path)?)?;
		if !credentials.rivet_cloud_token.trim().is_empty() {
			return Ok(credentials.rivet_cloud_token);
		}
	}
	bail!("missing Rivet Cloud token; pass --token or set RIVET_CLOUD_TOKEN")
}

pub fn write_credentials(token: &str) -> Result<()> {
	let path = credentials_path()?;
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}
	let contents = serde_json::to_string_pretty(&Credentials {
		rivet_cloud_token: token.to_string(),
	})?;
	write_secret_file(&path, contents.as_bytes())?;
	Ok(())
}

#[cfg(unix)]
fn write_secret_file(path: &std::path::Path, contents: &[u8]) -> Result<()> {
	use std::{
		fs::OpenOptions,
		io::Write,
		os::unix::fs::{OpenOptionsExt, PermissionsExt},
	};

	let mut file = OpenOptions::new()
		.create(true)
		.write(true)
		.truncate(true)
		.mode(0o600)
		.open(path)
		.with_context(|| format!("open {}", path.display()))?;
	file.write_all(contents)?;
	file.sync_all()?;
	let mut perms = file.metadata()?.permissions();
	perms.set_mode(0o600);
	fs::set_permissions(path, perms)?;
	Ok(())
}

#[cfg(not(unix))]
fn write_secret_file(path: &std::path::Path, contents: &[u8]) -> Result<()> {
	fs::write(path, contents)?;
	Ok(())
}

fn credentials_path() -> Result<PathBuf> {
	Ok(dirs::home_dir()
		.context("could not resolve home directory")?
		.join(".rivet")
		.join("credentials"))
}
