//! Depot addresses owned by the V2 migrator. Its volume key lives in
//! `pegboard::keys::partition`, since a namespace's backfill reads the volume.
pub mod kv;
pub mod ups;
