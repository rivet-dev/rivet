//! Typed KV commands shared by callers and tablet maintainers.
pub mod command;
pub use rivet_data::DEPOT_KV_COMMAND_VERSION as PROTOCOL_VERSION;
