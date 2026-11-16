pub mod keys;

pub use rivet_keys_macros::{S3Key, SlatedbKey, UpsSubject, VolumeKey, VolumeKvKey};

/// The crates the key derives' generated code names, so a crate deriving a key needs only
/// `rivet-keys`.
#[doc(hidden)]
pub mod __private {
	pub use anyhow;
	pub use bytes;
	pub use foundationdb_tuple;
	pub use object_store;
}
