pub mod generated;
pub use generated::v4 as protocol;
pub mod versioned;

pub use generated::PROTOCOL_VERSION;

pub const READ_STATE_REQUIRES_V4_ERROR: &str = "Epoxy read-state requires protocol v4";

mod convert;
