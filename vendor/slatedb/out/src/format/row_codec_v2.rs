#![allow(dead_code)]
use crate::error::SlateDBError;
use crate::format::row::RowFlags;
use crate::types::ValueDeletable;
use crate::utils::{decode_varint, encode_varint, varint_len};
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// Intermediate representation for V2 row encoding.
///
/// The V2 codec encodes entries with prefix compression relative to the **previous key**
/// (not the first key as in V0/V1). At restart points, the full key is stored (shared_bytes=0).
///
/// ```text
/// |-------------------------------------------------------------------------------------------------------|
/// | shared    | unshared  | value_len | key_suffix | value   | seq  | flags | [expire_ts] | [create_ts] |
/// | (varint)  | (varint)  | (varint)  | (var)      | (var)   | u64  | u8    | i64         | i64         |
/// |-------------------------------------------------------------------------------------------------------|
/// ```
///
/// | Field       | Type    | Description                                                |
/// |-------------|---------|------------------------------------------------------------|
/// | shared      | varint  | Bytes shared with previous key (0 at restart points)      |
/// | unshared    | varint  | Length of key_suffix                                       |
/// | value_len   | varint  | Length of value (0 for tombstones)                         |
/// | key_suffix  | var     | The non-shared suffix of the key                           |
/// | value       | var     | Value bytes (omitted for tombstones)                       |
/// | seq         | u64     | Sequence number                                            |
/// | flags       | u8      | Row flags (tombstone, merge operand, timestamp presence)   |
/// | expire_ts   | i64     | Optional expiration timestamp (present if HAS_EXPIRE_TS)   |
/// | create_ts   | i64     | Optional creation timestamp (present if HAS_CREATE_TS)     |
///
/// Note: seq, expire_ts, and create_ts use fixed-width encoding (not varint) because:
/// - Sequence numbers are typically large (monotonically increasing counters)
/// - Timestamps are 64-bit values that rarely benefit from varint compression
/// - Fixed-width encoding simplifies parsing and provides predictable performance
#[derive(Debug, Clone)]
pub(crate) struct SstRowEntryV2 {
    /// Bytes shared with previous key (0 at restart points)
    pub shared_bytes: u32,
    /// Unshared portion of the key (the key suffix/delta)
    pub key_suffix: Bytes,
    pub seq: u64,
    pub expire_ts: Option<i64>,
    pub create_ts: Option<i64>,
    pub value: ValueDeletable,
}

impl SstRowEntryV2 {
    pub(crate) fn new(
        shared_bytes: u32,
        key_suffix: Bytes,
        seq: u64,
        value: ValueDeletable,
        create_ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> Self {
        Self {
            shared_bytes,
            key_suffix,
            seq,
            expire_ts,
            create_ts,
            value,
        }
    }

    pub(crate) fn flags(&self) -> RowFlags {
        let mut flags = match &self.value {
            ValueDeletable::Value(_) => RowFlags::default(),
            ValueDeletable::Merge(_) => RowFlags::MERGE_OPERAND,
            ValueDeletable::Tombstone => RowFlags::TOMBSTONE,
        };
        if self.expire_ts.is_some() {
            flags |= RowFlags::HAS_EXPIRE_TS;
        }
        if self.create_ts.is_some() {
            flags |= RowFlags::HAS_CREATE_TS;
        }
        flags
    }

    /// Restore the full key given the previous key.
    pub(crate) fn restore_full_key(&self, previous_key: &[u8]) -> Bytes {
        let shared = self.shared_bytes as usize;
        let mut full_key = BytesMut::with_capacity(shared + self.key_suffix.len());
        full_key.extend_from_slice(&previous_key[..shared]);
        full_key.extend_from_slice(&self.key_suffix);
        full_key.freeze()
    }

    /// Calculate the encoded size of this entry including varint overhead.
    pub(crate) fn encoded_size(&self) -> usize {
        let shared_bytes_len = varint_len(self.shared_bytes);
        let unshared_bytes_len = varint_len(self.key_suffix.len() as u32);
        let value_len = match &self.value {
            ValueDeletable::Value(v) | ValueDeletable::Merge(v) => v.len(),
            ValueDeletable::Tombstone => 0,
        };
        let value_len_varint_size = varint_len(value_len as u32);

        let mut size = shared_bytes_len // shared_bytes varint
            + unshared_bytes_len        // unshared_bytes varint
            + value_len_varint_size     // value_len varint
            + self.key_suffix.len()     // key_delta
            + value_len                 // value
            + 8                         // seq (u64)
            + 1; // flags (u8)

        if self.expire_ts.is_some() {
            size += 8; // i64
        }
        if self.create_ts.is_some() {
            size += 8; // i64
        }
        size
    }
}

pub(crate) struct SstRowCodecV2;

impl SstRowCodecV2 {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Encode a V2 row entry to the output buffer.
    pub(crate) fn encode(&self, output: &mut Vec<u8>, row: &SstRowEntryV2) {
        // Encode varints
        encode_varint(output, row.shared_bytes);
        encode_varint(output, row.key_suffix.len() as u32);

        let value_len = match &row.value {
            ValueDeletable::Value(v) | ValueDeletable::Merge(v) => v.len(),
            ValueDeletable::Tombstone => 0,
        };
        encode_varint(output, value_len as u32);

        // Encode key delta
        output.put(row.key_suffix.as_ref());

        // Encode value (if not tombstone)
        match &row.value {
            ValueDeletable::Value(v) | ValueDeletable::Merge(v) => {
                output.put(v.as_ref());
            }
            ValueDeletable::Tombstone => {
                // No value bytes for tombstones
            }
        }

        // Encode seq & flags
        let flags = row.flags();
        output.put_u64(row.seq);
        output.put_u8(flags.bits());

        // Encode expire & create timestamps
        if flags.contains(RowFlags::HAS_EXPIRE_TS) {
            output.put_i64(
                row.expire_ts
                    .expect("expire_ts should be set with HAS_EXPIRE_TS"),
            );
        }
        if flags.contains(RowFlags::HAS_CREATE_TS) {
            output.put_i64(
                row.create_ts
                    .expect("create_ts should be set with HAS_CREATE_TS"),
            );
        }
    }

