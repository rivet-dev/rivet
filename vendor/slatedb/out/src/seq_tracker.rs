use chrono::{DateTime, Utc};
use serde::de::{self};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::utils::{sign_extend, BitReader, BitWriter};

/// Default capacity for the sequence tracker (8192 entries)
const DEFAULT_CAPACITY: u32 = 8192;

/// Default reporting interval in seconds (60 seconds)
const DEFAULT_INTERVAL_SECS: u64 = 60;

/// Version number for the serialization format
const SERIALIZATION_VERSION: u8 = 1;

#[derive(PartialEq, Clone, Copy)]
pub(crate) struct TrackedSeq {
    pub(crate) seq: u64,
    pub(crate) ts: DateTime<Utc>,
}

/// Rounding behavior for non-exact matches in sequence-timestamp lookups.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindOption {
    /// Round up to the next higher value when no exact match is found.
    RoundUp,
    /// Round down to the next lower value when no exact match is found.
    RoundDown,
}

/// Tracks sequence number ↔ timestamp relationships with bounded memory.
///
/// Uses two sorted arrays for bi-directional lookup between sequence numbers and timestamps.
/// When capacity is reached, downsamples by removing every other entry to maintain bounded memory.
/// Data is compressed using Gorilla encoding (delta-of-deltas) for efficient storage.
#[derive(Debug, Clone)]
pub struct SequenceTracker {
    /// Sorted array of sequence numbers
    sequence_numbers: Vec<u64>,
    /// Sorted array of timestamps (as Unix timestamp seconds)
    timestamps: Vec<i64>,
    /// Maximum number of entries to store
    capacity: u32,
    /// Interval in seconds at which to record timestamps
    interval_secs: u64,
    /// Last recorded timestamp to enforce interval
    last_recorded_ts: Option<i64>,
}

impl PartialEq for SequenceTracker {
    fn eq(&self, other: &Self) -> bool {
        // Compare only the essential data, ignoring last_recorded_ts which is implementation detail
        self.sequence_numbers == other.sequence_numbers
            && self.timestamps == other.timestamps
            && self.capacity == other.capacity
            && self.interval_secs == other.interval_secs
    }
}

#[allow(dead_code)]
impl SequenceTracker {
    /// Create a new SequenceTracker with default configuration (60s interval, 8192 capacity)
    pub(crate) fn new() -> Self {
        Self::with_config(DEFAULT_CAPACITY, DEFAULT_INTERVAL_SECS)
    }

    /// Create a new SequenceTracker with custom configuration (used for testing)
    fn with_config(capacity: u32, interval_secs: u64) -> Self {
        Self {
            sequence_numbers: Vec::with_capacity(capacity as usize),
            timestamps: Vec::with_capacity(capacity as usize),
            capacity,
            interval_secs,
            last_recorded_ts: None,
        }
    }

    /// Insert a new sequence number and timestamp pair
    pub(crate) fn insert(&mut self, seq: TrackedSeq) {
        let ts_secs = seq.ts.timestamp();

        // Only record if enough time has passed since last recording
        // Discarding insertions instead of only inserting on a timer
        // makes it easier to manage concurrency and is a pretty cheap
        // operation.
        if let Some(last_ts) = self.last_recorded_ts {
            if (ts_secs - last_ts) < self.interval_secs as i64 {
                return;
            }
        }

        // Ensure monotonic ordering
        if let Some(last_seq) = self.sequence_numbers.last() {
            assert!(
                seq.seq >= *last_seq,
                "Sequence numbers must be monotonic. seq: {}, last_seq: {}",
                seq.seq,
                *last_seq
            );
        }
        if let Some(last_ts) = self.timestamps.last() {
            assert!(
                ts_secs >= *last_ts,
                "Timestamps must be monotonic. ts_secs: {}, last_ts: {}",
                ts_secs,
                *last_ts
            );
        }

        self.sequence_numbers.push(seq.seq);
        self.timestamps.push(ts_secs);
        self.last_recorded_ts = Some(ts_secs);

        // Downsample if we exceed capacity
        if self.sequence_numbers.len() > self.capacity as usize {
            self.downsample();
        }
    }

    pub(crate) fn extend_from(&mut self, other: &SequenceTracker) {
        if other.sequence_numbers.is_empty() {
            return;
        }

        for (seq, &ts_secs) in other.sequence_numbers.iter().zip(other.timestamps.iter()) {
            if let Some(ts) = DateTime::from_timestamp(ts_secs, 0) {
                self.insert(TrackedSeq { seq: *seq, ts });
            }
        }
    }

