//! Bounded source snapshot exports for metadata publication and sparse materialization.
use super::{
	session::{Error, Result},
	store::FilesystemStore,
};
use depot_fs_core::{
	keys::slatedb as keys,
	store::{Row, RowKey, RowRange},
};
use rivet_data::generated::shared_depot_fs_read_v2::*;
use uuid::Uuid;

impl FilesystemStore {
	pub async fn fork_metadata_page(
		&self,
		request: FsForkMetadataRequest,
	) -> Result<FsForkMetadataPage> {
		self.admit()?;
		let limit = (request.limit as usize)
			.min(self.batch_rows)
			.min(self.config.max_rows);
		if limit == 0 {
			return Err(Error::TooLarge);
		}
		let scope = keys::Scope::new(
			&request.link.volume_key,
			Uuid::from_bytes(request.link.incarnation),
		);
		let after = request
			.after
			.map(|bytes| {
				let (found, row, seq) = keys::decode_key(&bytes).map_err(|_| Error::Corrupt)?;
				if found != scope || seq != 0 || !RowRange::MetadataAll.contains(&row) {
					return Err(Error::Corrupt);
				}
				Ok(row)
			})
			.transpose()?;
		let session = self.open_checkpoint_session(&request.link).await?;
		let result = session
			.scan(&RowRange::MetadataAll, after.as_ref(), limit)
			.await;
		self.close_session(session.id);
		self.admit()?;
		let rows = result?;
		Ok(FsForkMetadataPage {
			complete: rows.len() < limit,
			rows: rows
				.into_iter()
				.map(|(key, claim_log, value)| FsReadRow {
					change_seq: None,
					key: key.into(),
					value: value.into(),
					claim_log,
				})
				.collect(),
		})
	}

	/// Source-side read of one layer's metadata rows at a link's cut, for a child volume that
	/// reads its metadata through its lineage: this volume's own rows and whiteouts, never its
	/// bases. A get answers each key's newest version, with a KV key a range whiteout spans
	/// answered as a whiteout, reading each KV file's range whiteouts once for the batch; a scan
	/// answers a page of the range. Chunks have their own read, and the rows that never read
	/// through are refused.
	pub async fn read_base_rows(&self, request: FsBaseRowsRequest) -> Result<FsBaseRowsPage> {
		use depot_fs_core::wire::encode_base_row;
		self.admit()?;
		let session = self.open_checkpoint_session(&request.link).await?;
		let result = async {
			match request.read {
				FsBaseRead::FsBaseGet(get) => {
					// A get answers at most this maintainer's row limit of keys and its read byte
					// budget of values, in request order, and stops at whichever comes first with
					// `next` naming the last key it examined, so the child asks again for the rest.
					// The child's limits may differ from this maintainer's, as during a roll.
					let asked = get.keys.len();
					let keys: Vec<RowKey> = get
						.keys
						.into_iter()
						.take(self.config.max_rows)
						.map(RowKey::from)
						.collect();
					if keys.is_empty() && asked > 0 {
						return Err(Error::TooLarge);
					}
					if !keys.iter().all(base_row_kind) {
						return Err(Error::Corrupt);
					}

					// Each key's own version, until the byte budget runs out. A whiteout answers no
					// bytes, so the keys this layer holds no version of are settled after.
					let mut local = Vec::new();
					let mut bytes = 0usize;
					let mut complete = keys.len() == asked;
					for key in keys {
						let value = session
							.local_version(&key)
							.await?
							.map(|(_, _, value)| value);
						if let Some(value) = &value {
							let total = bytes.saturating_add(value.as_ref().map_or(0, row_bytes));
							if total > self.config.max_read_bytes {
								// A first key over the budget alone can never be answered.
								if local.is_empty() {
									return Err(Error::TooLarge);
								}
								complete = false;
								break;
							}
							bytes = total;
						}
						local.push((key, value));
					}

					// A KV key this layer holds no version of answers as a whiteout when one of
					// this layer's range whiteouts spans it.
					let absent: Vec<RowKey> = local
						.iter()
						.filter(|(_, value)| value.is_none())
						.map(|(key, _)| key.clone())
						.collect();
					let mut hidden = session.hidden_by_own_whiteouts(&absent).await?.into_iter();
					let mut rows = Vec::new();
					let mut examined: Option<RowKey> = None;
					for (key, value) in local {
						let value = match value {
							Some(value) => Some(value),
							None if hidden.next() == Some(true) => Some(None),
							None => None,
						};
						if let Some(value) = value {
							rows.push(encode_base_row(key.clone(), value));
						}
						examined = Some(key);
					}
					let next = match complete {
						true => None,
						false => examined.map(Into::into),
					};
					Ok(FsBaseRowsPage {
						rows,
						complete,
						next,
					})
				}
				FsBaseRead::FsBaseScan(scan) => {
					let range = RowRange::from(scan.range);
					if !base_range_kind(&range) {
						return Err(Error::Corrupt);
					}
					let limit = (scan.limit as usize)
						.min(self.batch_rows)
						.min(self.config.max_rows);
					if limit == 0 {
						return Err(Error::TooLarge);
					}
					let after = scan.after.map(RowKey::from);
					let (rows, complete) =
						session.scan_versions(&range, after.as_ref(), limit).await?;
					let next = rows.last().map(|(key, _, _)| key.clone().into());
					// A whiteout range reads this layer's whiteouts; any other range answers
					// only the rows a child reads through, since claims, orphans and
					// whiteouts belong to the layer that holds them.
					let whiteouts = matches!(
						range,
						RowRange::KvRangeWhiteouts(_) | RowRange::KvRangeWhiteoutsAll
					);
					Ok(FsBaseRowsPage {
						rows: rows
							.into_iter()
							.filter(|(key, _, _)| whiteouts || base_row_kind(key))
							.map(|(key, _, value)| encode_base_row(key, value))
							.collect(),
						complete,
						next,
					})
				}
			}
		}
		.await;
		self.close_session(session.id);
		self.admit()?;
		// A whiteout range read answers whiteouts as rows; nothing else is a whiteout here.
		result
	}

