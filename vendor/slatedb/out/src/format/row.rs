use std::fmt::Debug;

use crate::error::SlateDBError;
use crate::types::ValueDeletable;
use bitflags::bitflags;
use bytes::{Buf, BufMut, Bytes, BytesMut};

bitflags! {
    #[derive(Debug, Clone, PartialEq, Default)]
    pub(crate) struct RowFlags: u8 {
        const TOMBSTONE = 0b00000001;
        const HAS_EXPIRE_TS = 0b00000010;
        const HAS_CREATE_TS = 0b00000100;
        const MERGE_OPERAND = 0b00001000;
    }
}

/// Encodes key and value using the binary codec for SlateDB row representation
/// using the `v0` encoding scheme.
///
/// The `v0` codec for the key is (for non-tombstones):
///
/// ```txt
///  |---------------------------------------------------------------------------------------------------------------------|
///  |       u16      |      u16       |  var        | u64     | u8        | i64       | i64       | u32       |  var      |
///  |----------------|----------------|-------------|---------|-----------|-----------|-----------|-----------|-----------|
///  | key_prefix_len | key_suffix_len |  key_suffix | seq     | flags     | expire_ts | create_ts | value_len | value     |
///  |---------------------------------------------------------------------------------------------------------------------|
/// ```
///
/// And for tombstones (flags & Tombstone == 1):
///
///  ```txt
///  |----------------------------------------------------------|-----------|-----------|
///  |       u16      |      u16       |  var        | u64      | u8        | i64       |
///  |----------------|----------------|-------------|----------|-----------|-----------|
///  | key_prefix_len | key_suffix_len |  key_suffix | seq      | flags     | create_ts |
///  |----------------------------------------------------------------------------------|
///  ```
///
/// | Field            | Type  | Description                                            |
/// |------------------|-------|--------------------------------------------------------|
/// | `key_prefix_len` | `u16` | Length of the key prefix                               |
/// | `key_suffix_len` | `u16` | Length of the key suffix                               |
/// | `key_suffix`     | `var` | Suffix of the key                                      |
/// | `seq`            | `u64` | Sequence Number                                        |
/// | `flags`          | `u8`  | Flags of the row                                       |
/// | `expire_ts`      | `u64` | Optional, only has value when flags & HAS_EXPIRE_TS    |
/// | `create_ts`      | `u64` | Optional, only has value when flags & HAS_CREATE_TS    |
/// | `value_len`      | `u32` | Length of the value                                    |
/// | `value`          | `var` | Value bytes                                            |

#[derive(Debug, Clone)]
pub(crate) struct SstRowEntry {
    pub key_prefix_len: usize,
    pub key_suffix: Bytes,
    pub seq: u64,
    pub expire_ts: Option<i64>,
    pub create_ts: Option<i64>,
    pub value: ValueDeletable,
}