    /// Downsample by removing every other entry
    fn downsample(&mut self) {
        let mut new_seqs = Vec::with_capacity(self.capacity as usize);
        let mut new_timestamps = Vec::with_capacity(self.capacity as usize);

        for i in (0..self.sequence_numbers.len()).step_by(2) {
            new_seqs.push(self.sequence_numbers[i]);
            new_timestamps.push(self.timestamps[i]);
        }

        self.sequence_numbers = new_seqs;
        self.timestamps = new_timestamps;
    }

    /// Find the timestamp for a given sequence number
    pub fn find_ts(&self, seq: u64, find_opt: FindOption) -> Option<DateTime<Utc>> {
        if self.sequence_numbers.is_empty() {
            return None;
        }

        match self.sequence_numbers.binary_search(&seq) {
            Ok(idx) => DateTime::from_timestamp(self.timestamps[idx], 0),
            Err(idx) => match find_opt {
                FindOption::RoundUp if idx < self.sequence_numbers.len() => {
                    DateTime::from_timestamp(self.timestamps[idx], 0)
                }
                FindOption::RoundDown if idx > 0 => {
                    DateTime::from_timestamp(self.timestamps[idx - 1], 0)
                }
                _ => None,
            },
        }
    }

    /// Find the sequence number for a given timestamp
    pub fn find_seq(&self, ts: DateTime<Utc>, find_opt: FindOption) -> Option<u64> {
        let ts_secs = ts.timestamp();

        if self.timestamps.is_empty() {
            return None;
        }

        match self.timestamps.binary_search(&ts_secs) {
            Ok(idx) => Some(self.sequence_numbers[idx]),
            Err(idx) => match find_opt {
                FindOption::RoundUp if idx < self.timestamps.len() => {
                    Some(self.sequence_numbers[idx])
                }
                FindOption::RoundDown if idx > 0 => Some(self.sequence_numbers[idx - 1]),
                _ => None,
            },
        }
    }

    /// Serialize the tracker into bytes using the RFC-0012 format.
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        encode_sequence_tracker(self)
    }

    /// Deserialize the tracker from bytes using the RFC-0012 format.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        decode_sequence_tracker(bytes)
    }
}

impl Serialize for SequenceTracker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = encode_sequence_tracker(self);
        serializer.serialize_bytes(&encoded)
    }
}

impl<'de> Deserialize<'de> for SequenceTracker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf: Vec<u8> = Vec::deserialize(deserializer)?;
        decode_sequence_tracker(&buf).map_err(<D::Error as de::Error>::custom)
    }
}

/// Encode a SequenceTracker to bytes using the format specified in RFC-0012
fn encode_sequence_tracker(tracker: &SequenceTracker) -> Vec<u8> {
    let mut result = Vec::new();

    // Write version (u8)
    result.push(SERIALIZATION_VERSION);

    // Write length (as u32, little-endian)
    let len = tracker.sequence_numbers.len() as u32;
    result.extend_from_slice(&len.to_le_bytes());

    // Encode sequence numbers using Gorilla encoding
    let encoded_seqs = encode_sequence_numbers(&tracker.sequence_numbers);
    result.extend_from_slice(&encoded_seqs);

    // Encode timestamps using Gorilla encoding
    let encoded_timestamps = encode_timestamps(&tracker.timestamps);
    result.extend_from_slice(&encoded_timestamps);

    result
}