    /// Decode a V2 row entry from the data buffer.
    pub(crate) fn decode(&self, data: &mut impl Buf) -> Result<SstRowEntryV2, SlateDBError> {
        let shared_bytes = decode_varint(data);
        let unshared_bytes = decode_varint(data) as usize;
        let value_len = decode_varint(data) as usize;

        // Read key_delta
        let key_suffix = data.copy_to_bytes(unshared_bytes);

        // Read value
        let value_bytes = if value_len > 0 {
            Some(data.copy_to_bytes(value_len))
        } else {
            None
        };

        // Read seq & flags
        let seq = data.get_u64();
        let flags = self.decode_flags(data.get_u8())?;

        // Read timestamps
        let (expire_ts, create_ts) =
            if flags.contains(RowFlags::HAS_EXPIRE_TS | RowFlags::HAS_CREATE_TS) {
                (Some(data.get_i64()), Some(data.get_i64()))
            } else if flags.contains(RowFlags::HAS_EXPIRE_TS) {
                (Some(data.get_i64()), None)
            } else if flags.contains(RowFlags::HAS_CREATE_TS) {
                (None, Some(data.get_i64()))
            } else {
                (None, None)
            };

        // Determine value type
        let value = if flags.contains(RowFlags::TOMBSTONE) {
            ValueDeletable::Tombstone
        } else if flags.contains(RowFlags::MERGE_OPERAND) {
            ValueDeletable::Merge(value_bytes.unwrap_or_else(Bytes::new))
        } else {
            ValueDeletable::Value(value_bytes.unwrap_or_else(Bytes::new))
        };

        Ok(SstRowEntryV2 {
            shared_bytes,
            key_suffix,
            seq,
            expire_ts,
            create_ts,
            value,
        })
    }

    /// Decode only the key portion for seek optimization.
    /// Returns (shared_bytes, key_suffix).
    pub(crate) fn decode_key_only(&self, data: &mut impl Buf) -> (u32, Bytes) {
        let shared_bytes = decode_varint(data);
        let unshared_bytes = decode_varint(data) as usize;
        let _value_len = decode_varint(data);

        let key_suffix = data.copy_to_bytes(unshared_bytes);

        (shared_bytes, key_suffix)
    }

