//! Filesystem calls and optimistic KV transactions over one exclusively owned volume log.

/// Builds [`Error::Corrupt`] and logs where it was detected. Corruption is rare and fatal for
/// its opening, so the source location is what an operator needs to find the inconsistency.
macro_rules! corrupt {
	() => {{
		tracing::warn!(
			file = file!(),
			line = line!(),
			"filesystem corruption detected"
		);
		$crate::filesystem::Error::Corrupt
	}};
}

mod admission;
mod backlog;
mod cache;
pub mod capture;
mod client;
mod file_batch;
mod recovery;
mod replay;
mod txn;
pub use file_batch::{FileBuffer, FileEdit, FileUpdate, FileWrite};
pub use txn::TxnHandle;
pub mod owner;
pub use owner::OpenedFilesystem;
pub mod remote;
mod transaction;
pub mod transport;

pub use cache::{CacheBudget, CacheReservation};
pub use client::{FileLock, Filesystem, NoteExpiry, StagedFile};
pub use depot_fs_core::{FsKind as Kind, rules::Attributes};
pub use transaction::{ReadRange, Transaction};

use depot_volume::volume_client_machine::types::WriteError;

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
	#[error("inode is held by an open filesystem transaction")]
	Busy,
	#[error(transparent)]
	Filesystem(#[from] depot_fs_core::Error),
	#[error("filesystem client is fenced")]
	Fenced,
	#[error("filesystem request exceeds its byte limit")]
	TooLarge,
	#[error("filesystem memory or request budget is exhausted")]
	Overloaded,
	/// This opening's admitted writes are too far ahead of the tablet's apply, as when a slow
	/// cold store or compaction holds back its memtable flushes. The write was refused before
	/// it was built, so the edge may send it again after `retry_after_ms`.
	#[error("volume writes are ahead of the tablet's apply; retry after {retry_after_ms} ms")]
	ApplyBacklog { retry_after_ms: u64 },
	#[error("snapshot has expired")]
	TooOld,
	#[error("transaction read set conflicts with a newer write")]
	Conflict,
	#[error("cold store is behind the requested position")]
	Lagging,
	#[error("cold store is unavailable")]
	Unavailable,
	/// The tablet maintainer did not serve a cold read: the route names no maintainer, the
	/// maintainer refused the read, or the request failed in transport. Reads are idempotent,
	/// so a caller may send the read again after a pause, for example once a re-grant lands.
	#[error("cold read was refused while the tablet maintainer is unavailable")]
	ColdReadRefused,
	#[error("inherited filesystem data is unavailable")]
	BaseUnavailable,
	#[error("request deadline elapsed")]
	DeadlineExceeded,
	#[error("volume log mutation failed: {0:?}")]
	Log(WriteError),
	#[error("filesystem history or row is corrupt")]
	Corrupt,
	/// The store holds no record for the volume's current incarnation and the maintainer cannot
	/// rebuild one. Nothing serves the name until `incarnation` is deleted.
	#[error("the volume's stored record is missing; delete its incarnation to free the name")]
	RecordMissing { incarnation: [u8; 16] },
}

pub type Result<T> = std::result::Result<T, Error>;

/// The hint of a refused cold read, which carries none of its own. A refusal while the
/// tablet's maintainer moves or a split's children start settles on the scale of the route
/// refresh the refusal already triggered.
const COLD_READ_REFUSED_RETRY_AFTER_MS: u64 = depot_types::client::NOT_MAINTAINER_RETRY_AFTER_MS;

impl Error {
	/// The hint after which the edge may send the request again because this error proves it
	/// ran nothing, or `None` for every other error. A refused cold read ran nothing: reads
	/// change no state, and a write's cold reads all happen before the write is admitted.
	pub fn retry_after_ms(&self) -> Option<u64> {
		match self {
			Self::Log(error) => error.retry_after_ms(),
			Self::ColdReadRefused => Some(COLD_READ_REFUSED_RETRY_AFTER_MS),
			Self::ApplyBacklog { retry_after_ms } => Some(*retry_after_ms),
			Self::Busy
			| Self::Filesystem(_)
			| Self::Fenced
			| Self::TooLarge
			| Self::Overloaded
			| Self::TooOld
			| Self::Conflict
			| Self::Lagging
			| Self::Unavailable
			| Self::BaseUnavailable
			| Self::DeadlineExceeded
			| Self::Corrupt
			| Self::RecordMissing { .. } => None,
		}
	}
}

pub use rivet_config::config::depot::FilesystemConfig as Config;

/// Inode postimage together with the logical sequence of its last write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inode {
	pub inode: depot_fs_core::FsInode,
	pub change_seq: u64,
}
impl std::ops::Deref for Inode {
	type Target = depot_fs_core::FsInode;
	fn deref(&self) -> &Self::Target {
		&self.inode
	}
}
