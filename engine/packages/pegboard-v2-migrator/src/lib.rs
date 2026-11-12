//! Migration from V2: moves one V2 namespace's actors into this V3 cluster while they sleep.
//!
//! The migrator is a singleton state machine (see [`migrator_machine`]) that drives V2's
//! `/migration/v1/` routes and writes the moved actors into V3. Routing during the migration lives
//! in the gateway and the API shim.
pub mod commands;
pub mod import;
pub mod keys;
pub mod metrics;
pub mod migrator_machine;
pub mod operator;
pub mod recover;
pub mod requests;
pub mod services;
pub mod settings;
pub mod v2_client;
pub mod v3_actor;

/// Bytes per SQLite page on both sides. V2's export header carries its page size and the import
/// refuses any other.
pub const SQLITE_PAGE_SIZE: usize = 4096;