	/// Enumeration visits stored sparse chunks, never every index in a file's
	/// logical length. The destination resolves candidates through the full image.
	pub async fn base_chunk_keys(&self, request: FsBaseKeysRequest) -> Result<FsBaseKeysPage> {
		self.admit()?;
		let limit = (request.limit as usize)
			.min(self.batch_rows)
			.min(self.config.max_rows);
		if limit == 0 {
			return Err(Error::TooLarge);
		}
		let after = request.after.map(|p| RowKey::Chunk(p.ino, p.index));
		let session = self.open_checkpoint_session(&request.link).await?;
		let complete_image = session.base_chain.is_empty();
		let result = session
			.scan(&RowRange::ChunksAll, after.as_ref(), limit)
			.await;
		self.close_session(session.id);
		self.admit()?;
		let rows = result?;
		let complete = rows.len() < limit;
		let positions = rows
			.into_iter()
			.map(|(key, _, _)| match key {
				RowKey::Chunk(ino, index) => Ok(depot_fs_core::FsChunkPosition { ino, index }),
				_ => Err(Error::Corrupt),
			})
			.collect::<Result<Vec<_>>>()?;
		Ok(FsBaseKeysPage {
			complete_image,
			positions,
			complete,
		})
	}
}

/// Whether a base read may ask for `key`: the metadata rows a child reads through its lineage.
fn base_row_kind(key: &RowKey) -> bool {
	match key {
		RowKey::Format
		| RowKey::Inode(_)
		| RowKey::Dentry(..)
		| RowKey::Link(..)
		| RowKey::Xattr(..)
		| RowKey::Kv(..) => true,
		RowKey::Orphan(_) | RowKey::Chunk(..) | RowKey::KvRangeWhiteout(..) => false,
	}
}

/// Whether a base read may scan `range`: a metadata range, or this layer's own range whiteouts,
/// which a child reads to hide the keys they span from the levels below this one.
fn base_range_kind(range: &RowRange) -> bool {
	match range {
		RowRange::All
		| RowRange::MetadataAll
		| RowRange::Metadata(_)
		| RowRange::Directory(_)
		| RowRange::Xattrs(_)
		| RowRange::Kv { .. }
		| RowRange::KvRangeWhiteouts(_)
		| RowRange::KvRangeWhiteoutsAll => true,
		RowRange::ChunksAll
		| RowRange::Chunks(_)
		| RowRange::ChunkSuffix { .. }
		| RowRange::Orphans => false,
	}
}

/// The value bytes `row` carries, which bound a batched base get's answer.
fn row_bytes(row: &Row) -> usize {
	match row {
		Row::Bytes(bytes) => bytes.len(),
		Row::Inode(inode) => inode.inline.as_ref().map_or(0, |inline| inline.len()),
		Row::Format(_) | Row::Dentry(_) | Row::Unit | Row::KvRangeWhiteout(_) => 0,
	}
}
