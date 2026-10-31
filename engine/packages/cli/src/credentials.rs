use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Credentials {
	/// Always serialized, even when empty, because older CLI versions require
	/// the field to parse the file.
	#[serde(default)]
	pub rivet_cloud_token: String,
	/// Login sessions keyed by Cloud API endpoint.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub sessions: BTreeMap<String, Session>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
	pub access_token: String,
	pub refresh_token: String,
	/// Access token expiry as epoch milliseconds.
	pub expires_at: i64,
	pub user: SessionUser,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionUser {
	pub id: String,
	pub email: String,
	#[serde(default)]
	pub name: String,
}

pub fn session_key(cloud_api: &str) -> String {
	cloud_api.trim_end_matches('/').to_string()
}

pub fn read_credentials() -> Result<Credentials> {
	let path = credentials_path()?;
	if !path.exists() {
		return Ok(Credentials::default());
	}
	serde_json::from_str(&fs::read_to_string(&path)?)
		.with_context(|| format!("parse {}", path.display()))
}

pub fn write_credentials(credentials: &Credentials) -> Result<()> {
	let path = credentials_path()?;
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent)?;
	}
	let contents = serde_json::to_string_pretty(credentials)?;
	write_secret_file(&path, contents.as_bytes())?;
	Ok(())
}

pub fn store_cloud_token(token: &str) -> Result<()> {
	let mut credentials = read_credentials()?;
	credentials.rivet_cloud_token = token.to_string();
	write_credentials(&credentials)
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
