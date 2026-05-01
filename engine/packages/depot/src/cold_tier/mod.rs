use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use crate::compactor::metrics;

const UNKNOWN_NODE_ID: &str = "unknown";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdTierObjectMetadata {
	pub key: String,
	pub size_bytes: u64,
}

#[async_trait]
pub trait ColdTier: Send + Sync + 'static {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()>;

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>>;

	async fn delete_objects(&self, keys: &[String]) -> Result<()>;

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>>;
}

#[derive(Debug, Clone, Default)]
pub struct DisabledColdTier;

#[async_trait]
impl ColdTier for DisabledColdTier {
	async fn put_object(&self, _key: &str, _bytes: &[u8]) -> Result<()> {
		anyhow::bail!("sqlite cold tier is disabled")
	}

	async fn get_object(&self, _key: &str) -> Result<Option<Vec<u8>>> {
		anyhow::bail!("sqlite cold tier is disabled")
	}

	async fn delete_objects(&self, _keys: &[String]) -> Result<()> {
		anyhow::bail!("sqlite cold tier is disabled")
	}

	async fn list_prefix(&self, _prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		anyhow::bail!("sqlite cold tier is disabled")
	}
}

#[async_trait]
impl<T> ColdTier for Arc<T>
where
	T: ColdTier,
{
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		self.as_ref().put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.as_ref().get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.as_ref().delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.as_ref().list_prefix(prefix).await
	}
}

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
					return Err(err).with_context(|| format!("list cold-tier dir {}", dir.display()));
				}
			};

			while let Some(entry) = entries
				.next_entry()
				.await
				.with_context(|| format!("list cold-tier dir {}", dir.display()))?
			{
				let metadata = entry
					.metadata()
					.await
					.with_context(|| format!("read cold-tier metadata {}", entry.path().display()))?;

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
			Err(err) => Err(err).with_context(|| format!("read cold-tier object {}", path.display())),
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

#[derive(Debug, Clone)]
pub struct S3ColdTier {
	client: aws_sdk_s3::Client,
	bucket: String,
	root_prefix: String,
}

impl S3ColdTier {
	pub fn new(
		client: aws_sdk_s3::Client,
		bucket: impl Into<String>,
		root_prefix: impl Into<String>,
	) -> Self {
		S3ColdTier {
			client,
			bucket: bucket.into(),
			root_prefix: normalize_prefix(root_prefix.into()),
		}
	}

	pub async fn from_env(
		bucket: impl Into<String>,
		root_prefix: impl Into<String>,
		endpoint_url: Option<String>,
	) -> Result<Self> {
		let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

		if let Some(endpoint_url) = endpoint_url {
			loader = loader.endpoint_url(endpoint_url);
		}

		let config = loader.load().await;
		Ok(S3ColdTier::new(
			aws_sdk_s3::Client::new(&config),
			bucket,
			root_prefix,
		))
	}

	fn s3_key(&self, key: &str) -> Result<String> {
		validate_object_key(key)?;

		if self.root_prefix.is_empty() {
			Ok(key.to_string())
		} else {
			Ok(format!("{}/{}", self.root_prefix, key))
		}
	}

	fn strip_root_prefix(&self, key: &str) -> Option<String> {
		if self.root_prefix.is_empty() {
			Some(key.to_string())
		} else {
			key.strip_prefix(&format!("{}/", self.root_prefix))
				.map(str::to_string)
		}
	}
}

#[async_trait]
impl ColdTier for S3ColdTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		let key = self.s3_key(key)?;
		self.client
			.put_object()
			.bucket(&self.bucket)
			.key(&key)
			.body(ByteStream::from(bytes.to_vec()))
			.send()
			.await
			.with_context(|| format!("put cold-tier S3 object {key}"))?;

		Ok(())
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		let key = self.s3_key(key)?;
		let output = match self
			.client
			.get_object()
			.bucket(&self.bucket)
			.key(&key)
			.send()
			.await
		{
			Ok(output) => output,
			Err(err) if err.to_string().contains("NoSuchKey") => return Ok(None),
			Err(err) => return Err(err).with_context(|| format!("get cold-tier S3 object {key}")),
		};

		let bytes = output
			.body
			.collect()
			.await
			.with_context(|| format!("read cold-tier S3 object body {key}"))?
			.into_bytes()
			.to_vec();

		Ok(Some(bytes))
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		for chunk in keys.chunks(1000) {
			if chunk.is_empty() {
				continue;
			}

			let mut objects = Vec::with_capacity(chunk.len());
			for key in chunk {
				objects.push(
					ObjectIdentifier::builder()
						.key(self.s3_key(key)?)
						.build()
						.context("build cold-tier S3 delete object identifier")?,
				);
			}

			self.client
				.delete_objects()
				.bucket(&self.bucket)
				.delete(
					Delete::builder()
						.set_objects(Some(objects))
						.build()
						.context("build cold-tier S3 delete request")?,
				)
				.send()
				.await
				.context("delete cold-tier S3 objects")?;
		}

		Ok(())
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		let prefix = if prefix.is_empty() {
			self.root_prefix.clone()
		} else {
			self.s3_key(prefix)?
		};
		let mut continuation_token = None;
		let mut objects = Vec::new();

		loop {
			let output = self
				.client
				.list_objects_v2()
				.bucket(&self.bucket)
				.prefix(&prefix)
				.set_continuation_token(continuation_token)
				.send()
				.await
				.with_context(|| format!("list cold-tier S3 prefix {prefix}"))?;

			for object in output.contents() {
				if let Some(key) = object.key() {
					if let Some(key) = self.strip_root_prefix(key) {
						objects.push(ColdTierObjectMetadata {
							key,
							size_bytes: object.size().unwrap_or_default() as u64,
						});
					}
				}
			}

			if output.is_truncated().unwrap_or(false) {
				continuation_token = output.next_continuation_token().map(str::to_string);
			} else {
				break;
			}
		}

		objects.sort_by(|a, b| a.key.cmp(&b.key));

		Ok(objects)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColdTierOperation {
	Put,
	Get,
	Delete,
	List,
}

#[derive(Debug)]
pub struct FaultyColdTier<T> {
	inner: T,
	state: Arc<FaultyColdTierState>,
}

#[derive(Debug, Default)]
struct FaultyColdTierState {
	latency_ms: AtomicU64,
	fail_puts: AtomicBool,
	fail_gets: AtomicBool,
	fail_deletes: AtomicBool,
	fail_lists: AtomicBool,
	fail_next_operations: AtomicUsize,
}

impl<T> FaultyColdTier<T> {
	pub fn new(inner: T) -> Self {
		FaultyColdTier {
			inner,
			state: Arc::new(FaultyColdTierState::default()),
		}
	}

	pub fn set_latency(&self, latency: Duration) {
		self.state
			.latency_ms
			.store(latency.as_millis() as u64, Ordering::SeqCst);
	}

	pub fn fail_operation(&self, operation: ColdTierOperation, enabled: bool) {
		let flag = match operation {
			ColdTierOperation::Put => &self.state.fail_puts,
			ColdTierOperation::Get => &self.state.fail_gets,
			ColdTierOperation::Delete => &self.state.fail_deletes,
			ColdTierOperation::List => &self.state.fail_lists,
		};
		flag.store(enabled, Ordering::SeqCst);
	}

	pub fn fail_next_operations(&self, count: usize) {
		self.state
			.fail_next_operations
			.store(count, Ordering::SeqCst);
	}

	async fn maybe_fail(&self, operation: ColdTierOperation) -> Result<()> {
		let latency_ms = self.state.latency_ms.load(Ordering::SeqCst);
		if latency_ms > 0 {
			tokio::time::sleep(Duration::from_millis(latency_ms)).await;
		}

		let fail_by_operation = match operation {
			ColdTierOperation::Put => self.state.fail_puts.load(Ordering::SeqCst),
			ColdTierOperation::Get => self.state.fail_gets.load(Ordering::SeqCst),
			ColdTierOperation::Delete => self.state.fail_deletes.load(Ordering::SeqCst),
			ColdTierOperation::List => self.state.fail_lists.load(Ordering::SeqCst),
		};

		let fail_next = self
			.state
			.fail_next_operations
			.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
				(remaining > 0).then_some(remaining - 1)
			})
			.is_ok();

		if fail_by_operation || fail_next {
			metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
				.with_label_values(&[UNKNOWN_NODE_ID, operation.as_label()])
				.inc();
			anyhow::bail!("injected cold-tier failure for {operation:?}");
		}

		Ok(())
	}
}