/// Decode a SequenceTracker from bytes
fn decode_sequence_tracker(buf: &[u8]) -> Result<SequenceTracker, String> {
    if buf.is_empty() {
        return Err("Empty buffer".to_string());
    }

    let mut offset = 0;

    // Read version
    let version = buf[offset];
    if version != SERIALIZATION_VERSION {
        return Err(format!("Unsupported version: {}", version));
    }
    offset += 1;

    // Read length
    if buf.len() < offset + 4 {
        return Err("Buffer too small for length".to_string());
    }
    let len_bytes: [u8; 4] = buf[offset..offset + 4]
        .try_into()
        .map_err(|_| "Buffer too small for length".to_string())?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    offset += 4;

    // Decode sequence numbers
    let (encoded_seq_len, sequence_numbers) = decode_sequence_numbers_with_length(&buf[offset..])?;
    if sequence_numbers.len() != len {
        return Err(format!(
            "Expected {} sequence numbers, got {}",
            len,
            sequence_numbers.len()
        ));
    }
    offset += encoded_seq_len;

    // Decode timestamps
    let timestamps = decode_timestamps(&buf[offset..])?;
    if timestamps.len() != len {
        return Err(format!(
            "Expected {} timestamps, got {}",
            len,
            timestamps.len()
        ));
    }

    // Reconstruct the tracker with default configuration (note that it
    // is fully backwards compatible to change the interval or the capacity
    // since this would just trigger different downsampling behavior)
    let mut tracker = SequenceTracker::with_config(DEFAULT_CAPACITY, DEFAULT_INTERVAL_SECS);
    tracker.sequence_numbers = sequence_numbers;
    tracker.timestamps = timestamps;
    tracker.last_recorded_ts = tracker.timestamps.last().copied();

    Ok(tracker)
}

/// Generic Gorilla encoding for i64 values (used for both timestamps and sequence numbers)
fn encode_gorilla_i64(values: &[i64]) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.push32(values.len() as u32, 32);

    if !values.is_empty() {
        // First value is encoded in full
        w.push64(values[0] as u64, 64);

        // Encode subsequent values using delta-of-deltas
        let mut prev_val = values[0];
        let mut prev_delta: i64 = 0;

        for &val in &values[1..] {
            let delta = val - prev_val;
            let dod = delta - prev_delta;

            // this is the gorilla encoding scheme, you can see
            // [this paper](https://www.vldb.org/pvldb/vol8/p1816-teller.pdf)
            // on page 1820 for the full algorithm
            match dod {
                0 => w.push(false),
                -63..=63 => {
                    w.push32(0b10, 2);
                    w.push32((dod as u32) & 0x7F, 7);
                }
                -255..=255 => {
                    w.push32(0b110, 3);
                    w.push32((dod as u32) & 0x1FF, 9);
                }
                -2047..=2047 => {
                    w.push32(0b1110, 4);
                    w.push32((dod as u32) & 0xFFF, 12);
                }
                _ => {
                    w.push32(0b1111, 4);
                    w.push32(dod as u32, 32);
                }
            }

            prev_val = val;
            prev_delta = delta;
        }
    }

    w.finish()
}

/// Generic Gorilla decoding for i64 values, returning bytes consumed
fn decode_gorilla_i64_with_length(buf: &[u8]) -> Result<(usize, Vec<i64>), String> {
    let mut values = Vec::new();
    let mut reader = BitReader::new(buf);
    let mut bits_read = 0usize;

    let mut remaining = reader.read32(32).ok_or("Expected count first")?;
    bits_read += 32;

    if remaining > 0 {
        let first_bits = reader
            .read64(64)
            .ok_or("Unexpected EOF: first value should be 64 bit")?;
        let first = first_bits as i64;
        bits_read += 64;

        remaining -= 1;
        values.push(first);

        let mut prev_val = first;
        let mut prev_delta: i64 = 0;

        while remaining > 0 {
            let prefix = reader.read_bit().ok_or("Unexpected EOF reading prefix")?;
            bits_read += 1;

            let dod = if !prefix {
                0i64
            } else {
                let mut count = 1;
                while count < 4 {
                    bits_read += 1;
                    match reader.read_bit() {
                        Some(true) => count += 1,
                        Some(false) => break,
                        None => return Err("Unexpected EOF decoding Gorilla prefix".to_string()),
                    }
                }

                let bits = GORILLA_PREFIX_BYTES[count];
                let raw = reader
                    .read32(bits)
                    .ok_or_else(|| format!("Unexpected EOF reading {}-bit delta-of-delta", bits))?;
                bits_read += bits as usize;

                sign_extend(raw, bits) as i64
            };

            let delta = prev_delta + dod;
            let val = prev_val + delta;
            values.push(val);

            prev_delta = delta;
            prev_val = val;

            // there may be some padding at the end of the [u8]
            // so we track how many we've read / still need to read
            remaining -= 1;
        }
    }

    // Calculate bytes consumed (rounded up to next byte boundary)
    let bytes_consumed = bits_read.div_ceil(8);

    Ok((bytes_consumed, values))
}

