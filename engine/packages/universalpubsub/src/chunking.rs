use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::*;
use rivet_ups_protocol::versioned::UpsMessage;
use rivet_ups_protocol::{MessageBody, MessageChunk, MessageStart, PROTOCOL_VERSION};
use vbare::OwnedVersionedData;

pub const CHUNK_BUFFER_MAX_AGE: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub struct ChunkBuffer {
	pub message_id: [u8; 16],
	pub received_chunks: u32,
	pub last_chunk_ts: Instant,
	pub buffer: Vec<u8>,
	pub chunk_count: u32,
	pub reply_subject: Option<String>,
}

pub struct ChunkTracker {
	chunks_in_process: HashMap<[u8; 16], ChunkBuffer>,
}

impl ChunkTracker {
	pub fn new() -> Self {
		Self {
			chunks_in_process: HashMap::new(),
		}
	}

	pub fn process_chunk(
		&mut self,
		raw_message: &[u8],
	) -> Result<Option<(Vec<u8>, Option<String>)>> {
		let message = UpsMessage::deserialize_with_embedded_version(raw_message)?;

		match message.body {
			MessageBody::MessageStart(msg) => {
				// If only one chunk, return immediately
				if msg.chunk_count == 1 {
					return Ok(Some((msg.payload, msg.reply_subject)));
				}

				// Start of a multi-chunk message
				let buffer = ChunkBuffer {
					message_id: msg.message_id,
					received_chunks: 1,
					last_chunk_ts: Instant::now(),
					buffer: msg.payload,
					chunk_count: msg.chunk_count,
					reply_subject: msg.reply_subject,
				};
				self.chunks_in_process.insert(msg.message_id, buffer);
				Ok(None)
			}
			MessageBody::MessageChunk(msg) => {
				// Find the matching buffer using message_id
				let buffer = self.chunks_in_process.get_mut(&msg.message_id);

				let Some(buffer) = buffer else {
					bail!(
						"received chunk {} for message {:?} but no matching buffer found",
						msg.chunk_index,
						msg.message_id
					);
				};

				// Validate chunk order
				if buffer.received_chunks != msg.chunk_index {
					bail!(
						"received chunk {} but expected chunk {} for message {:?}",
						msg.chunk_index,
						buffer.received_chunks,
						msg.message_id
					);
				}

				// Update buffer
				buffer.buffer.extend_from_slice(&msg.payload);
				buffer.received_chunks += 1;
				buffer.last_chunk_ts = Instant::now();
				let is_complete = buffer.received_chunks == buffer.chunk_count;

				if is_complete {
					let completed_buffer = self.chunks_in_process.remove(&msg.message_id).unwrap();
					Ok(Some((
						completed_buffer.buffer,
						completed_buffer.reply_subject,
					)))
				} else {
					Ok(None)
				}
			}
		}
	}

	pub fn gc(&mut self) {
		let now = Instant::now();
		let size_before = self.chunks_in_process.len();
		self.chunks_in_process
			.retain(|_, buffer| now.duration_since(buffer.last_chunk_ts) < CHUNK_BUFFER_MAX_AGE);
		let size_after = self.chunks_in_process.len();

		tracing::trace!(
			?size_before,
			?size_after,
			"performed chunk buffer garbage collection"
		);
	}
}

/// Returns the number of bytes needed to encode `n` as a BARE unsigned integer (LEB128).
fn bare_uint_len(n: usize) -> usize {
	let mut len = 1;
	let mut v = n >> 7;
	while v > 0 {
		len += 1;
		v >>= 7;
	}
	len
}

