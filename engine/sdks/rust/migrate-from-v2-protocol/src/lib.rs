//! Wire types for V2's public `/migration/v1/` routes, which the V3 migrator calls.
//!
//! This is a copy of V2's schema (`engine/sdks/schemas/v2-migration-protocol/v1.bare` on V2).
//! Bodies are raw BARE: the `/v1` path pins the schema version for both peers, so no version
//! header is written. Keep the schema byte-identical to V2's.

use anyhow::{Context, Result, bail};
use vbare::{Bytes, Encode, Message};

pub mod generated;

pub use generated::v1::*;

/// Content type of the streamed export response.
pub const EXPORT_CONTENT_TYPE: &str = "application/x-rivet-v2-migration";

/// Content type of every other BARE request and response body.
pub const BARE_CONTENT_TYPE: &str = "application/x-bare";

/// Largest frame a reader accepts. Batches are capped well below this.
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Encodes one export frame with its little-endian `u32` length prefix.
pub fn encode_frame(frame: &Frame) -> Result<Vec<u8>> {
	let body = frame.encode_to_vec();
	let len = u32::try_from(body.len()).context("export frame too large")?;

	let mut out = Vec::with_capacity(4 + body.len());
	out.extend_from_slice(&len.to_le_bytes());
	out.extend_from_slice(&body);
	Ok(out)
}

/// Splits one length-prefixed frame off the front of `buf`, returning the frame's BARE body and
/// the bytes consumed. Returns `None` until the whole frame is buffered.
pub fn split_frame(buf: &[u8]) -> Result<Option<(&[u8], usize)>> {
	if buf.len() < 4 {
		return Ok(None);
	}

	let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
	if len > MAX_FRAME_BYTES {
		bail!("export frame of {len} bytes exceeds the {MAX_FRAME_BYTES} byte limit");
	}
	if buf.len() < 4 + len {
		return Ok(None);
	}

	Ok(Some((&buf[4..4 + len], 4 + len)))
}

pub fn decode_frame(body: &[u8]) -> Result<Frame> {
	decode(body)
}

pub fn encode<T: Encode>(value: &T) -> Vec<u8> {
	value.encode_to_vec()
}

pub fn decode<T: Message>(body: &[u8]) -> Result<T> {
	T::decode(Bytes::copy_from_slice(body)).context("decode migration message")
}

/// Chains one data frame body into the export digest: `xxh3_128(previous || frame)`. V2 computes
/// the same chain over every `KvBatch` and `PageBatch` frame, starting from 16 zero bytes.
pub fn chain_digest(previous: &[u8], frame_body: &[u8]) -> [u8; 16] {
	let mut input = Vec::with_capacity(previous.len() + frame_body.len());
	input.extend_from_slice(previous);
	input.extend_from_slice(frame_body);
	xxh3_128(&input)
}

fn xxh3_128(input: &[u8]) -> [u8; 16] {
	xxhash_rust::xxh3::xxh3_128(input).to_be_bytes()
}