    fn decode_flags(&self, flags: u8) -> Result<RowFlags, SlateDBError> {
        let parsed =
            RowFlags::from_bits(flags).ok_or_else(|| SlateDBError::InvalidRowFlags {
                encoded_bits: flags,
                known_bits: RowFlags::all().bits(),
                message: "Unable to parse flags. This may be caused by reading data encoded with a newer codec.".to_string(),
            })?;
        if parsed.contains(RowFlags::TOMBSTONE | RowFlags::MERGE_OPERAND) {
            return Err(SlateDBError::InvalidRowFlags {
                encoded_bits: parsed.bits(),
                known_bits: RowFlags::all().bits(),
                message: "Tombstone and Merge Operand are mutually exclusive.".to_string(),
            });
        }
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    /// Helper to create ValueDeletable from a tag and optional data
    fn make_value(tag: &str, data: Option<Vec<u8>>) -> ValueDeletable {
        match tag {
            "value" => ValueDeletable::Value(Bytes::from(data.unwrap())),
            "tombstone" => ValueDeletable::Tombstone,
            "merge" => ValueDeletable::Merge(Bytes::from(data.unwrap())),
            _ => panic!("unknown value tag: {}", tag),
        }
    }

    #[rstest]
    #[case("value entry", 3, b"suffix".to_vec(), 100, "value", Some(b"value".to_vec()), None, None)]
    #[case("entry with timestamps", 0, b"key".to_vec(), 1, "value", Some(b"val".to_vec()), Some(1000), Some(2000))]
    #[case("tombstone", 5, b"tomb".to_vec(), 42, "tombstone", None, Some(500), None)]
    #[case("tombstone with create_ts", 0, b"deleted_key".to_vec(), 999, "tombstone", None, Some(12345), None)]
    #[case("merge operand", 0, b"merge_key".to_vec(), 42, "merge", Some(b"merge_value".to_vec()), None, Some(1000))]
    #[case("2-byte varints", 200, vec![b'k'; 200], 1, "value", Some(vec![b'v'; 200]), None, None)]
    #[case("3-byte varints", 20000, vec![b'k'; 20000], 1, "value", Some(vec![b'v'; 20000]), None, None)]
    fn should_encode_decode_round_trip(
        #[case] _name: &str,
        #[case] shared_bytes: u32,
        #[case] key_suffix: Vec<u8>,
        #[case] seq: u64,
        #[case] value_tag: &str,
        #[case] value_data: Option<Vec<u8>>,
        #[case] create_ts: Option<i64>,
        #[case] expire_ts: Option<i64>,
    ) {
        // given: a row entry
        let value = make_value(value_tag, value_data);

        let entry = SstRowEntryV2::new(
            shared_bytes,
            Bytes::from(key_suffix.clone()),
            seq,
            value.clone(),
            create_ts,
            expire_ts,
        );

        let codec = SstRowCodecV2::new();
        let mut buf = Vec::new();

        // when: encoding and decoding
        codec.encode(&mut buf, &entry);
        let mut slice = buf.as_slice();
        let decoded = codec.decode(&mut slice).expect("decode failed");

        // then: all fields match
        assert_eq!(decoded.shared_bytes, shared_bytes);
        assert_eq!(decoded.key_suffix.as_ref(), key_suffix.as_slice());
        assert_eq!(decoded.seq, seq);
        assert_eq!(decoded.value, value);
        assert_eq!(decoded.create_ts, create_ts);
        assert_eq!(decoded.expire_ts, expire_ts);
    }

    #[rstest]
    #[case("basic", 5, b"suffix".to_vec(), "value", Some(b"value".to_vec()))]
    #[case("at restart point", 0, b"full_key_at_restart".to_vec(), "value", Some(b"value".to_vec()))]
    #[case("empty suffix", 10, b"".to_vec(), "value", Some(b"value".to_vec()))]
    #[case("large shared bytes", 16384, b"x".to_vec(), "value", Some(b"v".to_vec()))]
    #[case("with tombstone", 3, b"deleted".to_vec(), "tombstone", None)]
    #[case("large key suffix", 0, vec![b'k'; 1000], "value", Some(b"v".to_vec()))]
    fn should_decode_key_only(
        #[case] _name: &str,
        #[case] shared_bytes: u32,
        #[case] key_suffix: Vec<u8>,
        #[case] value_tag: &str,
        #[case] value_data: Option<Vec<u8>>,
    ) {
        // given: an encoded entry
        let value = make_value(value_tag, value_data);
        let entry = SstRowEntryV2::new(
            shared_bytes,
            Bytes::from(key_suffix.clone()),
            1,
            value,
            None,
            None,
        );

        let codec = SstRowCodecV2::new();
        let mut buf = Vec::new();
        codec.encode(&mut buf, &entry);

        // when: decoding key only
        let mut slice = buf.as_slice();
        let (decoded_shared, decoded_suffix) = codec.decode_key_only(&mut slice);

        // then: key info is correct
        assert_eq!(decoded_shared, shared_bytes);
        assert_eq!(decoded_suffix.as_ref(), key_suffix.as_slice());
    }

    #[test]
    fn should_encode_with_varints() {
        // given: an entry with known values
        let entry = SstRowEntryV2::new(
            3,
            Bytes::from("abc"),
            1,
            ValueDeletable::Value(Bytes::from("xyz")),
            None,
            None,
        );

        let codec = SstRowCodecV2::new();
        let mut buf = Vec::new();

        // when: encoding
        codec.encode(&mut buf, &entry);

        // then: varints are at the start
        // shared_bytes=3 -> 0x03 (1 byte)
        // unshared_bytes=3 -> 0x03 (1 byte)
        // value_len=3 -> 0x03 (1 byte)
        assert_eq!(buf[0], 3);
        assert_eq!(buf[1], 3);
        assert_eq!(buf[2], 3);
        // key_delta follows
        assert_eq!(&buf[3..6], b"abc");
        // value follows
        assert_eq!(&buf[6..9], b"xyz");
    }

    #[test]
    fn should_restore_full_key_from_previous() {
        // given: an entry with shared bytes
        let previous_key = b"shared_prefix_different_suffix";
        let entry = SstRowEntryV2::new(
            13, // "shared_prefix"
            Bytes::from("_new_suffix"),
            1,
            ValueDeletable::Value(Bytes::from("val")),
            None,
            None,
        );

        // when: restoring full key
        let full_key = entry.restore_full_key(previous_key);

        // then: key is correctly reconstructed
        assert_eq!(full_key.as_ref(), b"shared_prefix_new_suffix");
    }

    #[test]
    fn should_decode_key_only_advance_buffer_correctly() {
        // given: an encoded entry
        let entry = SstRowEntryV2::new(
            2,
            Bytes::from("key"),
            1,
            ValueDeletable::Value(Bytes::from("value")),
            None,
            None,
        );

        let codec = SstRowCodecV2::new();
        let mut buf = Vec::new();
        codec.encode(&mut buf, &entry);
        let original_len = buf.len();

        // when: decoding key only
        let mut slice = buf.as_slice();
        let _ = codec.decode_key_only(&mut slice);

        // then: buffer is advanced past the varints and key_suffix
        // shared_bytes(1) + unshared_bytes(1) + value_len(1) + key_suffix(3) = 6 bytes consumed
        let consumed = original_len - slice.len();
        assert_eq!(consumed, 6); // 1 + 1 + 1 + 3 = 6
    }

    #[test]
    fn should_calculate_encoded_size_correctly() {
        // given: entries with various configurations
        let entry = SstRowEntryV2::new(
            3,
            Bytes::from("abc"),
            1,
            ValueDeletable::Value(Bytes::from("xyz")),
            None,
            None,
        );

        // when: calculating encoded size
        let size = entry.encoded_size();

        // then: size matches actual encoding
        let codec = SstRowCodecV2::new();
        let mut buf = Vec::new();
        codec.encode(&mut buf, &entry);
        assert_eq!(size, buf.len());
    }

    /// Strategy to generate arbitrary ValueDeletable
    fn arb_value_deletable() -> impl Strategy<Value = ValueDeletable> {
        prop_oneof![
            prop::collection::vec(any::<u8>(), 0..1024)
                .prop_map(|v| ValueDeletable::Value(Bytes::from(v))),
            prop::collection::vec(any::<u8>(), 0..1024)
                .prop_map(|v| ValueDeletable::Merge(Bytes::from(v))),
            Just(ValueDeletable::Tombstone),
        ]
    }

    /// Strategy to generate arbitrary optional timestamps
    fn arb_optional_timestamp() -> impl Strategy<Value = Option<i64>> {
        prop_oneof![Just(None), any::<i64>().prop_map(Some),]
    }

    proptest! {
        #[test]
        fn should_encode_decode_round_trip_proptest(
            shared_bytes in any::<u32>(),
            key_suffix in prop::collection::vec(any::<u8>(), 0..1024),
            seq in any::<u64>(),
            value in arb_value_deletable(),
            create_ts in arb_optional_timestamp(),
            expire_ts in arb_optional_timestamp(),
        ) {
            // given: an arbitrary row entry
            let entry = SstRowEntryV2::new(
                shared_bytes,
                Bytes::from(key_suffix.clone()),
                seq,
                value.clone(),
                create_ts,
                expire_ts,
            );

            let codec = SstRowCodecV2::new();
            let mut buf = Vec::new();

            // when: encoding and decoding
            codec.encode(&mut buf, &entry);
            let mut slice = buf.as_slice();
            let decoded = codec.decode(&mut slice).expect("decode failed");

            // then: all fields match
            prop_assert_eq!(decoded.shared_bytes, shared_bytes);
            prop_assert_eq!(decoded.key_suffix.as_ref(), key_suffix.as_slice());
            prop_assert_eq!(decoded.seq, seq);
            prop_assert_eq!(decoded.value, value);
            prop_assert_eq!(decoded.create_ts, create_ts);
            prop_assert_eq!(decoded.expire_ts, expire_ts);

            // and: encoded_size matches actual size
            prop_assert_eq!(entry.encoded_size(), buf.len());
        }
    }
}
