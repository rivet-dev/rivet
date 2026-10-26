//! Serialize/Deserialize impls for CachedKey and CachedEntry to support DbCache impls that
//! can store cache items on-disk. Internally, the Serialize/Deserialize impls work by
//! converting these types to representations that derive Serialize/Deserialize. The purpose
//! of the indirection is to decouple the serialized format from the in-memory representation
//! used by the rest of the codebase.

use crate::db_cache::{CachedEntry, CachedItem, CachedKey, EncodedCachedFilter};
use crate::db_state::SsTableId;
use crate::error::SlateDBError;
use crate::filter_policy::BloomFilterPolicy;
use crate::flatbuffer_types::SsTableIndexOwned;
use crate::format::block::Block;
use crate::sst_stats::SstStats;
use bytes::Bytes;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use ulid::Ulid;

// V1/V2 encoded `SsTableId` when it could identify either a WAL or compacted
// SST. Keep both variants, and their order, so existing cache keys remain
// decodable by bincode.
#[derive(Serialize, Deserialize)]
enum SerializedSsTableOrWalId {
    Wal(u64),
    Compacted(Ulid),
}

impl TryFrom<SerializedSsTableOrWalId> for SsTableId {
    type Error = &'static str;

    fn try_from(value: SerializedSsTableOrWalId) -> Result<Self, Self::Error> {
        match value {
            SerializedSsTableOrWalId::Wal(_) => Err("WAL entries are not valid in the SST cache"),
            SerializedSsTableOrWalId::Compacted(id) => Ok(SsTableId::from(id)),
        }
    }
}

// Mirrors the current `SsTableId`, which only wraps the ULID of a compacted
// SST.
#[derive(Serialize, Deserialize)]
struct SerializedSsTableId(Ulid);

impl From<SsTableId> for SerializedSsTableId {
    fn from(value: SsTableId) -> Self {
        Self(value.value())
    }
}

impl From<SerializedSsTableId> for SsTableId {
    fn from(value: SerializedSsTableId) -> Self {
        Self::from(value.0)
    }
}

// Bincode identifies enum variants by their index. New versions must be
// appended so the indices of existing cache-key formats remain stable.
#[derive(Serialize, Deserialize)]
enum SerializedCachedKey {
    V1(SerializedSsTableOrWalId, u64),
    V2(u64, SerializedSsTableOrWalId, u64),
    V3(u64, SerializedSsTableId, u64),
}

impl TryFrom<SerializedCachedKey> for CachedKey {
    type Error = &'static str;

    fn try_from(value: SerializedCachedKey) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializedCachedKey::V1(sst_id, block_id) => CachedKey {
                db_cache_id: 0,
                sst_id: sst_id.try_into()?,
                block_id,
            },
            SerializedCachedKey::V2(db_cache_id, sst_id, block_id) => CachedKey {
                db_cache_id,
                sst_id: sst_id.try_into()?,
                block_id,
            },
            SerializedCachedKey::V3(db_cache_id, sst_id, block_id) => CachedKey {
                db_cache_id,
                sst_id: sst_id.into(),
                block_id,
            },
        })
    }
}

impl From<CachedKey> for SerializedCachedKey {
    fn from(value: CachedKey) -> Self {
        SerializedCachedKey::V3(value.db_cache_id, value.sst_id.into(), value.block_id)
    }
}

