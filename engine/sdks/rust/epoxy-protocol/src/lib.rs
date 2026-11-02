pub mod generated;
pub use generated::v5 as protocol;
pub mod versioned;

pub use generated::PROTOCOL_VERSION;

/// The oldest Epoxy version this build speaks. Versions 1 to 4 are the layouts released with
/// Rivet 2.x, which this build cannot decode. Its own layout is numbered above them, so a
/// released peer's request fails the version check instead of being decoded as this layout.
pub const OLDEST_PROTOCOL_VERSION: u16 = 5;

// TODO: Remove this legacy error sentinel with the deferred Auth-JWT read fallback.
pub const READ_STATE_REQUIRES_V4_ERROR: &str = "Epoxy read-state requires protocol v4";
