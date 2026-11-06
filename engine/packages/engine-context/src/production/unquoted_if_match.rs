//! S3 stores that reject a quoted `If-Match` header.
//!
//! `object_store` sends the ETag exactly as the server returned it, with its quotes
//! (`If-Match: "abc"`), as RFC 9110 requires. Hetzner Object Storage answers 412 to that form
//! even when the ETag is current and accepts the bare form (`If-Match: abc`), so every
//! compare-and-swap the engine makes (leases, heartbeats, tablet manifests, conditional
//! delete) would fail forever there. This adapter strips one pair of surrounding quotes from
//! the ETag of `PutMode::Update` and `delete_if_match`. Everything else passes through.
//! It is enabled per store with `object_storage.primary.s3.unquoted_if_match`, never
//! implicitly.

use std::{fmt, ops::Range, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::stream::BoxStream;
use object_store::{
	CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
	PutMode, PutMultipartOptions, PutOptions, PutPayload, PutResult, RenameOptions,
	UpdateVersion, path::Path,
};

#[derive(Debug)]
pub struct UnquotedIfMatchStore {
	inner: Arc<dyn ObjectStore>,
}

impl UnquotedIfMatchStore {
	pub fn wrap(inner: Arc<dyn ObjectStore>) -> Arc<dyn ObjectStore> {
		Arc::new(Self { inner })
	}
}

impl fmt::Display for UnquotedIfMatchStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "UnquotedIfMatch({})", self.inner)
	}
}

/// One pair of surrounding double quotes removed; a weak validator (`W/"..."`) or an
/// already bare tag is returned unchanged.
fn unquote(etag: &str) -> &str {
	etag.strip_prefix('"')
		.and_then(|rest| rest.strip_suffix('"'))
		.unwrap_or(etag)
}

#[async_trait]
impl ObjectStore for UnquotedIfMatchStore {
	async fn put_opts(
		&self,
		location: &Path,
		payload: PutPayload,
		mut opts: PutOptions,
	) -> object_store::Result<PutResult> {
		if let PutMode::Update(version) = &opts.mode {
			opts.mode = PutMode::Update(UpdateVersion {
				e_tag: version.e_tag.as_deref().map(|etag| unquote(etag).to_owned()),
				version: version.version.clone(),
			});
		}
		self.inner.put_opts(location, payload, opts).await
	}

	async fn put_multipart_opts(
		&self,
		location: &Path,
		opts: PutMultipartOptions,
	) -> object_store::Result<Box<dyn MultipartUpload>> {
		self.inner.put_multipart_opts(location, opts).await
	}

	async fn get_opts(
		&self,
		location: &Path,
		options: GetOptions,
	) -> object_store::Result<GetResult> {
		self.inner.get_opts(location, options).await
	}

	async fn get_ranges(
		&self,
		location: &Path,
		ranges: &[Range<u64>],
	) -> object_store::Result<Vec<Bytes>> {
		self.inner.get_ranges(location, ranges).await
	}

	async fn delete_if_match(&self, location: &Path, etag: &str) -> object_store::Result<()> {
		self.inner.delete_if_match(location, unquote(etag)).await
	}

	fn delete_stream(
		&self,
		locations: BoxStream<'static, object_store::Result<Path>>,
	) -> BoxStream<'static, object_store::Result<Path>> {
		self.inner.delete_stream(locations)
	}

	fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
		self.inner.list(prefix)
	}

	fn list_with_offset(
		&self,
		prefix: Option<&Path>,
		offset: &Path,
	) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
		self.inner.list_with_offset(prefix, offset)
	}

	async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
		self.inner.list_with_delimiter(prefix).await
	}

	async fn copy_opts(
		&self,
		from: &Path,
		to: &Path,
		options: CopyOptions,
	) -> object_store::Result<()> {
		self.inner.copy_opts(from, to, options).await
	}

	async fn rename_opts(
		&self,
		from: &Path,
		to: &Path,
		options: RenameOptions,
	) -> object_store::Result<()> {
		self.inner.rename_opts(from, to, options).await
	}
}

#[cfg(test)]
mod tests {
	use super::unquote;

	#[test]
	fn strips_one_pair_of_quotes_only() {
		assert_eq!(unquote("\"abc\""), "abc");
		assert_eq!(unquote("abc"), "abc");
		assert_eq!(unquote("W/\"abc\""), "W/\"abc\"");
		assert_eq!(unquote("\""), "\"");
		assert_eq!(unquote(""), "");
	}
}
