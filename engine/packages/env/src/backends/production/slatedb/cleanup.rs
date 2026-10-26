//! Guarded object_store metadata updates and deletion for retired SlateDB namespaces.
use crate::MutationGuard;
use futures_util::{StreamExt, stream::BoxStream};
use object_store::{
	CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
	ObjectStoreExt, PutMultipartOptions, PutOptions, PutPayload, PutResult, Result, path::Path,
};
use std::{fmt, sync::Arc};

/// Deletion and checkpoint metadata updates recheck the admitted cleanup owner.
/// Already-sent work targets only an irrevocably retired database namespace.
pub(super) struct CleanupStore {
	store: Arc<dyn ObjectStore>,
	guard: Arc<dyn MutationGuard>,
}

impl CleanupStore {
	pub fn new(store: Arc<dyn ObjectStore>, guard: Arc<dyn MutationGuard>) -> Arc<Self> {
		Arc::new(Self { store, guard })
	}

	fn check(&self) -> Result<()> {
		if self.guard.may_send() {
			Ok(())
		} else {
			Err(denied("database cleanup authority revoked"))
		}
	}
}

fn denied(message: &'static str) -> object_store::Error {
	// An expired/restricted capability is permanent for this attempt. Generic
	// errors are retried indefinitely by upstream and would strand reconciliation.
	object_store::Error::NotSupported {
		source: std::io::Error::other(message).into(),
	}
}

impl fmt::Display for CleanupStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("guarded SlateDB cleanup")
	}
}

impl fmt::Debug for CleanupStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

#[async_trait::async_trait]
impl ObjectStore for CleanupStore {
	async fn put_opts(
		&self,
		path: &Path,
		payload: PutPayload,
		options: PutOptions,
	) -> Result<PutResult> {
		self.check()?;
		self.store.put_opts(path, payload, options).await
	}

	async fn put_multipart_opts(
		&self,
		_: &Path,
		_: PutMultipartOptions,
	) -> Result<Box<dyn MultipartUpload>> {
		Err(denied("database cleanup cannot create multipart objects"))
	}

	async fn get_opts(&self, path: &Path, options: GetOptions) -> Result<GetResult> {
		self.store.get_opts(path, options).await
	}

	fn delete_stream(
		&self,
		paths: BoxStream<'static, Result<Path>>,
	) -> BoxStream<'static, Result<Path>> {
		let store = self.store.clone();
		let guard = self.guard.clone();
		paths
			.then(move |path| {
				let store = store.clone();
				let guard = guard.clone();
				async move {
					let path = path?;
					if !guard.may_send() {
						return Err(denied("database cleanup authority revoked"));
					}
					store.delete(&path).await?;
					Ok(path)
				}
			})
			.boxed()
	}

	fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
		self.store.list(prefix)
	}

	async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
		self.store.list_with_delimiter(prefix).await
	}

	async fn copy_opts(&self, _: &Path, _: &Path, _: CopyOptions) -> Result<()> {
		Err(denied("database cleanup cannot copy objects"))
	}
}


