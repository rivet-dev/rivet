pub mod generated;
pub mod versioned;

// Re-export latest schema types so callers can use unprefixed paths.
pub use generated::v1::*;

pub const SQLITE_STORAGE_PROTOCOL_VERSION: u16 = 1;