/// Splits a payload into chunks that fit within message size limits.
///
/// This function handles chunking by accounting for different overhead
/// between the first chunk (MessageStart) and subsequent chunks (MessageChunk).
///
/// The first chunk carries additional metadata like the reply_subject and chunk_count,
/// which means it has more protocol overhead and less room for payload data.
/// Subsequent chunks only carry a chunk_index, allowing them to fit more payload.
///
/// This optimization ensures:
/// - Reply subject is only transmitted once (in MessageStart)
/// - Maximum payload utilization in each chunk
/// - Efficient bandwidth usage for multi-chunk messages
///
/// # Returns
/// A vector of payload chunks, where each chunk is sized to fit within the message limit
/// after accounting for protocol overhead.
pub fn split_payload_into_chunks(
	payload: &[u8],
	max_message_size: usize,
	message_id: [u8; 16],
	reply_subject: Option<&str>,
) -> Result<Vec<Vec<u8>>> {
	// Calculate overhead for MessageStart (first chunk)
	let start_message = MessageStart {
		message_id,
		chunk_count: 1,
		reply_subject: reply_subject.map(|s| s.to_string()),
		payload: vec![],
	};
	let start_ups_message = rivet_ups_protocol::UpsMessage {
		body: MessageBody::MessageStart(start_message),
	};
	let start_overhead = UpsMessage::wrap_latest(start_ups_message)
		.serialize_with_embedded_version(PROTOCOL_VERSION)?
		.len();

	// Calculate overhead for MessageChunk (subsequent chunks)
	let chunk_message = MessageChunk {
		message_id,
		chunk_index: 0,
		payload: vec![],
	};
	let chunk_ups_message = rivet_ups_protocol::UpsMessage {
		body: MessageBody::MessageChunk(chunk_message),
	};
	let chunk_overhead = UpsMessage::wrap_latest(chunk_ups_message)
		.serialize_with_embedded_version(PROTOCOL_VERSION)?
		.len();

	// Calculate max payload sizes, correcting for the variable-length encoding of the
	// data length prefix. The overhead above was computed with an empty payload
	// (uint(0) = 1 byte). For payloads >= 128 bytes the length prefix grows (LEB128
	// encoding), so we subtract those extra bytes to ensure every encoded chunk fits
	// within max_message_size.
	let first_chunk_max_payload = {
		let raw = max_message_size.saturating_sub(start_overhead);
		let extra = bare_uint_len(raw).saturating_sub(1);
		raw.saturating_sub(extra)
	};
	let other_chunk_max_payload = {
		let raw = max_message_size.saturating_sub(chunk_overhead);
		let extra = bare_uint_len(raw).saturating_sub(1);
		raw.saturating_sub(extra)
	};

	if first_chunk_max_payload == 0 || other_chunk_max_payload == 0 {
		bail!("message overhead exceeds max message size");
	}

	// Calculate how many chunks we need
	if payload.len() <= first_chunk_max_payload {
		// Single chunk - all data fits in first message
		return Ok(vec![payload.to_vec()]);
	}

	// Multi-chunk: first chunk + remaining chunks
	let remaining_after_first = payload.len() - first_chunk_max_payload;
	let additional_chunks =
		(remaining_after_first + other_chunk_max_payload - 1) / other_chunk_max_payload;

	let mut chunks = Vec::new();

	// First chunk (smaller due to reply_subject overhead)
	chunks.push(payload[..first_chunk_max_payload].to_vec());

	// Subsequent chunks
	let mut offset = first_chunk_max_payload;
	for _ in 0..additional_chunks {
		let end = std::cmp::min(offset + other_chunk_max_payload, payload.len());
		chunks.push(payload[offset..end].to_vec());
		offset = end;
	}

	Ok(chunks)
}

/// Encodes a chunk to the resulting BARE message.
pub fn encode_chunk(
	payload: Vec<u8>,
	chunk_idx: u32,
	chunk_count: u32,
	message_id: [u8; 16],
	reply_subject: Option<String>,
) -> Result<Vec<u8>> {
	let body = if chunk_idx == 0 {
		// First chunk - MessageStart
		MessageBody::MessageStart(MessageStart {
			message_id,
			chunk_count,
			reply_subject,
			payload,
		})
	} else {
		// Subsequent chunks - MessageChunk
		MessageBody::MessageChunk(MessageChunk {
			message_id,
			chunk_index: chunk_idx,
			payload,
		})
	};

	let ups_message = rivet_ups_protocol::UpsMessage { body };
	UpsMessage::wrap_latest(ups_message).serialize_with_embedded_version(PROTOCOL_VERSION)
}
