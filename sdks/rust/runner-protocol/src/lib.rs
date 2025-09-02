pub mod generated;
pub mod protocol;
pub mod versioned;

// Re-export latest
pub use generated::v1::*;

pub const PROTOCOL_VERSION: u16 = 1;
