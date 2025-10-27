pub mod generated;
pub mod versioned;

// Re-export latest
pub use generated::v2::*;

pub const PROTOCOL_VERSION: u16 = 2;