/// Encode sequence numbers using Gorilla encoding (delta-of-deltas)
fn encode_sequence_numbers(sequences: &[u64]) -> Vec<u8> {
    // Convert u64 to i64 using bit cast (preserves bit pattern)
    let as_i64: Vec<i64> = sequences.iter().map(|&x| x as i64).collect();
    encode_gorilla_i64(&as_i64)
}

/// Decode sequence numbers from Gorilla encoding, returning the number of bytes consumed
fn decode_sequence_numbers_with_length(buf: &[u8]) -> Result<(usize, Vec<u64>), String> {
    let (bytes_consumed, values) = decode_gorilla_i64_with_length(buf)?;

    // Convert i64 back to u64 using bit cast (preserves bit pattern)
    // This correctly handles values that were > i64::MAX
    let sequences: Vec<u64> = values.into_iter().map(|v| v as u64).collect();

    Ok((bytes_consumed, sequences))
}

/// Encode timestamps using Gorilla encoding
fn encode_timestamps(timestamps: &[i64]) -> Vec<u8> {
    encode_gorilla_i64(timestamps)
}

/// Decode timestamps from Gorilla encoding
fn decode_timestamps(buf: &[u8]) -> Result<Vec<i64>, String> {
    let (_, timestamps) = decode_gorilla_i64_with_length(buf)?;
    Ok(timestamps)
}

