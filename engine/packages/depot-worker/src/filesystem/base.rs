//! Read-only base effects; the containing tablet owner supplies regional routing.
use super::{
	session::{Error, ReadSession, Result},
	store::FilesystemStore,
};
use depot_fs_core::{CHUNK_BYTES, FsBaseLink, MAX_FORK_CHAIN_DEPTH, forks::ChunkLookup};

/// Implementations send a bounded point batch to the link's region and tablet.
/// They read only that layer; this caller owns the flat nearest-first chain walk.
#[async_trait::async_trait]
pub trait BaseChunkReader: Send + Sync {
	async fn read(
		&self,
		link: &FsBaseLink,
		ino: u64,
		indices: &[u64],
		deadline_boot_ms: u64,
	) -> Result<Vec<ChunkLookup>>;
}

impl ReadSession {
	pub async fn read_chunks(
		&self,
		ino: u64,
		first: u64,
		count: usize,
	) -> Result<Vec<Option<Vec<u8>>>> {
		self.check_chunk_count(count)?;
		let indices = (0..count)
			.map(|offset| first.checked_add(offset as u64).ok_or(Error::TooLarge))
			.collect::<Result<Vec<_>>>()?;
		self.resolve_chunks(ino, &indices).await
	}

	/// Own writes and holes never need the parent. Each remaining link receives
	/// one batch, and every partial truncation limit is carried into its reply.
	pub async fn resolve_chunks(&self, ino: u64, indices: &[u64]) -> Result<Vec<Option<Vec<u8>>>> {
		self.check()?;
		if self.base_chain.len() > MAX_FORK_CHAIN_DEPTH {
			return Err(Error::Corrupt);
		}
		let local = self.read_local_chunks(ino, indices).await?;
		let mut result = vec![None; indices.len()];
		let mut pending = Vec::new();
		for (slot, lookup) in local.into_iter().enumerate() {
			match lookup {
				ChunkLookup::Resolved(bytes) => result[slot] = bytes,
				ChunkLookup::Base { max_bytes } => pending.push((slot, indices[slot], max_bytes)),
			}
		}
		for link in &self.base_chain {
			if pending.is_empty() {
				break;
			}
			self.check()?;
			let reader = self.base_reader.as_ref().ok_or(Error::BaseUnavailable)?;
			let indices: Vec<_> = pending.iter().map(|(_, index, _)| *index).collect();
			let values = reader
				.read(link, ino, &indices, self.expires_boot_ms)
				.await?;
			self.check()?;
			if values.len() != pending.len() {
				return Err(Error::Corrupt);
			}
			let mut remaining = Vec::new();
			for ((slot, index, limit), value) in pending.into_iter().zip(values) {
				match value {
					ChunkLookup::Resolved(Some(mut bytes)) => {
						if bytes.len() > CHUNK_BYTES {
							return Err(Error::Corrupt);
						}
						bytes.truncate(limit);
						result[slot] = (!bytes.is_empty()).then_some(bytes);
					}
					ChunkLookup::Resolved(None) => {}
					ChunkLookup::Base { max_bytes }
						if max_bytes > 0 && max_bytes <= CHUNK_BYTES =>
					{
						remaining.push((slot, index, limit.min(max_bytes)))
					}
					_ => return Err(Error::Corrupt),
				}
			}
			pending = remaining;
		}
		if !pending.is_empty() {
			return Err(Error::Corrupt);
		}
		self.check()?;
		Ok(result)
	}
}

impl FilesystemStore {
	/// Source-side read effect. A short-lived local session protects the image
	/// while the point batch runs and is closed on both success and failure.
	pub async fn read_base_chunks(
		&self,
		link: &FsBaseLink,
		ino: u64,
		indices: &[u64],
	) -> Result<Vec<ChunkLookup>> {
		let session = self.open_checkpoint_session(link).await?;
		let result = session.read_local_chunks(ino, indices).await;
		self.close_session(session.id);
		self.admit()?;
		result
	}
}
