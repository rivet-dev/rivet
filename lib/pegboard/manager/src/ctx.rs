use anyhow::*;

use std::path::{Path, PathBuf};
use tokio::fs;
use url::Url;
use uuid::Uuid;

use crate::utils;

pub struct Ctx {
	path: PathBuf,
}

impl Ctx {
	pub fn new() -> Self {
		Ctx {
			path: Path::new("/etc/pegboard").to_path_buf(),
		}
	}

	pub async fn fetch_container_runner(
		&self,
		container_runner_binary_url: &str,
	) -> Result<PathBuf> {
		let url = Url::parse(container_runner_binary_url)?;
		let path_stub = utils::get_s3_path_stub(&url, true)?;
		let path = self.runner_binaries_path().join(path_stub);

		if !fs::metadata(&path).await.is_ok() {
			utils::download_file(container_runner_binary_url, &path).await?;
		}

		Ok(path.to_path_buf())
	}
}

impl Ctx {
	pub fn working_path(&self) -> &Path {
		self.path.as_path()
	}

	pub fn container_path(&self, container_id: Uuid) -> PathBuf {
		self.working_path()
			.join("containers")
			.join(container_id.to_string())
	}

	pub fn runner_binaries_path(&self) -> PathBuf {
		self.working_path().join("bin")
	}
}
