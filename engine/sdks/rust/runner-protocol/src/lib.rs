pub mod compat;
pub mod generated;
pub mod util;
pub mod versioned;

// Re-export latest
pub use generated::v3::*;

pub const PROTOCOL_VERSION: u16 = 3;
