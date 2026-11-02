//! Payload codecs and the payload digest for `PayloadWrite`, over `lz4_flex` block compression
//! and `xxh3`.
//!
//! A writer encodes a payload with its configured codec and records the codec in the entry.
//! Every reader decodes by the entry's own codec, so changing the configured codec never makes
//! an older entry unreadable. Shards replicate the encoded bytes without decoding them.

use anyhow::{Context, Result, ensure};
use rivet_data::generated::depot_log_shard_v1::{PayloadCodec, PayloadWrite};
use std::borrow::Cow;

/// The retry identity of a payload's decoded bytes, so it is the same under every codec.
///
/// The digest is one component of a write's retry identity beside its `request_id` and
/// `client_seq`: a retry whose bytes differ from the admitted write is refused rather than
/// deduplicated. Callers choose payload bytes but never the identity they are paired with:
/// the engine assigns `request_id` and resends the admitted bytes under it, so a correct writer
/// never presents two payloads under one identity. The digest is therefore a check against a
/// writer bug, not what makes retries correct, and the bytes on disk are protected by the batch
/// checksums. 128-bit xxh3 catches accidental mismatches at a fraction of a cryptographic hash's
/// cost, which matters because the digest is taken on every write; it does not resist bytes
/// chosen to collide.
pub type PayloadDigest = [u8; 16];

/// Digests a payload's decoded bytes. See [`PayloadDigest`].
pub fn payload_digest(decoded: &[u8]) -> PayloadDigest {
	xxhash_rust::xxh3::xxh3_128(decoded).to_be_bytes()
}

/// The largest decoded payload a reader accepts. A log entry is bounded far below this by the
/// append limit, so a larger length prefix is corrupt input, never a real payload.
pub const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;

/// Encodes `raw` with `codec`. Falls back to `None` when the encoding is not smaller, so a
/// payload never grows.
pub fn encode(codec: PayloadCodec, raw: &[u8]) -> (PayloadCodec, Option<Vec<u8>>) {
	match codec {
		PayloadCodec::None => (PayloadCodec::None, None),
		PayloadCodec::Lz4 => {
			let encoded = lz4_flex::block::compress_prepend_size(raw);
			match encoded.len() < raw.len() {
				true => (PayloadCodec::Lz4, Some(encoded)),
				false => (PayloadCodec::None, None),
			}
		}
	}
}

/// Encodes a write's payload in place with `codec`, keeping its digest of the decoded bytes.
pub fn encode_write(codec: PayloadCodec, write: &mut PayloadWrite) {
	if write.codec != PayloadCodec::None {
		return;
	}
	if let (codec, Some(encoded)) = encode(codec, &write.payload) {
		write.codec = codec;
		write.payload = encoded.into();
	}
}

/// The decoded bytes of a write's payload. Fails on a corrupt or oversized encoding.
pub fn decode(write: &PayloadWrite) -> Result<Cow<'_, [u8]>> {
	decode_bytes(write.codec, &write.payload)
}

/// The decoded bytes of a write's payload as a shared buffer. An unencoded payload shares the
/// entry's buffer, so decoding it copies nothing.
pub fn decode_shared(write: &PayloadWrite) -> Result<vbare::Bytes> {
	match write.codec {
		PayloadCodec::None => Ok(write.payload.clone()),
		PayloadCodec::Lz4 => Ok(decode(write)?.into_owned().into()),
	}
}

/// Decodes `payload` encoded with `codec`.
pub fn decode_bytes(codec: PayloadCodec, payload: &[u8]) -> Result<Cow<'_, [u8]>> {
	match codec {
		PayloadCodec::None => Ok(Cow::Borrowed(payload)),
		PayloadCodec::Lz4 => {
			let (size, block) = lz4_flex::block::uncompressed_size(payload)
				.context("lz4 payload has no length prefix")?;
			ensure!(
				size <= MAX_DECODED_BYTES,
				"lz4 payload claims {size} decoded bytes"
			);
			let mut decoded = vec![0; size];
			let written = lz4_flex::block::decompress_into(block, &mut decoded)
				.context("corrupt lz4 payload")?;
			ensure!(written == size, "lz4 payload decoded to the wrong length");
			Ok(Cow::Owned(decoded))
		}
	}
}

/// The codec's name, as a metric label and in configuration.
pub fn label(codec: PayloadCodec) -> &'static str {
	match codec {
		PayloadCodec::None => "none",
		PayloadCodec::Lz4 => "lz4",
	}
}
