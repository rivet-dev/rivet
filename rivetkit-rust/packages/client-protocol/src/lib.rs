pub mod generated;
pub mod versioned;

// Re-export latest.
pub use generated::v5::*;

pub const PROTOCOL_VERSION: u16 = 5;
