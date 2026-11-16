//! Binary formats and typed addresses shared by Depot storage workers and clients.
//!
//! Schema ownership and version envelopes live in `rivet-data`. This crate adds
//! log batch framing and address codecs; it performs no storage or network I/O.

#![warn(unreachable_pub)]

pub mod alarm;
pub mod client;
pub mod hash_range_index;
pub mod keys;
pub mod log_shard;
pub mod membership;
pub mod payload_codec;

mod claim_log;
pub use claim_log::{ClaimLogName, ROOT_CLAIM_LOG, claim_log_key, claim_logs, is_claim_log};

mod volume_log_key;
pub use volume_log_key::{VolumeLogKey, listing_order, partition_key_hash};

pub use rivet_data::generated::shared_depot_types_v1::{
	PartitionKeyHash, PartitionKeyHashRange, VolumeKey,
};
pub use rivet_keys::keys::codec::GroupId;

pub mod stop;
pub mod storage_worker;
