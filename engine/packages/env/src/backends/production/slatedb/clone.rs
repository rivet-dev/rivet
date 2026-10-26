//! Guarded object_store operations for SlateDB projected-clone initialization.
use crate::MutationGuard;
use bytes::Bytes;
use futures_util::{StreamExt, stream::BoxStream};
use object_store::{
	CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
	ObjectStoreExt, PutMode, PutMultipartOptions, PutOptions, PutPayload, PutResult, Result,
	path::Path,
};
use std::{fmt, sync::Arc};

/// Restricted to immutable manifest creation and empty WAL-fence copying. An
/// already-sent request remains safe after revocation because it cannot replace
/// an existing object. No deletion, multipart or overwrite capability is exposed.
pub(super) struct CloneStore {
	store: Arc<dyn ObjectStore>,
	guard: Arc<dyn MutationGuard>,
}

impl CloneStore {
	pub fn new(store: Arc<dyn ObjectStore>, guard: Arc<dyn MutationGuard>) -> Arc<Self> {
		Arc::new(Self { store, guard })
	}

	fn check(&self) -> Result<()> {
		if self.guard.may_send() {
			Ok(())
		} else {
			Err(denied("clone authority revoked"))
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

impl fmt::Display for CloneStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("guarded SlateDB clone")
	}
}

impl fmt::Debug for CloneStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

#[async_trait::async_trait]
impl ObjectStore for CloneStore {
	async fn put_opts(
		&self,
		path: &Path,
		payload: PutPayload,
		options: PutOptions,
	) -> Result<PutResult> {
		self.check()?;
		if options.mode != PutMode::Create {
			return Err(denied("clone metadata must use conditional creation"));
		}
		self.store.put_opts(path, payload, options).await
	}

	async fn put_multipart_opts(
		&self,
		_: &Path,
		_: PutMultipartOptions,
	) -> Result<Box<dyn MultipartUpload>> {
		Err(denied("projected clones do not require multipart writes"))
	}

	async fn get_opts(&self, path: &Path, options: GetOptions) -> Result<GetResult> {
		self.store.get_opts(path, options).await
	}

	fn delete_stream(
		&self,
		paths: BoxStream<'static, Result<Path>>,
	) -> BoxStream<'static, Result<Path>> {
		paths
			.map(|_| Err(denied("clone initialization cannot delete objects")))
			.boxed()
	}

	fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
		self.store.list(prefix)
	}

	async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
		self.store.list_with_delimiter(prefix).await
	}

	async fn copy_opts(&self, from: &Path, to: &Path, _: CopyOptions) -> Result<()> {
		self.check()?;
		// A final projected checkpoint must have no data WAL left. Only empty
		// native-WAL fences may be copied, and never with upstream's overwrite.
		if self.store.head(from).await?.size != 0 {
			return Err(denied("projected clone encountered an unflushed data WAL"));
		}
		self.check()?;
		match self
			.store
			.put_opts(to, Bytes::new().into(), PutMode::Create.into())
			.await
		{
			Ok(_) => Ok(()),
			Err(object_store::Error::AlreadyExists { .. })
				if self.store.head(to).await?.size == 0 =>
			{
				Ok(())
			}
			Err(error) => Err(error),
		}
	}
}