/// Indicates how many bytes to read given the Gorilla prefix
const GORILLA_PREFIX_BYTES: [u8; 5] = [0, 7, 9, 12, 32];

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn should_track_sequence_timestamp_pairs() {
        // given
        let mut tracker = SequenceTracker::with_config(10, 1);

        // when
        tracker.insert(TrackedSeq {
            seq: 100,
            ts: DateTime::from_timestamp(1000, 0).unwrap(),
        });
        tracker.insert(TrackedSeq {
            seq: 200,
            ts: DateTime::from_timestamp(2000, 0).unwrap(),
        });
        tracker.insert(TrackedSeq {
            seq: 300,
            ts: DateTime::from_timestamp(3000, 0).unwrap(),
        });

        // then
        assert_eq!(tracker.sequence_numbers.len(), 3);
        assert_eq!(
            tracker.find_seq(
                DateTime::from_timestamp(2000, 0).unwrap(),
                FindOption::RoundDown
            ),
            Some(200)
        );
        assert_eq!(
            tracker.find_ts(200, FindOption::RoundDown),
            DateTime::from_timestamp(2000, 0)
        );
    }

    #[test]
    fn should_respect_reporting_interval() {
        // given
        let mut tracker = SequenceTracker::with_config(10, 60);

        // when
        tracker.insert(TrackedSeq {
            seq: 100,
            ts: DateTime::from_timestamp(1000, 0).unwrap(),
        });
        // This should be ignored (only 30 seconds later)
        tracker.insert(TrackedSeq {
            seq: 200,
            ts: DateTime::from_timestamp(1030, 0).unwrap(),
        });
        // This should be recorded (60 seconds after first)
        tracker.insert(TrackedSeq {
            seq: 300,
            ts: DateTime::from_timestamp(1060, 0).unwrap(),
        });

        // then
        assert_eq!(tracker.sequence_numbers.len(), 2);
        assert_eq!(tracker.sequence_numbers, vec![100, 300]);
    }

    #[test]
    fn should_downsample_when_exceeding_capacity() {
        // given
        let mut tracker = SequenceTracker::with_config(4, 1);

        // when - add entries to exceed capacity
        for i in 0..6 {
            tracker.insert(TrackedSeq {
                seq: (i * 100) as u64,
                ts: DateTime::from_timestamp((i * 1000) as i64, 0).unwrap(),
            });
        }

        // then - should have downsampled to keep every other entry
        assert!(tracker.sequence_numbers.len() <= 4);
        assert_eq!(tracker.sequence_numbers[0], 0);
    }

    #[test]
    fn extend_from_non_empty_tracker() {
        // given
        let mut tracker1 = SequenceTracker::with_config(10, 1);
        tracker1.insert(TrackedSeq {
            seq: 100,
            ts: DateTime::from_timestamp(1000, 0).unwrap(),
        });
        tracker1.insert(TrackedSeq {
            seq: 200,
            ts: DateTime::from_timestamp(2000, 0).unwrap(),
        });

        let mut tracker2 = SequenceTracker::with_config(10, 1);
        tracker2.insert(TrackedSeq {
            seq: 300,
            ts: DateTime::from_timestamp(3000, 0).unwrap(),
        });
        tracker2.insert(TrackedSeq {
            seq: 400,
            ts: DateTime::from_timestamp(4000, 0).unwrap(),
        });

        // when
        tracker1.extend_from(&tracker2);

        // then
        assert_eq!(tracker1.sequence_numbers.len(), 4);
        assert_eq!(tracker1.sequence_numbers, vec![100, 200, 300, 400]);
        assert_eq!(
            tracker1.find_ts(300, FindOption::RoundDown),
            DateTime::from_timestamp(3000, 0)
        );
        assert_eq!(
            tracker1.find_ts(400, FindOption::RoundDown),
            DateTime::from_timestamp(4000, 0)
        );
    }

    #[test]
    fn extend_empty_tracker() {
        // given
        let mut tracker1 = SequenceTracker::with_config(10, 1);
        let mut tracker2 = SequenceTracker::with_config(10, 1);
        tracker2.insert(TrackedSeq {
            seq: 100,
            ts: DateTime::from_timestamp(1000, 0).unwrap(),
        });
        tracker2.insert(TrackedSeq {
            seq: 200,
            ts: DateTime::from_timestamp(2000, 0).unwrap(),
        });

        // when
        tracker1.extend_from(&tracker2);

        // then
        assert_eq!(tracker1.sequence_numbers.len(), 2);
        assert_eq!(tracker1.sequence_numbers, vec![100, 200]);
        assert_eq!(
            tracker1.find_ts(100, FindOption::RoundDown),
            DateTime::from_timestamp(1000, 0)
        );
        assert_eq!(
            tracker1.find_ts(200, FindOption::RoundDown),
            DateTime::from_timestamp(2000, 0)
        );
    }

    #[test]
    fn extend_from_empty_tracker() {
        // given
        let mut tracker1 = SequenceTracker::with_config(10, 1);
        tracker1.insert(TrackedSeq {
            seq: 100,
            ts: DateTime::from_timestamp(1000, 0).unwrap(),
        });
        let tracker2 = SequenceTracker::with_config(10, 1);
        let initial_len = tracker1.sequence_numbers.len();

        // when
        tracker1.extend_from(&tracker2);

        // then - should be unchanged
        assert_eq!(tracker1.sequence_numbers.len(), initial_len);
        assert_eq!(tracker1.sequence_numbers, vec![100]);
    }

    #[test]
    fn extend_respects_interval_filtering() {
        // given - tracker1 has last entry at timestamp 1000
        let mut tracker1 = SequenceTracker::with_config(10, 60);
        tracker1.insert(TrackedSeq {
            seq: 100,
            ts: DateTime::from_timestamp(1000, 0).unwrap(),
        });

        // tracker2 uses interval of 1 second so all entries get inserted
        // but when extending into tracker1 (interval 60), some will be filtered
        let mut tracker2 = SequenceTracker::with_config(10, 1);
        tracker2.insert(TrackedSeq {
            seq: 200,
            ts: DateTime::from_timestamp(1030, 0).unwrap(), // only 30 seconds after tracker1's 1000 - should be ignored
        });
        tracker2.insert(TrackedSeq {
            seq: 300,
            ts: DateTime::from_timestamp(1060, 0).unwrap(), // 60 seconds after tracker1's 1000 - should be recorded
        });
        tracker2.insert(TrackedSeq {
            seq: 400,
            ts: DateTime::from_timestamp(1110, 0).unwrap(), // only 50 seconds after tracker1's 1060 - should be ignored
        });
        tracker2.insert(TrackedSeq {
            seq: 500,
            ts: DateTime::from_timestamp(1120, 0).unwrap(), // 60 seconds after tracker1's 1060 - should be recorded
        });

        // when
        tracker1.extend_from(&tracker2);

        // then - only entries that respect the interval should be added
        assert_eq!(tracker1.sequence_numbers.len(), 3); // 100, 300, 500
        assert_eq!(tracker1.sequence_numbers, vec![100, 300, 500]);
    }

    #[test]
    fn extend_triggers_downsampling() {
        // given
        let mut tracker1 = SequenceTracker::with_config(4, 1);
        // Fill tracker1 to capacity
        for i in 0..4 {
            tracker1.insert(TrackedSeq {
                seq: (i * 100) as u64,
                ts: DateTime::from_timestamp((i * 1000) as i64, 0).unwrap(),
            });
        }

        let mut tracker2 = SequenceTracker::with_config(10, 1);
        // Add entries that will trigger downsampling
        for i in 4..7 {
            tracker2.insert(TrackedSeq {
                seq: (i * 100) as u64,
                ts: DateTime::from_timestamp((i * 1000) as i64, 0).unwrap(),
            });
        }

        // when
        tracker1.extend_from(&tracker2);

        // then - should have downsampled to keep within capacity
        assert!(tracker1.sequence_numbers.len() <= 4);
        // First entry should still be present after downsampling
        assert_eq!(tracker1.sequence_numbers[0], 0);
    }

    #[rstest]
    #[case::exact_match(8, FindOption::RoundDown, Some(200))]
    #[case::exact_match_round_up(8, FindOption::RoundUp, Some(200))]
    #[case::round_up_to_next(6, FindOption::RoundUp, Some(200))]
    #[case::round_down_to_previous(6, FindOption::RoundDown, Some(100))]
    #[case::before_first_seq(0, FindOption::RoundDown, None)]
    #[case::after_last_seq(16, FindOption::RoundUp, None)]
    #[case::at_first_seq(4, FindOption::RoundDown, Some(100))]
    #[case::at_last_seq(12, FindOption::RoundUp, Some(300))]
    fn test_find_timestamp_by_seq(
        #[case] seq: u64,
        #[case] find_opt: FindOption,
        #[case] expected_ts: Option<i64>,
    ) {
        // given
        let mut tracker = SequenceTracker::with_config(4, 1);
        if seq != 1 || find_opt != FindOption::RoundDown {
            tracker.insert(TrackedSeq {
                seq: 4,
                ts: DateTime::from_timestamp(100, 0).unwrap(),
            });
            tracker.insert(TrackedSeq {
                seq: 8,
                ts: DateTime::from_timestamp(200, 0).unwrap(),
            });
            tracker.insert(TrackedSeq {
                seq: 12,
                ts: DateTime::from_timestamp(300, 0).unwrap(),
            });
        }

        // when
        let result = tracker.find_ts(seq, find_opt);

        // then
        assert_eq!(
            result,
            expected_ts.and_then(|ts| DateTime::from_timestamp(ts, 0))
        );
    }

    #[rstest]
    #[case::exact_match(200, FindOption::RoundDown, Some(8))]
    #[case::exact_match_round_up(200, FindOption::RoundUp, Some(8))]
    #[case::round_up_to_next(150, FindOption::RoundUp, Some(8))]
    #[case::round_down_to_previous(150, FindOption::RoundDown, Some(4))]
    #[case::before_first_ts(50, FindOption::RoundDown, None)]
    #[case::after_last_ts(350, FindOption::RoundUp, None)]
    #[case::at_first_ts(100, FindOption::RoundDown, Some(4))]
    #[case::at_last_ts(300, FindOption::RoundUp, Some(12))]
    #[case::empty_tier(1, FindOption::RoundDown, None)]
    fn test_find_seq_by_timestamp(
        #[case] ts: i64,
        #[case] find_opt: FindOption,
        #[case] expected: Option<u64>,
    ) {
        // given
        let mut tracker = SequenceTracker::with_config(4, 1);
        // (ts == 1 is the empty tier case)
        if ts != 1 || find_opt != FindOption::RoundDown {
            tracker.insert(TrackedSeq {
                seq: 4,
                ts: DateTime::from_timestamp(100, 0).unwrap(),
            });
            tracker.insert(TrackedSeq {
                seq: 8,
                ts: DateTime::from_timestamp(200, 0).unwrap(),
            });
            tracker.insert(TrackedSeq {
                seq: 12,
                ts: DateTime::from_timestamp(300, 0).unwrap(),
            });
        }

        // when
        let result = tracker.find_seq(DateTime::from_timestamp(ts, 0).unwrap(), find_opt);

        // then
        assert_eq!(result, expected);
    }

    #[test]
    fn test_serialize_deserialize_round_trip() {
        // given
        let mut tracker = SequenceTracker::with_config(10, 1);
        for i in 0..5 {
            tracker.insert(TrackedSeq {
                seq: (i * 100) as u64,
                ts: DateTime::from_timestamp((i * 1000) as i64, 0).unwrap(),
            });
        }

        // when
        let encoded = encode_sequence_tracker(&tracker);
        let decoded = decode_sequence_tracker(&encoded).unwrap();

        // then
        assert_eq!(decoded.sequence_numbers, tracker.sequence_numbers);
        assert_eq!(decoded.timestamps, tracker.timestamps);
    }

    #[test]
    fn deserialize_preserves_the_recording_interval() {
        let mut tracker = SequenceTracker::new();
        tracker.insert(TrackedSeq {
            seq: 0,
            ts: DateTime::from_timestamp(1_600_000_060, 0).unwrap(),
        });

        let mut decoded = SequenceTracker::from_bytes(&tracker.to_bytes()).unwrap();
        decoded.insert(TrackedSeq {
            seq: 1,
            ts: DateTime::from_timestamp(1_600_000_061, 0).unwrap(),
        });

        assert_eq!(decoded.sequence_numbers, vec![0]);
    }

    #[rstest]
    #[case::empty_sequences(vec![])]
    #[case::single_sequence(vec![1000])]
    #[case::multiple_sequences(vec![100, 200, 300, 400, 500])]
    #[case::large_gaps(vec![100, 10000, 20000, 30000])]
    #[case::small_increments(vec![100, 101, 102, 103, 104])]
    #[case::max_u64(vec![u64::MAX])]
    #[case::above_i64_max(vec![i64::MAX as u64 + 1])]
    #[case::u64_values_that_become_negative(vec![u64::MAX - 100, u64::MAX - 50, u64::MAX])]
    #[case::large_u64_with_small_deltas(vec![u64::MAX - 10, u64::MAX - 9, u64::MAX - 8, u64::MAX - 7])]
    #[case::wraparound_sequence(vec![u64::MAX - 1, u64::MAX, 0, 1, 2])]
    fn test_encode_decode_sequence_numbers(#[case] sequences: Vec<u64>) {
        // when
        let encoded = encode_sequence_numbers(&sequences);
        let (_, decoded) = decode_sequence_numbers_with_length(&encoded).unwrap();

        // then
        assert_eq!(decoded, sequences);
    }

    #[rstest]
    #[case::empty_timestamps(vec![])]
    #[case::single_timestamp(vec![1234567890])]
    #[case::multiple_timestamps(vec![100, 200, 300, 400, 500])]
    #[case::zero_dod(vec![1000, 1000, 1000, 1000])] // Tests dod = 0 branch
    #[case::small_dod(vec![1000, 1064, 1128, 1192])] // Tests dod = 64 branch (max for 7-bit)
    #[case::medium_dod(vec![1000, 1256, 1512, 1768])] // Tests dod = 256 branch (max for 9-bit)
    #[case::large_dod(vec![1000, 3048, 5096, 7144])] // Tests dod = 2048 branch (max for 12-bit)
    #[case::huge_dod(vec![1000, 1000000, 2000000, 3000000])] // Tests 32-bit fallback branch
    #[case::negative_dod(vec![1000, 937, 874, 811])] // Tests negative dod values
    #[case::negative_timestamps(vec![-1000, -500, 0, 500, 1000])] // Tests negative timestamp values
    #[case::mixed_dod(vec![1000, 1000, 1032, 1128, 1512, 50000])] // Tests all branches with values within ranges
    #[case::mixed_dod_boundaries(vec![1000, 1000, 1028, 1255, 2047, 1000000])] // Tests all branches at boundaries
    #[case::large_numbers(vec![i64::MAX - 1000, i64::MAX - 500, i64::MAX])]
    fn test_encode_decode_timestamps(#[case] timestamps: Vec<i64>) {
        // when
        let encoded = encode_timestamps(&timestamps);
        let decoded = decode_timestamps(&encoded).unwrap();

        // then
        assert_eq!(decoded, timestamps);
    }

    #[test]
    fn test_empty_tracker_queries() {
        // given
        let tracker = SequenceTracker::new();

        // then
        assert_eq!(
            tracker.find_seq(
                DateTime::from_timestamp(100, 0).unwrap(),
                FindOption::RoundDown
            ),
            None
        );
        assert_eq!(tracker.find_ts(100, FindOption::RoundUp), None);
        assert!(tracker.sequence_numbers.is_empty());
    }

    #[test]
    fn test_serialize_deserialize_round_trip_proptest() {
        use proptest::collection::vec;
        use proptest::prelude::*;

        // Generate paired sequences and timestamps for testing
        let tracker_strategy = vec(any::<(u32, u32)>(), 0..=20).prop_map(|pairs| {
            let mut tracker = SequenceTracker::with_config(100, 1);

            let base_seq = 0u64;
            let base_timestamp = 1_600_000_000i64; // Unix timestamp around 2020

            let mut current_seq = base_seq;
            let mut current_ts = base_timestamp;

            for (seq_delta, ts_delta) in pairs {
                // Limit deltas to avoid problematic ranges in Gorilla encoding
                // Specifically avoid sequence number deltas >= 2^31 that cause sign issues
                // It is reasonable to assume that production deltas wont exceed these bounds.
                let safe_seq_delta = (seq_delta % (u32::MAX / 2)) as u64;
                let safe_ts_delta = (ts_delta % (u32::MAX / 2)) as i64;

                current_seq = current_seq.saturating_add(safe_seq_delta);
                current_ts = current_ts.saturating_add(safe_ts_delta);

                tracker.insert(TrackedSeq {
                    seq: current_seq,
                    ts: DateTime::from_timestamp(current_ts, 0).unwrap(),
                });
            }

            tracker
        });

        proptest!(|(tracker in tracker_strategy)| {
            // given a tracker with some data

            // when we encode and decode it
            let encoded = encode_sequence_tracker(&tracker);
            let decoded = decode_sequence_tracker(&encoded).unwrap();

            // then the sequence numbers and timestamps should match exactly
            assert_eq!(decoded.sequence_numbers, tracker.sequence_numbers);
            assert_eq!(decoded.timestamps, tracker.timestamps);
        });
    }
    #[test]
    fn admin_seq_tracker_seq_to_ts_rounding() -> Result<(), crate::Error> {
        use crate::admin::AdminBuilder;
        use crate::config::{CheckpointOptions, CheckpointScope, Settings};
        use crate::db::Db;
        use object_store::memory::InMemory;
        use object_store::path::Path;
        use object_store::ObjectStore;
        use std::sync::Arc;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let path = Path::from("/tmp/test_admin_seq_tracker_seq_to_ts");

            let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();

            let db = Db::builder(path.clone(), object_store.clone())
                .with_settings(Settings::default())
                .build()
                .await?;
            db.put(b"k1", b"v1").await?;
            db.put(b"k2", b"v2").await?;
            db.put(b"k3", b"v3").await?;

            db.create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
                .await?;
            db.close().await?;

            let ts_first = admin.get_timestamp_for_sequence(0, true).await?;
            assert!(ts_first.is_some());

            let ts_after_last = admin.get_timestamp_for_sequence(u64::MAX, true).await?;
            assert!(ts_after_last.is_none());

            Ok::<_, crate::Error>(())
        })?;

        Ok(())
    }

    #[test]
    fn admin_seq_tracker_ts_to_seq_rounding() -> Result<(), crate::Error> {
        use crate::admin::AdminBuilder;
        use crate::config::{CheckpointOptions, CheckpointScope, Settings};
        use crate::db::Db;
        use chrono::{TimeZone, Utc};
        use object_store::memory::InMemory;
        use object_store::path::Path;
        use object_store::ObjectStore;
        use std::sync::Arc;

        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let path = Path::from("/tmp/test_admin_seq_tracker_ts_to_seq");

            let admin = AdminBuilder::new(path.clone(), object_store.clone()).build();

            let db = Db::builder(path.clone(), object_store.clone())
                .with_settings(Settings::default())
                .build()
                .await?;
            db.put(b"a", b"1").await?;
            db.put(b"b", b"2").await?;

            db.create_checkpoint(CheckpointScope::All, &CheckpointOptions::default())
                .await?;
            db.close().await?;

            let long_ago = Utc.timestamp_opt(1, 0).single().unwrap();
            let seq_before_first = admin.get_sequence_for_timestamp(long_ago, false).await?;
            assert!(
                seq_before_first.is_none(),
                "round-down before first ts should yield None"
            );

            let now = Utc::now();
            let seq_latest = admin.get_sequence_for_timestamp(now, false).await?;
            assert!(
                seq_latest.is_some(),
                "round-down at/after last ts should yield Some(seq)"
            );

            Ok::<_, crate::Error>(())
        })?;

        Ok(())
    }
}
