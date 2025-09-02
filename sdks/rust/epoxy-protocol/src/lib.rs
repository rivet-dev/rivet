pub mod generated;
pub mod versioned;

// Re-export latest
pub use generated::v1 as protocol;

pub const PROTOCOL_VERSION: u16 = 1;
