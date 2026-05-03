use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;

use super::{ColdTier, ColdTierObjectMetadata};

#[derive(Debug, Clone)]
pub struct FilesystemColdTier {
	root: PathBuf,
}

impl FilesystemColdTier {
	pub fn new(root: impl Into<PathBuf>) -> Self {
		FilesystemColdTier { root: root.into() }
	}

	pub fn root(&self) -> &Path {
		&self.root
	}

	fn object_path(&self, key: &str) -> Result<PathBuf> {
		let path = Path::new(key);

		if key.is_empty() {
			anyhow::bail!("cold-tier object key must not be empty");
		}

		for component in path.components() {
			match component {
				Component::Normal(_) => {}
				Component::CurDir
				| Component::ParentDir
				| Component::RootDir
				| Component::Prefix(_) => {
					anyhow::bail!("invalid cold-tier object key: {key}");
				}
			}
		}

		Ok(self.root.join(path))
	}

	async fn list_dir(&self, dir: PathBuf) -> Result<Vec<ColdTierObjectMetadata>> {
		let mut out = Vec::new();
		let mut pending = vec![dir];

		while let Some(dir) = pending.pop() {
			let mut entries = match tokio::fs::read_dir(&dir).await {
				Ok(entries) => entries,
				Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
				Err(err) => {
					return Err(err)
						.with_context(|| format!("list cold-tier dir {}", dir.display()));
				}
			};

			while let Some(entry) = entries
				.next_entry()
				.await
				.with_context(|| format!("list cold-tier dir {}", dir.display()))?
			{
				let metadata = entry.metadata().await.with_context(|| {
					format!("read cold-tier metadata {}", entry.path().display())
				})?;

				if metadata.is_dir() {
					pending.push(entry.path());
				} else if metadata.is_file() {
					let key = entry
						.path()
						.strip_prefix(&self.root)
						.with_context(|| {
							format!(
								"cold-tier path {} escaped root {}",
								entry.path().display(),
								self.root.display()
							)
						})?
						.to_string_lossy()
						.replace('\\', "/");

					out.push(ColdTierObjectMetadata {
						key,
						size_bytes: metadata.len(),
					});
				}
			}
		}

		Ok(out)
	}
}

#[async_trait]
impl ColdTier for FilesystemColdTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		let path = self.object_path(key)?;
		if let Some(parent) = path.parent() {
			tokio::fs::create_dir_all(parent)
				.await
				.with_context(|| format!("create cold-tier dir {}", parent.display()))?;
		}

		let mut file = tokio::fs::File::create(&path)
			.await
			.with_context(|| format!("create cold-tier object {}", path.display()))?;
		file.write_all(bytes)
			.await
			.with_context(|| format!("write cold-tier object {}", path.display()))?;
		file.flush()
			.await
			.with_context(|| format!("flush cold-tier object {}", path.display()))?;

		Ok(())
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		let path = self.object_path(key)?;
		match tokio::fs::read(&path).await {
			Ok(bytes) => Ok(Some(bytes)),
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
			Err(err) => {
				Err(err).with_context(|| format!("read cold-tier object {}", path.display()))
			}
		}
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		for key in keys {
			let path = self.object_path(key)?;
			match tokio::fs::remove_file(&path).await {
				Ok(()) => {}
				Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
				Err(err) => {
					return Err(err)
						.with_context(|| format!("delete cold-tier object {}", path.display()));
				}
			}
		}

		Ok(())
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		let start = if prefix.is_empty() {
			self.root.clone()
		} else {
			self.object_path(prefix)?
		};

		let mut objects = self.list_dir(start).await?;
		objects.sort_by(|a, b| a.key.cmp(&b.key));

		Ok(objects)
	}
}

pub(super) fn validate_object_key(key: &str) -> Result<()> {
	FilesystemColdTier::new(PathBuf::new()).object_path(key)?;
	Ok(())
}