impl ColdTierOperation {
	fn as_label(self) -> &'static str {
		match self {
			ColdTierOperation::Put => "put",
			ColdTierOperation::Get => "get",
			ColdTierOperation::Delete => "delete",
			ColdTierOperation::List => "list",
		}
	}
}

impl<T> Clone for FaultyColdTier<T>
where
	T: Clone,
{
	fn clone(&self) -> Self {
		FaultyColdTier {
			inner: self.inner.clone(),
			state: self.state.clone(),
		}
	}
}

#[async_trait]
impl<T> ColdTier for FaultyColdTier<T>
where
	T: ColdTier,
{
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		self.maybe_fail(ColdTierOperation::Put).await?;
		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.maybe_fail(ColdTierOperation::Get).await?;
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.maybe_fail(ColdTierOperation::Delete).await?;
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.maybe_fail(ColdTierOperation::List).await?;
		self.inner.list_prefix(prefix).await
	}
}

fn normalize_prefix(prefix: String) -> String {
	prefix
		.trim_matches('/')
		.split('/')
		.filter(|part| !part.is_empty())
		.collect::<Vec<_>>()
		.join("/")
}

fn validate_object_key(key: &str) -> Result<()> {
	FilesystemColdTier::new(PathBuf::new()).object_path(key)?;
	Ok(())
}
