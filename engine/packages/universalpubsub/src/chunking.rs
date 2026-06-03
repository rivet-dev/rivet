use std::borrow::Cow;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use rivet_ups_protocol::versioned::UpsMessage;
use rivet_ups_protocol::{MessageBody, MessageChunk, MessageStart, PROTOCOL_VERSION};
use scc::HashMap;
use uuid::Uuid;
use vbare::OwnedVersionedData;

pub const CHUNK_BUFFER_MAX_AGE: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub struct ChunkBuffer {
	pub message_id: Uuid,
	pub timestamp: i64,
	pub received_chunks: u32,
	pub last_chunk_ts: Instant,
	pub buffer: Vec<u8>,
	pub chunk_count: u32,
	pub reply_subject: Option<String>,
	pub request_deadline_at: Option<i64>,
}

#[derive(Debug)]
pub struct DecodedMessage {
	pub message_id: Uuid,
	pub timestamp: i64,
	pub payload: Vec<u8>,
	pub reply_subject: Option<String>,
	pub request_deadline_at: Option<i64>,
}

/// Result of the sync fast-path chunk decode.
pub enum FastPath {
	/// Single-chunk message fully decoded inline; the hashmap was not touched.
	Decoded(DecodedMessage),
	/// Multi-chunk start or continuation; caller must invoke `process_chunk_async`
	/// with the already-decoded `UpsMessage` to avoid a second BARE deserialize.
	Multi(rivet_ups_protocol::UpsMessage),
}

pub struct ChunkTracker {
	chunks_in_process: HashMap<Uuid, ChunkBuffer>,
}

impl ChunkTracker {
	pub fn new() -> Self {
		Self {
			chunks_in_process: HashMap::new(),
		}
	}

	/// Sync fast path. Single-chunk messages are decoded inline without touching
	/// `chunks_in_process`; multi-chunk start/continuation return the parsed
	/// `UpsMessage` so the caller can hand it to `process_chunk_async` without
	/// a second BARE deserialize.
	pub fn try_process_chunk_fast(&self, raw_message: &[u8]) -> Result<FastPath> {
		let message = UpsMessage::deserialize_with_embedded_version(raw_message)?;

		match message.body {
			MessageBody::MessageStart(msg) if msg.chunk_count == 1 => {
				let message_id = Uuid::from_bytes(msg.message_id);
				Ok(FastPath::Decoded(DecodedMessage {
					message_id,
					timestamp: msg.timestamp,
					payload: msg.payload,
					reply_subject: msg.reply_subject,
					request_deadline_at: msg.request_deadline_at,
				}))
			}
			body => Ok(FastPath::Multi(rivet_ups_protocol::UpsMessage { body })),
		}
	}

	pub async fn process_chunk_async(
		&self,
		message: rivet_ups_protocol::UpsMessage,
	) -> Result<Option<DecodedMessage>> {
		match message.body {
			MessageBody::MessageStart(msg) => {
				let message_id = Uuid::from_bytes(msg.message_id);
				// If only one chunk, return immediately
				if msg.chunk_count == 1 {
					return Ok(Some(DecodedMessage {
						message_id,
						timestamp: msg.timestamp,
						payload: msg.payload,
						reply_subject: msg.reply_subject,
						request_deadline_at: msg.request_deadline_at,
					}));
				}

				// Start of a multi-chunk message
				let buffer = ChunkBuffer {
					message_id,
					timestamp: msg.timestamp,
					received_chunks: 1,
					last_chunk_ts: Instant::now(),
					buffer: msg.payload,
					chunk_count: msg.chunk_count,
					reply_subject: msg.reply_subject,
					request_deadline_at: msg.request_deadline_at,
				};
				// Overwrite any prior in-flight buffer for the same message_id,
				// matching the previous std HashMap::insert semantics.
				let entry = self.chunks_in_process.entry_async(message_id).await;
				match entry {
					scc::hash_map::Entry::Occupied(mut o) => {
						o.insert(buffer);
					}
					scc::hash_map::Entry::Vacant(v) => {
						v.insert_entry(buffer);
					}
				}
				Ok(None)
			}
			MessageBody::MessageChunk(msg) => {
				let message_id = Uuid::from_bytes(msg.message_id);

				let entry = self.chunks_in_process.entry_async(message_id).await;
				let mut occupied = match entry {
					scc::hash_map::Entry::Occupied(o) => o,
					scc::hash_map::Entry::Vacant(_) => {
						bail!(
							"received chunk {} for message {} but no matching buffer found",
							msg.chunk_index,
							message_id
						);
					}
				};

				{
					let buffer = occupied.get_mut();

					// Validate chunk order
					if buffer.received_chunks != msg.chunk_index {
						bail!(
							"received chunk {} but expected chunk {} for message {}",
							msg.chunk_index,
							buffer.received_chunks,
							message_id
						);
					}

					// Update buffer
					buffer.buffer.extend_from_slice(&msg.payload);
					buffer.received_chunks += 1;
					buffer.last_chunk_ts = Instant::now();
				}

				let is_complete = {
					let buffer = occupied.get();
					buffer.received_chunks == buffer.chunk_count
				};

				if is_complete {
					let (_, completed_buffer) = occupied.remove_entry();
					Ok(Some(DecodedMessage {
						message_id,
						timestamp: completed_buffer.timestamp,
						payload: completed_buffer.buffer,
						reply_subject: completed_buffer.reply_subject,
						request_deadline_at: completed_buffer.request_deadline_at,
					}))
				} else {
					Ok(None)
				}
			}
		}
	}

