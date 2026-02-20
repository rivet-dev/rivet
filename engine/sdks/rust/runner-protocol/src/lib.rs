pub mod compat;
pub mod generated;
pub mod util;
pub mod uuid_compat;
pub mod versioned;

// Re-export latest
pub use generated::v3::*;
pub use generated::v6 as mk2;

pub const PROTOCOL_MK1_VERSION: u16 = 3;
pub const PROTOCOL_MK2_VERSION: u16 = 6;

pub fn is_mk2(protocol_version: u16) -> bool {
	protocol_version > PROTOCOL_MK1_VERSION
}
