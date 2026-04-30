use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel::Snapshot;

use super::{
	ActorDb, branch, keys,
	error::SqliteStorageError,
	types::{
		BookmarkStr, NamespaceId, PinStatus, decode_db_head, decode_pinned_bookmark_record,
	},
};

impl ActorDb {
	pub async fn create_bookmark(&self, at_ms: i64) -> Result<BookmarkStr> {
		create_bookmark(
			&self.udb,
			self.sqlite_namespace_id(),
			self.actor_id.clone(),
			at_ms,
		)
		.await
	}

	pub async fn bookmark_status(&self, bookmark: BookmarkStr) -> Result<Option<PinStatus>> {
		bookmark_status(
			&self.udb,
			self.sqlite_namespace_id(),
			self.actor_id.clone(),
			bookmark,
		)
		.await
	}
}

pub async fn create_bookmark(
	udb: &universaldb::Database,
	namespace_id: NamespaceId,
	actor_id: String,
	at_ms: i64,
) -> Result<BookmarkStr> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let branch_id = branch::resolve_actor_branch(&tx, namespace_id, &actor_id, Snapshot)
				.await?
				.ok_or(SqliteStorageError::ActorNotFound)?;
			let head_bytes = tx
				.informal()
				.get(&keys::branch_meta_head_key(branch_id), Snapshot)
				.await?
				.context("sqlite actor branch head is missing")?;
			let head = decode_db_head(&head_bytes).context("decode sqlite actor branch head")?;

			BookmarkStr::format(at_ms, head.head_txid)
		}
	})
	.await
}

pub async fn bookmark_status(
	udb: &universaldb::Database,
	_namespace_id: NamespaceId,
	actor_id: String,
	bookmark: BookmarkStr,
) -> Result<Option<PinStatus>> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let bookmark = bookmark.clone();

		async move {
			let Some(bytes) = tx
				.informal()
				.get(
					&keys::bookmark_pinned_key(&actor_id, bookmark.as_str()),
					Snapshot,
				)
				.await?
			else {
				return Ok(None);
			};
			let record = decode_pinned_bookmark_record(&bytes)
				.context("decode sqlite pinned bookmark record")?;

			Ok(Some(record.status))
		}
	})
	.await
}