	pub async fn gc(&self) {
		let now = Instant::now();
		let size_before = self.chunks_in_process.len();
		self.chunks_in_process
			.retain_async(|_, buffer| {
				now.duration_since(buffer.last_chunk_ts) < CHUNK_BUFFER_MAX_AGE
			})
			.await;
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
	message_id: Uuid,
	reply_subject: Option<&str>,
	request_deadline_at: Option<i64>,
) -> Result<Vec<Vec<u8>>> {
	let message_id_buf = *message_id.as_bytes();

	// Calculate overhead for MessageStart (first chunk)
	let start_message = MessageStart {
		message_id: message_id_buf,
		chunk_count: 1,
		timestamp: rivet_util::timestamp::now(),
		reply_subject: reply_subject.map(|s| s.to_string()),
		request_deadline_at,
		payload: vec![],
	};
	let start_ups_message = rivet_ups_protocol::UpsMessage {
		body: MessageBody::MessageStart(start_message),
	};
	let start_overhead = UpsMessage::wrap_latest(start_ups_message)
		.serialize_with_embedded_version(*PROTOCOL_VERSION)?
		.len();

	// Calculate overhead for MessageChunk (subsequent chunks)
	let chunk_message = MessageChunk {
		message_id: message_id_buf,
		chunk_index: 0,
		timestamp: rivet_util::timestamp::now(),
		payload: vec![],
	};
	let chunk_ups_message = rivet_ups_protocol::UpsMessage {
		body: MessageBody::MessageChunk(chunk_message),
	};
	let chunk_overhead = UpsMessage::wrap_latest(chunk_ups_message)
		.serialize_with_embedded_version(*PROTOCOL_VERSION)?
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
	message_id: Uuid,
	reply_subject: Option<Cow<str>>,
	request_deadline_at: Option<i64>,
) -> Result<Vec<u8>> {
	let message_id_buf = *message_id.as_bytes();

	let body = if chunk_idx == 0 {
		// First chunk - MessageStart
		MessageBody::MessageStart(MessageStart {
			message_id: message_id_buf,
			chunk_count,
			timestamp: rivet_util::timestamp::now(),
			reply_subject: reply_subject.map(|x| x.into_owned()),
			request_deadline_at,
			payload,
		})
	} else {
		// Subsequent chunks - MessageChunk
		MessageBody::MessageChunk(MessageChunk {
			message_id: message_id_buf,
			chunk_index: chunk_idx,
			timestamp: rivet_util::timestamp::now(),
			payload,
		})
	};

	let ups_message = rivet_ups_protocol::UpsMessage { body };
	UpsMessage::wrap_latest(ups_message).serialize_with_embedded_version(*PROTOCOL_VERSION)
}
