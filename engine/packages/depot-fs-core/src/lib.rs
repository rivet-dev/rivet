//! Deterministic filesystem operations and versioned rows shared by clients and maintainers.

#![warn(unreachable_pub)]

pub mod apply;
pub mod forks;
pub mod keys;
pub mod rules;
pub mod store;
pub mod txn;
pub mod wire;

pub use rivet_data::generated::depot_fs_entry_v1::{
	FsEntry, FsTxn, FsTxnAbort, FsTxnCommit, FsTxnPending,
};
pub use rivet_data::generated::shared_depot_fs_v1::*;

pub const CHUNK_BYTES: usize = 4096;
/// Creation default only. Mutations use the filesystem's persisted format row.
pub const DEFAULT_INLINE_MAX_BYTES: u64 = 900;
pub const ROOT_INO: u64 = 1;
pub const MAX_FORK_CHAIN_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
	#[error("filesystem snapshot read required")]
	ReadRequired(store::ReadRequest),
	#[error("entry does not exist")]
	NotFound,
	#[error("entry already exists")]
	Exists,
	#[error("not a directory")]
	NotDirectory,
	#[error("is a directory")]
	IsDirectory,
	#[error("directory is not empty")]
	NotEmpty,
	#[error("invalid filesystem operation")]
	Invalid,
	#[error("name is too long")]
	NameTooLong,
	#[error("too many symbolic links")]
	SymlinkLoop,
	#[error("file is too large")]
	TooLarge,
	#[error("extended attribute does not exist")]
	NoAttribute,
	#[error("corrupt filesystem row")]
	Corrupt,
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod checkpoints;

pub mod work;