impl SstRowEntry {
    pub(crate) fn new(
        key_prefix_len: usize,
        key_suffix: Bytes,
        seq: u64,
        value: ValueDeletable,
        create_ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> Self {
        let key_suffix_len = key_suffix.len();
        assert!(
            key_prefix_len <= (u16::MAX as usize),
            "key_prefix_len > u16"
        );
        assert!(
            key_suffix_len <= (u16::MAX as usize),
            "key_suffix.len() > u16"
        );
        assert!(
            key_prefix_len + key_suffix_len <= (u16::MAX as usize),
            "key_prefix_len + key_suffix.len() > u16"
        );
        assert!(value.len() <= (u32::MAX as usize), "value.len() > u32");
        Self {
            key_prefix_len,
            key_suffix,
            seq,
            create_ts,
            expire_ts,
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

    #[cfg(test)]
    pub(crate) fn size(&self) -> usize {
        let mut size = 2  // u16 key_prefix_len
        + 2 // u16 key_suffix_len
        + self.key_suffix.len() // key_suffix
        + 8  // u64 seq
        + 1; // u8 flags
        if self.expire_ts.is_some() {
            size += 8; // i64 expire_ts
        }
        if self.create_ts.is_some() {
            size += 8; // i64 create_ts
        }
        if !matches!(self.value, ValueDeletable::Tombstone) {
            size += 4; // u32 value_len
            size += self.value.len(); // value
        }
        size
    }

    /// Keys in a Block are stored with prefix stripped off to compress the storage size. This function
    /// restores the full key by prepending the prefix to the key suffix.
    pub(crate) fn restore_full_key(&self, prefix: &Bytes) -> Bytes {
        let mut full_key = BytesMut::with_capacity(self.key_prefix_len + self.key_suffix.len());
        full_key.extend_from_slice(&prefix[..self.key_prefix_len]);
        full_key.extend_from_slice(&self.key_suffix);
        full_key.freeze()
    }
}

pub(crate) struct SstRowCodecV0 {}

impl SstRowCodecV0 {
    pub(crate) fn new() -> Self {
        Self {}
    }

    /// estimated_entries_size include the size of seqnum,create_ts(if exist),expire_ts(exist),key,value
    pub(crate) fn estimate_encoded_size(entry_num: usize, estimated_entries_size: usize) -> usize {
        let key_prefix_len_size = size_of::<u16>();
        let key_suffix_len_size = size_of::<u16>();
        let value_len_size = size_of::<u32>();
        let flag_size = size_of::<u8>();
        let mut ans = estimated_entries_size;
        ans += (key_prefix_len_size + key_suffix_len_size + value_len_size + flag_size) * entry_num;
        ans
    }

    pub(crate) fn encode(&self, output: &mut Vec<u8>, row: &SstRowEntry) {
        output.put_u16(row.key_prefix_len.try_into().expect("key_prefix_len > u16"));
        output.put_u16(
            row.key_suffix
                .len()
                .try_into()
                .expect("key_suffix.len() > u16"),
        );
        output.put(row.key_suffix.as_ref());

        // encode seq & flags
        let flags = row.flags();
        output.put_u64(row.seq);
        output.put_u8(flags.bits());

        // encode expire & create ts
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

        match &row.value {
            ValueDeletable::Value(v) | ValueDeletable::Merge(v) => {
                let value_len = u32::try_from(v.len()).expect("value len > u32");
                output.put_u32(value_len);
                output.put(v.as_ref());
            }
            ValueDeletable::Tombstone => {
                // skip encoding value for tombstone
            }
        }
    }

    pub(crate) fn decode(&self, data: &mut Bytes) -> Result<SstRowEntry, SlateDBError> {
        let key_prefix_len = data.get_u16() as usize;
        let key_suffix_len = data.get_u16() as usize;
        let key_suffix = data.slice(..key_suffix_len);
        data.advance(key_suffix_len);

        // decode seq & flags
        let seq = data.get_u64();
        let flags = self.decode_flags(data.get_u8())?;

        // decode expire_ts & create_ts
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

        // skip decoding value for tombstone.
        if flags.contains(RowFlags::TOMBSTONE) {
            return Ok(SstRowEntry::new(
                key_prefix_len,
                key_suffix,
                seq,
                ValueDeletable::Tombstone,
                create_ts,
                None,
            ));
        }

        // decode value
        let value_len = data.get_u32() as usize;
        let value = data.slice(..value_len);
        Ok(SstRowEntry {
            key_prefix_len,
            key_suffix,
            seq,
            expire_ts,
            create_ts,
            value: if flags.contains(RowFlags::MERGE_OPERAND) {
                ValueDeletable::Merge(value)
            } else {
                ValueDeletable::Value(value)
            },
        })
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
    use crate::test_utils::assert_debug_snapshot;
    use crate::types::ValueDeletable;
    use rstest::rstest;

    #[derive(Debug)]
    struct CodecTestCase {
        name: &'static str,
        key_prefix_len: usize,
        key_suffix: Vec<u8>,
        seq: u64,
        value: Option<Vec<u8>>,
        create_ts: Option<i64>,
        expire_ts: Option<i64>,
        first_key: Vec<u8>,
    }

    #[rstest]
    #[case(CodecTestCase {
        name: "normal row with expire_ts",
        key_prefix_len: 3,
        key_suffix: b"key".to_vec(),
        seq: 1,
        value: Some(b"value".to_vec()),
        create_ts: None,
        expire_ts: Some(10),
        first_key: b"prefixdata".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "normal row without expire_ts",
        key_prefix_len: 0,
        key_suffix: b"key".to_vec(),
        seq: 1,
        value: Some(b"value".to_vec()),
        create_ts: None,
        expire_ts: None,
        first_key: b"".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "row with both timestamps",
        key_prefix_len: 5,
        key_suffix: b"both".to_vec(),
        seq: 100,
        value: Some(b"value".to_vec()),
        create_ts: Some(1234567890),
        expire_ts: Some(9876543210),
        first_key: b"test_both".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "row with only create_ts",
        key_prefix_len: 4,
        key_suffix: b"create".to_vec(),
        seq: 50,
        value: Some(b"test_value".to_vec()),
        create_ts: Some(1234567890),
        expire_ts: None,
        first_key: b"timecreate".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "tombstone row",
        key_prefix_len: 4,
        key_suffix: b"tomb".to_vec(),
        seq: 1,
        value: None,
        create_ts: Some(2),
        expire_ts: Some(1),
        first_key: b"deadbeefdata".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "empty key suffix",
        key_prefix_len: 4,
        key_suffix: b"".to_vec(),
        seq: 1,
        value: Some(b"value".to_vec()),
        create_ts: None,
        expire_ts: None,
        first_key: b"keyprefixdata".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "large sequence number",
        key_prefix_len: 3,
        key_suffix: b"seq".to_vec(),
        seq: u64::MAX,
        value: Some(b"value".to_vec()),
        create_ts: None,
        expire_ts: None,
        first_key: b"bigseq".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "large value",
        key_prefix_len: 2,
        key_suffix: b"big".to_vec(),
        seq: 1,
        value: Some(vec![b'x'; 100]),
        create_ts: None,
        expire_ts: None,
        first_key: b"bigvalue".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "long key suffix",
        key_prefix_len: 2,
        key_suffix: vec![b'k'; 100],
        seq: 1,
        value: Some(b"value".to_vec()),
        create_ts: None,
        expire_ts: None,
        first_key: b"longkey".to_vec(),
    })]
    #[case(CodecTestCase {
        name: "unicode key suffix",
        key_prefix_len: 3,
        key_suffix: "你好世界".as_bytes().to_vec(),
        seq: 1,
        value: Some(b"value".to_vec()),
        create_ts: None,
        expire_ts: None,
        first_key: b"unicode".to_vec(),
    })]
    #[should_panic(expected = "key_suffix.len() > u16")]
    #[case(CodecTestCase {
        name: "large key suffix",
        key_prefix_len: 0,
        key_suffix: vec![b'k'; u16::MAX as usize + 1], // 2^16
        seq: 1,
        value: Some(vec![b'x'; 100]),
        create_ts: None,
        expire_ts: None,
        first_key: vec![b'k'; u16::MAX as usize + 1], // 2^16
    })]
    #[should_panic(expected = "key_prefix_len > u16")]
    #[case(CodecTestCase {
        name: "large key prefix",
        key_prefix_len: u16::MAX as usize + 1, // 2^16
        key_suffix: vec![b'k'; 1024],
        seq: 1,
        value: Some(vec![b'x'; 100]),
        create_ts: None,
        expire_ts: None,
        first_key: vec![b'k'; 65_536], // 2^16 + 100
    })]
    #[should_panic(expected = "key_prefix_len + key_suffix.len() > u16")]
    #[case(CodecTestCase {
        name: "large key",
        key_prefix_len: u16::MAX as usize, // 2^16 - 1
        key_suffix: vec![b'k'; 1],
        seq: 1,
        value: Some(vec![b'x'; 100]),
        create_ts: None,
        expire_ts: None,
        first_key: vec![b'k'; 65_536], // 2^16
    })]
    #[should_panic(expected = "value.len() > u32")]
    #[case(CodecTestCase {
        name: "large value",
        key_prefix_len: 0,
        key_suffix: vec![b'k'; 1024],
        seq: 1,
        value: Some(vec![b'x'; u32::MAX as usize + 1]), // 2^32
        create_ts: None,
        expire_ts: None,
        first_key: vec![b'k'; 1024],
    })]
    fn test_encode_decode(#[case] test_case: CodecTestCase) {
        let mut encoded_data = Vec::new();
        let codec = SstRowCodecV0 {};

        // Encode the row
        let value = match test_case.value {
            Some(v) => ValueDeletable::Value(Bytes::from(v)),
            None => ValueDeletable::Tombstone,
        };

        codec.encode(
            &mut encoded_data,
            &SstRowEntry::new(
                test_case.key_prefix_len,
                Bytes::from(test_case.key_suffix),
                test_case.seq,
                value,
                test_case.create_ts,
                test_case.expire_ts,
            ),
        );

        let mut data = Bytes::from(encoded_data.clone());
        let decoded = codec.decode(&mut data).expect("decoding failed");
        let output = (
            test_case.name,
            String::from_utf8_lossy(&encoded_data),
            decoded.clone(),
            decoded.restore_full_key(&Bytes::from(test_case.first_key)),
        );

        assert_debug_snapshot!(test_case.name, output);
    }

    #[test]
    fn test_decode_invalid_flags() {
        let codec = SstRowCodecV0::new();
        let mut tests: Vec<u8> = Vec::new();

        // Tombstone and Merge Operand are mutually exclusive
        tests.push(0b00001001);
        // Unknown bits
        tests.push(0b00010000);
        tests.push(0b00100000);
        tests.push(0b01000000);
        tests.push(0b10000000);

        for invalid_flags in tests.iter() {
            let err = codec.decode_flags(*invalid_flags).unwrap_err();
            assert!(matches!(err, SlateDBError::InvalidRowFlags { .. }));
        }
    }

    #[test]
    fn test_decode_invalid_flags_from_row() {
        let mut encoded_data = Vec::new();
        let key_prefix_len = 3;
        let key_suffix = b"bad".as_slice();

        // Manually encode invalid flags
        encoded_data.put_u16(key_prefix_len as u16);
        encoded_data.put_u16(key_suffix.len() as u16);
        encoded_data.put(key_suffix);
        encoded_data.put_u64(100);
        encoded_data.put_u8(0xFF); // Invalid flag
        encoded_data.put_u32(4);
        encoded_data.put(b"data".as_slice());

        let mut data = Bytes::from(encoded_data);

        // Attempt to decode the row
        let codec = SstRowCodecV0::new();
        let err = codec
            .decode(&mut data)
            .map(|_| "decoded entry")
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidRowFlags { .. }));
    }

    #[test]
    fn test_encode_decode_merge_row() {
        let mut encoded_data = Vec::new();
        let key_prefix_len = 5;
        let key_suffix = b"merge";
        let value: &[u8] = b"value";

        // Encode the row
        let codec = SstRowCodecV0::new();
        codec.encode(
            &mut encoded_data,
            &SstRowEntry::new(
                key_prefix_len,
                Bytes::from(key_suffix.to_vec()),
                1,
                ValueDeletable::Merge(Bytes::from(value)),
                Some(2),
                Some(1),
            ),
        );

        let first_key = Bytes::from(b"happybeefdata".as_ref());
        let mut data = Bytes::from(encoded_data);
        let decoded = codec.decode(&mut data).expect("decoding failed");

        // Expected key: first_key[..5] + "merge" = "happymerge"
        let expected_key = Bytes::from(b"happymerge" as &[u8]);
        let expected_value = ValueDeletable::Merge(Bytes::from(value));

        assert_eq!(decoded.restore_full_key(&first_key), &expected_key);
        assert_eq!(decoded.value, expected_value);
        assert_eq!(decoded.expire_ts, Some(1));
        assert_eq!(decoded.create_ts, Some(2));
        assert_eq!(decoded.size(), 43);
    }

    #[test]
    fn test_estimate_encoded_size() {
        // Test with zero entries
        assert_eq!(SstRowCodecV0::estimate_encoded_size(0, 0), 0);
        assert_eq!(SstRowCodecV0::estimate_encoded_size(0, 100), 100);

        // Test with one entry
        let entry_size = 50; // Example entry size
        let expected_size = entry_size + (2 + 2 + 4 + 1); // key_prefix_len + key_suffix_len + value_len + flag
        assert_eq!(
            SstRowCodecV0::estimate_encoded_size(1, entry_size),
            expected_size
        );

        // Test with multiple entries
        let num_entries = 5;
        let total_entry_size = entry_size * num_entries;
        let expected_size = total_entry_size + (2 + 2 + 4 + 1) * num_entries;
        assert_eq!(
            SstRowCodecV0::estimate_encoded_size(num_entries, total_entry_size),
            expected_size
        );

        // Test with large numbers
        let large_num_entries = 10000;
        let large_entry_size = 10000;
        let total_large_size = large_entry_size * large_num_entries;
        let expected_large_size = total_large_size + (2 + 2 + 4 + 1) * large_num_entries;
        assert_eq!(
            SstRowCodecV0::estimate_encoded_size(large_num_entries, total_large_size),
            expected_large_size
        );
    }
}
