use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::options::{MutationType, StreamingMode};
use universaldb::utils::IsolationLevel::{Serializable, Snapshot};
use universaldb::RangeOption;

use super::{
	ActorDb, branch, keys,
	constants::MAX_PINS_PER_NAMESPACE,
	error::SqliteStorageError,
	types::{
		ActorBranchId, BookmarkRef, BookmarkStr, NamespaceBranchId, NamespaceId, PinStatus,
		PinnedBookmarkRecord, ResolvedVersionstamp, decode_actor_pointer, decode_commit_row,
		decode_db_head, decode_namespace_branch_record, decode_pinned_bookmark_record,
		encode_pinned_bookmark_record,
	},
};

const VERSIONSTAMP_INFINITY: [u8; 16] = [0xff; 16];

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

	pub async fn create_pinned_bookmark(&self, at_ms: i64) -> Result<BookmarkStr> {
		create_pinned_bookmark(
			&self.udb,
			&self.ups,
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

	pub async fn delete_pinned_bookmark(&self, bookmark: BookmarkStr) -> Result<()> {
		delete_pinned_bookmark(
			&self.udb,
			&self.ups,
			self.sqlite_namespace_id(),
			self.actor_id.clone(),
			bookmark,
		)
		.await
	}

	pub async fn resolve_bookmark(&self, bookmark: BookmarkStr) -> Result<ResolvedVersionstamp> {
		resolve_bookmark(
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

pub async fn create_pinned_bookmark(
	udb: &universaldb::Database,
	ups: &crate::compactor::Ups,
	namespace_id: NamespaceId,
	actor_id: String,
	at_ms: i64,
) -> Result<BookmarkStr> {
	let result = udb
		.run(move |tx| {
			let actor_id = actor_id.clone();

			async move {
				let namespace_branch_id =
					branch::resolve_namespace_branch(&tx, namespace_id, Serializable)
						.await?
						.unwrap_or_else(NamespaceBranchId::nil);
				let branch_id = branch::resolve_actor_branch(&tx, namespace_id, &actor_id, Serializable)
					.await?
					.ok_or(SqliteStorageError::ActorNotFound)?;
				let head_bytes = tx
					.informal()
					.get(&keys::branch_meta_head_key(branch_id), Serializable)
					.await?
					.context("sqlite actor branch head is missing")?;
				let head = decode_db_head(&head_bytes).context("decode sqlite actor branch head")?;
				let commit_bytes = tx
					.informal()
					.get(&keys::branch_commit_key(branch_id, head.head_txid), Serializable)
					.await?
					.context("sqlite actor branch head commit row is missing")?;
				let commit =
					decode_commit_row(&commit_bytes).context("decode sqlite actor branch commit row")?;
				let bookmark = BookmarkStr::format(at_ms, head.head_txid)?;
				let pinned_key = keys::bookmark_pinned_key(&actor_id, bookmark.as_str());

				if tx.informal().get(&pinned_key, Serializable).await?.is_none() {
					let pin_count_key = keys::namespace_branches_pin_count_key(namespace_branch_id);
					let pin_count = tx
						.informal()
						.get(&pin_count_key, Serializable)
						.await?
						.map(|bytes| decode_i64_counter(&bytes))
						.transpose()?
						.unwrap_or(0);
					if pin_count >= i64::from(MAX_PINS_PER_NAMESPACE) {
						return Err(SqliteStorageError::TooManyPins.into());
					}

					let record = PinnedBookmarkRecord {
						bookmark: bookmark.clone(),
						actor_branch_id: branch_id,
						versionstamp: commit.versionstamp,
						status: PinStatus::Pending,
						pin_object_key: None,
						created_at_ms: at_ms,
						updated_at_ms: at_ms,
					};
					let encoded = encode_pinned_bookmark_record(record)
						.context("encode sqlite pinned bookmark record")?;
					tx.informal().set(&pinned_key, &encoded);
					tx.informal().atomic_op(
						&pin_count_key,
						&1_i64.to_le_bytes(),
						MutationType::Add,
					);
				}
				tx.informal().atomic_op(
					&keys::branches_bk_pin_key(branch_id),
					&commit.versionstamp,
					MutationType::ByteMin,
				);

				Ok(PinnedBookmarkCreateResult {
					bookmark: bookmark.clone(),
					payload: crate::compactor::SqliteColdCompactPayload::CreatePinnedBookmark {
						actor_id,
						actor_branch_id: branch_id,
						bookmark,
						versionstamp: commit.versionstamp,
					},
				})
			}
		})
		.await?;

	crate::compactor::publish_cold_compact_payload(ups, result.payload).await?;

	Ok(result.bookmark)
}

pub async fn delete_pinned_bookmark(
	udb: &universaldb::Database,
	ups: &crate::compactor::Ups,
	namespace_id: NamespaceId,
	actor_id: String,
	bookmark: BookmarkStr,
) -> Result<()> {
	let result = udb
		.run(move |tx| {
			let actor_id = actor_id.clone();
			let bookmark = bookmark.clone();

			async move {
				let pinned_key = keys::bookmark_pinned_key(&actor_id, bookmark.as_str());
				let Some(pinned_bytes) = tx.informal().get(&pinned_key, Serializable).await? else {
					return Ok(None);
				};
				let pinned = decode_pinned_bookmark_record(&pinned_bytes)
					.context("decode sqlite pinned bookmark record")?;
				let namespace_branch_id =
					branch::resolve_namespace_branch(&tx, namespace_id, Serializable)
						.await?
						.unwrap_or_else(NamespaceBranchId::nil);
				let pin_count_key = keys::namespace_branches_pin_count_key(namespace_branch_id);

				tx.informal()
					.clear(&keys::bookmark_key(&actor_id, bookmark.as_str()));
				tx.informal().clear(&pinned_key);
				tx.informal().atomic_op(
					&pin_count_key,
					&(-1_i64).to_le_bytes(),
					MutationType::Add,
				);

				let recomputed_pin =
					recompute_actor_branch_bk_pin(&tx, &actor_id, pinned.actor_branch_id, &pinned_key)
						.await?;
				tx.informal()
					.set(&keys::branches_bk_pin_key(pinned.actor_branch_id), &recomputed_pin);

				Ok(Some(
					crate::compactor::SqliteColdCompactPayload::DeletePinnedBookmark {
						actor_id,
						actor_branch_id: pinned.actor_branch_id,
						bookmark,
						versionstamp: pinned.versionstamp,
						pin_object_key: pinned.pin_object_key,
					},
				))
			}
		})
		.await?;

	if let Some(payload) = result {
		crate::compactor::publish_cold_compact_payload(ups, payload).await?;
	}

	Ok(())
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

pub async fn resolve_bookmark(
	udb: &universaldb::Database,
	namespace_id: NamespaceId,
	actor_id: String,
	bookmark: BookmarkStr,
) -> Result<ResolvedVersionstamp> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let bookmark = bookmark.clone();

		async move {
			let (branch_id, namespace_cap) =
				resolve_visible_actor_branch_for_bookmark(&tx, namespace_id, &actor_id).await?;
			resolve_bookmark_in_branch_chain(&tx, &actor_id, branch_id, namespace_cap, bookmark).await
		}
	})
	.await
}

async fn resolve_visible_actor_branch_for_bookmark(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	actor_id: &str,
) -> Result<(ActorBranchId, [u8; 16])> {
	let Some(mut namespace_branch_id) =
		branch::resolve_namespace_branch(tx, namespace_id, Snapshot).await?
	else {
		if let Some(pointer) =
			branch::resolve_actor_pointer(tx, NamespaceBranchId::nil(), actor_id, Snapshot).await?
		{
			return Ok((pointer.current_branch, VERSIONSTAMP_INFINITY));
		}

		return Err(SqliteStorageError::BranchNotReachable.into());
	};

	let mut cap = VERSIONSTAMP_INFINITY;
	for _ in 0..=crate::constants::MAX_NAMESPACE_DEPTH {
		if let Some(pointer_bytes) = tx
			.informal()
			.get(
				&keys::actor_pointer_cur_key(namespace_branch_id, actor_id),
				Snapshot,
			)
			.await?
		{
			let pointer = decode_actor_pointer(&pointer_bytes)
				.context("decode sqlite actor pointer during bookmark resolution")?;
			return Ok((pointer.current_branch, cap));
		}

		if tx
			.informal()
			.get(
				&keys::namespace_branches_actor_tombstone_key(namespace_branch_id, actor_id),
				Snapshot,
			)
			.await?
			.is_some()
		{
			return Err(SqliteStorageError::BranchNotReachable.into());
		}

		let Some(record_bytes) = tx
			.informal()
			.get(&keys::namespace_branches_list_key(namespace_branch_id), Snapshot)
			.await?
		else {
			return Err(SqliteStorageError::BranchNotReachable.into());
		};
		let record = decode_namespace_branch_record(&record_bytes)
			.context("decode sqlite namespace branch record during bookmark resolution")?;
		let Some(parent) = record.parent else {
			return Err(SqliteStorageError::BranchNotReachable.into());
		};
		if let Some(parent_versionstamp) = record.parent_versionstamp {
			cap = cap.min(parent_versionstamp);
		}
		namespace_branch_id = parent;
	}

	Err(SqliteStorageError::NamespaceForkChainTooDeep.into())
}

async fn resolve_bookmark_in_branch_chain(
	tx: &universaldb::Transaction,
	actor_id: &str,
	branch_id: ActorBranchId,
	namespace_cap: [u8; 16],
	bookmark: BookmarkStr,
) -> Result<ResolvedVersionstamp> {
	let (_ts_ms, bookmark_txid) = bookmark.parse()?;
	let pinned_record = tx
		.informal()
		.get(
			&keys::bookmark_pinned_key(actor_id, bookmark.as_str()),
			Snapshot,
		)
		.await?
		.map(|bytes| {
			decode_pinned_bookmark_record(&bytes)
				.context("decode sqlite pinned bookmark record during bookmark resolution")
		})
		.transpose()?;

	let mut current_branch_id = branch_id;
	let mut cap = namespace_cap;
	let mut saw_branch = false;
	for _ in 0..=crate::constants::MAX_FORK_DEPTH {
		saw_branch = true;

		if let Some(record) = pinned_record.as_ref().filter(|record| {
			record.actor_branch_id == current_branch_id && record.versionstamp <= cap
		}) {
			return Ok(ResolvedVersionstamp {
				versionstamp: record.versionstamp,
				bookmark: Some(BookmarkRef {
					bookmark: bookmark.clone(),
					resolved_versionstamp: Some(record.versionstamp),
				}),
			});
		}

		if let Some(row) = read_commit_row(tx, current_branch_id, bookmark_txid).await? {
			if row.versionstamp > cap {
				return Err(SqliteStorageError::BranchNotReachable.into());
			}

			let Some(vtx_txid) = lookup_vtx_txid(tx, current_branch_id, row.versionstamp).await?
			else {
				return Err(SqliteStorageError::BookmarkExpired.into());
			};
			if vtx_txid == bookmark_txid {
				return Ok(ResolvedVersionstamp {
					versionstamp: row.versionstamp,
					bookmark: Some(BookmarkRef {
						bookmark: bookmark.clone(),
						resolved_versionstamp: Some(row.versionstamp),
					}),
				});
			}
		}

		let record = branch::read_actor_branch_record(tx, current_branch_id).await?;
		let Some(parent) = record.parent else {
			break;
		};
		let parent_versionstamp = record
			.parent_versionstamp
			.context("sqlite actor branch parent versionstamp is missing")?;
		cap = cap.min(parent_versionstamp);
		current_branch_id = parent;
	}

	if pinned_record.is_some() && saw_branch {
		return Err(SqliteStorageError::BranchNotReachable.into());
	}

	Err(SqliteStorageError::BookmarkExpired.into())
}

async fn read_commit_row(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	txid: u64,
) -> Result<Option<super::types::CommitRow>> {
	let Some(bytes) = tx
		.informal()
		.get(&keys::branch_commit_key(branch_id, txid), Serializable)
		.await?
	else {
		return Ok(None);
	};

	Ok(Some(
		decode_commit_row(&bytes).context("decode sqlite commit row during bookmark resolution")?,
	))
}

async fn lookup_vtx_txid(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	versionstamp: [u8; 16],
) -> Result<Option<u64>> {
	let Some(bytes) = tx
		.informal()
		.get(&keys::branch_vtx_key(branch_id, versionstamp), Serializable)
		.await?
	else {
		return Ok(None);
	};
	let bytes = Vec::<u8>::from(bytes);
	let bytes: [u8; std::mem::size_of::<u64>()] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(Some(u64::from_be_bytes(bytes)))
}

async fn recompute_actor_branch_bk_pin(
	tx: &universaldb::Transaction,
	actor_id: &str,
	branch_id: ActorBranchId,
	deleted_pinned_key: &[u8],
) -> Result<[u8; 16]> {
	let start = keys::bookmark_key(actor_id, "");
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(start));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Serializable,
	);
	let mut pin = VERSIONSTAMP_INFINITY;

	while let Some(entry) = stream.try_next().await? {
		if entry.key() == deleted_pinned_key || !entry.key().ends_with(b"/pinned") {
			continue;
		}

		let record = decode_pinned_bookmark_record(entry.value())
			.context("decode sqlite pinned bookmark record during pin recompute")?;
		if record.actor_branch_id == branch_id {
			pin = pin.min(record.versionstamp);
		}
	}

	Ok(pin)
}

struct PinnedBookmarkCreateResult {
	bookmark: BookmarkStr,
	payload: crate::compactor::SqliteColdCompactPayload,
}

fn decode_i64_counter(bytes: &[u8]) -> Result<i64> {
	let bytes: [u8; std::mem::size_of::<i64>()] = bytes
		.try_into()
		.context("sqlite counter should be exactly 8 bytes")?;

	Ok(i64::from_le_bytes(bytes))
}
