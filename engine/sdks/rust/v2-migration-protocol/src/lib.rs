//! Wire types for the V2 public API's `/migration/v1/` routes.
//!
//! Request and response bodies are raw BARE: the `/v1` path pins the schema version for both peers.
//! The parked record is persisted, so it alone carries a vbare version header (see [`versioned`]).

use anyhow::{Context, Result, bail};

pub mod generated;
pub mod versioned;

pub use generated::v1::*;

/// Content type of the streamed export response.
pub const EXPORT_CONTENT_TYPE: &str = "application/x-rivet-v2-migration";

/// Content type of every other BARE request and response body.
pub const BARE_CONTENT_TYPE: &str = "application/x-bare";

/// Largest frame a reader accepts. Batches are capped well below this.
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

/// Encodes one export frame with its little-endian `u32` length prefix.
pub fn encode_frame(frame: &Frame) -> Result<Vec<u8>> {
	let body = serde_bare::to_vec(frame).context("encode export frame")?;
	let len = u32::try_from(body.len()).context("export frame too large")?;

	let mut out = Vec::with_capacity(4 + body.len());
	out.extend_from_slice(&len.to_le_bytes());
	out.extend_from_slice(&body);
	Ok(out)
}

/// Splits one length-prefixed frame off the front of `buf`.
///
/// Returns `None` until the whole frame is buffered. The returned bytes are the frame's BARE body,
/// which is also what the export digest chains over.
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
	serde_bare::from_slice(body).context("decode export frame")
}

pub fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
	serde_bare::to_vec(value).context("encode migration message")
}

pub fn decode<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T> {
	serde_bare::from_slice(body).context("decode migration message")
}