impl Serialize for CachedKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_key: SerializedCachedKey = self.clone().into();
        serialized_key.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CachedKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized_key = SerializedCachedKey::deserialize(deserializer)?;
        serialized_key.try_into().map_err(D::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
enum SerializedCachedEntryV1 {
    Block(Bytes),
    SsTableIndex(Bytes),
    BloomFilter(Bytes),
    SstStats(Bytes),
}

impl SerializedCachedEntryV1 {
    fn into_cached_entry(self) -> Result<CachedEntry, SlateDBError> {
        let item = match self {
            SerializedCachedEntryV1::Block(encoded) => {
                let block = Block::decode(encoded);
                CachedItem::Block(Arc::new(block))
            }
            SerializedCachedEntryV1::SsTableIndex(encoded) => {
                let index = SsTableIndexOwned::new(encoded)?;
                CachedItem::SsTableIndex(Arc::new(index))
            }
            SerializedCachedEntryV1::BloomFilter(encoded) => {
                // Produce an EncodedFilters item; decoding is deferred until
                // TableStore::read_filters resolves it against the configured
                // filter policies.
                CachedItem::EncodedFilters(Arc::from([EncodedCachedFilter {
                    name: BloomFilterPolicy::NAME.to_string(),
                    data: encoded,
                }]))
            }
            SerializedCachedEntryV1::SstStats(encoded) => {
                let stats = SstStats::decode(encoded)?;
                CachedItem::SstStats(Arc::new(stats))
            }
        };
        Ok(CachedEntry { item })
    }
}

/// Serialized representation of a composite filter block.
/// Format: list of (name, encoded_data) pairs.
#[derive(Serialize, Deserialize)]
struct SerializedCompositeFilters(Vec<(String, Bytes)>);

#[derive(Serialize, Deserialize)]
enum SerializedCachedEntryV2 {
    Filters(SerializedCompositeFilters),
}

#[derive(Serialize, Deserialize)]
enum SerializedCachedEntry {
    V1(SerializedCachedEntryV1),
    V2(SerializedCachedEntryV2),
}

impl SerializedCachedEntry {
    fn into_cached_entry(self) -> Result<CachedEntry, SlateDBError> {
        match self {
            SerializedCachedEntry::V1(entry) => entry.into_cached_entry(),
            SerializedCachedEntry::V2(entry) => match entry {
                SerializedCachedEntryV2::Filters(composite) => {
                    // Produce EncodedFilters; decoding is deferred until
                    // TableStore::read_filters resolves them against the
                    // configured filter policies.
                    let filters: Vec<EncodedCachedFilter> = composite
                        .0
                        .into_iter()
                        .map(|(name, data)| EncodedCachedFilter { name, data })
                        .collect();
                    Ok(CachedEntry {
                        item: CachedItem::EncodedFilters(filters.into()),
                    })
                }
            },
        }
    }
}

impl From<CachedEntry> for SerializedCachedEntry {
    fn from(value: CachedEntry) -> Self {
        match value.item {
            CachedItem::Block(block) => {
                let encoded = block.encode();
                SerializedCachedEntry::V1(SerializedCachedEntryV1::Block(encoded))
            }
            CachedItem::SsTableIndex(index) => {
                let encoded = index.data();
                SerializedCachedEntry::V1(SerializedCachedEntryV1::SsTableIndex(encoded))
            }
            CachedItem::Filters(filters) => {
                let pairs = filters
                    .iter()
                    .map(|nf| {
                        let mut buf = Vec::new();
                        nf.filter.encode(&mut buf);
                        (nf.name.clone(), Bytes::from(buf))
                    })
                    .collect();
                SerializedCachedEntry::V2(SerializedCachedEntryV2::Filters(
                    SerializedCompositeFilters(pairs),
                ))
            }
            CachedItem::EncodedFilters(filters) => {
                let pairs = filters
                    .iter()
                    .map(|ef| (ef.name.clone(), ef.data.clone()))
                    .collect();
                SerializedCachedEntry::V2(SerializedCachedEntryV2::Filters(
                    SerializedCompositeFilters(pairs),
                ))
            }
            CachedItem::SstStats(stats) => {
                let encoded = stats.encode();
                SerializedCachedEntry::V1(SerializedCachedEntryV1::SstStats(encoded))
            }
        }
    }
}

impl Serialize for CachedEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_entry: SerializedCachedEntry = self.clone().into();
        serialized_entry.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CachedEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized_entry = SerializedCachedEntry::deserialize(deserializer)?;
        serialized_entry
            .into_cached_entry()
            // the error returned by deserialize must be lowercase and not end in a .
            .map_err(|e| D::Error::custom(format!("slatedb error ({})", e).to_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::{SerializedCachedKey, SerializedSsTableId};
    use crate::block_iterator::BlockIteratorLatest;
    use crate::db_cache::{CachedEntry, CachedItem, CachedKey};
    use crate::db_state::SsTableId;
    use crate::filter_policy::{BloomFilterPolicy, FilterPolicy, NamedFilter};
    use crate::flatbuffer_types::{
        BlockMeta, BlockMetaArgs, SsTableIndex, SsTableIndexArgs, SsTableIndexOwned,
    };
    use crate::format::sst::BlockBuilder;
    use crate::iter::IterationOrder;
    use crate::sst_stats::SstStats;
    use crate::test_utils::assert_iterator;
    use crate::types::RowEntry;
    use bytes::Bytes;
    use std::sync::Arc;
    use ulid::Ulid;

    #[derive(serde::Serialize)]
    enum LegacySerializedSsTableOrWalId {
        Wal(u64),
        Compacted(Ulid),
    }

    #[derive(serde::Serialize)]
    enum LegacySerializedCachedKey {
        V1(LegacySerializedSsTableOrWalId, u64),
        V2(u64, LegacySerializedSsTableOrWalId, u64),
    }

    #[test]
    fn test_should_serialize_deserialize_compacted_sst_key() {
        let key = CachedKey {
            db_cache_id: 0,
            sst_id: SsTableId::from(Ulid::from((123, 456))),
            block_id: 99,
        };

        let encoded = bincode::serialize(&key).unwrap();
        let decoded: CachedKey = bincode::deserialize(&encoded).unwrap();

        assert_eq!(decoded, key);
    }

    #[test]
    fn test_should_serialize_cached_key_as_v3() {
        let key = CachedKey {
            db_cache_id: 5,
            sst_id: SsTableId::from(Ulid::from_parts(123, 0)),
            block_id: 99,
        };

        let encoded = bincode::serialize(&key).unwrap();
        let decoded: SerializedCachedKey = bincode::deserialize(&encoded).unwrap();

        match decoded {
            SerializedCachedKey::V3(db_cache_id, SerializedSsTableId(sst_id), block_id) => {
                assert_eq!(db_cache_id, key.db_cache_id);
                assert_eq!(sst_id, key.sst_id.value());
                assert_eq!(block_id, key.block_id);
            }
            _ => panic!("expected V3 cache key"),
        }
    }

    #[test]
    fn test_should_deserialize_legacy_compacted_sst_keys() {
        let sst_id = Ulid::from((123, 456));
        let expected_v1 = CachedKey {
            db_cache_id: 0,
            sst_id: SsTableId::from(sst_id),
            block_id: 98,
        };
        let expected_v2 = CachedKey {
            db_cache_id: 5,
            sst_id: SsTableId::from(sst_id),
            block_id: 99,
        };

        let encoded_v1 = bincode::serialize(&LegacySerializedCachedKey::V1(
            LegacySerializedSsTableOrWalId::Compacted(sst_id),
            expected_v1.block_id,
        ))
        .unwrap();
        let encoded_v2 = bincode::serialize(&LegacySerializedCachedKey::V2(
            expected_v2.db_cache_id,
            LegacySerializedSsTableOrWalId::Compacted(sst_id),
            expected_v2.block_id,
        ))
        .unwrap();

        assert_eq!(
            bincode::deserialize::<CachedKey>(&encoded_v1).unwrap(),
            expected_v1
        );
        assert_eq!(
            bincode::deserialize::<CachedKey>(&encoded_v2).unwrap(),
            expected_v2
        );
    }

    #[test]
    fn test_should_reject_legacy_wal_sst_key() {
        let encoded = bincode::serialize(&LegacySerializedCachedKey::V2(
            5,
            LegacySerializedSsTableOrWalId::Wal(123),
            99,
        ))
        .unwrap();

        let error = bincode::deserialize::<CachedKey>(&encoded).unwrap_err();
        assert!(error
            .to_string()
            .contains("WAL entries are not valid in the SST cache"));
    }

    #[tokio::test]
    async fn test_should_serialize_deserialize_block() {
        let rows = vec![
            RowEntry::new_value(b"foo", b"bar", 0),
            RowEntry::new_merge(b"biz", b"baz", 1),
            RowEntry::new_tombstone(b"bla", 2),
        ];
        let mut builder = BlockBuilder::new_latest(4096);
        for row in rows.iter() {
            assert!(builder.add(row.clone()).unwrap());
        }
        let block = Arc::new(builder.build().unwrap());
        let entry = CachedEntry {
            item: CachedItem::Block(block.clone()),
        };

        let encoded = bincode::serialize(&entry).unwrap();
        let decoded: CachedEntry = bincode::deserialize(&encoded).unwrap();

        let decoded_block = decoded.block().unwrap();
        assert!(block.as_ref() == decoded_block.as_ref());
        let mut iter = BlockIteratorLatest::new(decoded_block, IterationOrder::Ascending);
        assert_iterator(&mut iter, rows).await;
    }

    #[test]
    fn test_should_serialize_deserialize_index() {
        let first_keys = vec![b"foo".as_slice(), b"bar".as_slice(), b"baz".as_slice()];
        let index = Arc::new(build_index_with_first_keys(&first_keys));
        let entry = CachedEntry {
            item: CachedItem::SsTableIndex(index.clone()),
        };

        let encoded = bincode::serialize(&entry).unwrap();
        let decoded: CachedEntry = bincode::deserialize(&encoded).unwrap();

        let decoded_index = decoded.sst_index().unwrap();
        assert!(index.as_ref() == decoded_index.as_ref());
    }

    #[test]
    fn test_should_serialize_deserialize_filter() {
        let policy = BloomFilterPolicy::new(10);
        let mut builder = policy.builder();
        for k in [b"foo", b"bar", b"baz"] {
            builder.add_entry(&RowEntry::new(
                Bytes::copy_from_slice(k),
                crate::types::ValueDeletable::Value(Bytes::new()),
                0,
                None,
                None,
            ));
        }
        let filter = builder.build();
        let named = NamedFilter {
            name: BloomFilterPolicy::NAME.to_string(),
            filter: filter.clone(),
        };
        let entry = CachedEntry {
            item: CachedItem::Filters(Arc::from([named])),
        };

        let encoded = bincode::serialize(&entry).unwrap();
        let decoded: CachedEntry = bincode::deserialize(&encoded).unwrap();

        // Deserialization produces EncodedFilters, not Filters, because the
        // deserializer has no access to the filter policy list.
        assert!(decoded.filters().is_none());
        let encoded_filters = decoded.encoded_filters().unwrap();
        assert_eq!(1, encoded_filters.len());
        let decoded_ef = &encoded_filters[0];
        assert_eq!(decoded_ef.name, BloomFilterPolicy::NAME);
        // Verify the stored bytes round-trip to the original encoding.
        let mut original_bytes = Vec::new();
        filter.encode(&mut original_bytes);
        assert_eq!(Bytes::from(original_bytes), decoded_ef.data);
    }

    #[test]
    fn test_should_serialize_deserialize_stats() {
        let stats = Arc::new(SstStats {
            num_puts: 100,
            num_deletes: 10,
            num_merges: 5,
            raw_key_size: 2048,
            raw_val_size: 8192,
            block_stats: vec![],
        });
        let entry = CachedEntry {
            item: CachedItem::SstStats(stats.clone()),
        };

        let encoded = bincode::serialize(&entry).unwrap();
        let decoded: CachedEntry = bincode::deserialize(&encoded).unwrap();

        let decoded_stats = decoded.sst_stats().unwrap();
        assert_eq!(stats.as_ref(), decoded_stats.as_ref());
    }

    fn build_index_with_first_keys(first_keys: &[&[u8]]) -> SsTableIndexOwned {
        let mut index_builder = flatbuffers::FlatBufferBuilder::new();
        let mut block_metas = Vec::new();
        for fk in first_keys {
            let fk = index_builder.create_vector(fk);
            let block_meta = BlockMeta::create(
                &mut index_builder,
                &BlockMetaArgs {
                    first_key: Some(fk),
                    offset: 0u64,
                },
            );
            block_metas.push(block_meta);
        }
        let block_metas = index_builder.create_vector(&block_metas);
        let index_wip = SsTableIndex::create(
            &mut index_builder,
            &SsTableIndexArgs {
                block_meta: Some(block_metas),
            },
        );
        index_builder.finish(index_wip, None);
        let index_bytes = Bytes::copy_from_slice(index_builder.finished_data());
        SsTableIndexOwned::new(index_bytes).unwrap()
    }
}
