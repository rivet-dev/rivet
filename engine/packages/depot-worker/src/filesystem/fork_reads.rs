//! Bounded source snapshot exports for metadata publication and sparse materialization.
use super::{
	session::{Error, Result},
	store::FilesystemStore,
};
use depot_fs_core::{
	keys::slatedb as keys,
	store::{RowKey, RowRange},
};
use rivet_data::generated::shared_depot_fs_read_v1::*;
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
				.map(|(key, value)| FsReadRow {
					change_seq: None,
					key: key.into(),
					value: value.into(),
				})
				.collect(),
		})
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
			.map(|(key, _)| match key {
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
