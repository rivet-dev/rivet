#![allow(dead_code)]

// This module mirrors the old TypeScript actor key layout. Some helpers are
// reserved for storage owned by foreign-runtime layers.
//
// Paths below are actor-local KV keys. Byte arrays are shown as decimal path
// segments because they are stored as opaque byte keys.

use anyhow::Result;
use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

// Persisted actor state lives at [1].
pub const PERSIST_DATA_KEY: &[u8] = &[1];
// Connection records live under [2, ...conn_id_bytes].
pub const CONN_PREFIX: [u8; 1] = [2];
// The inspector auth token lives at [3].
pub const INSPECTOR_TOKEN_KEY: [u8; 1] = [3];
// User KV entries live under [4, ...user_key].
pub const KV_PREFIX: [u8; 1] = [4];
// Queue storage lives under [5, ...].
pub const QUEUE_PREFIX: [u8; 1] = [5];
// Workflow storage lives under [6, ...].
pub const WORKFLOW_PREFIX: [u8; 1] = [6];
// Trace storage lives under [7, ...].
pub const TRACES_PREFIX: [u8; 1] = [7];

// This exact key does not collide with workflow storage because workflows use
// the versioned [6, 1] prefix.
pub const LAST_PUSHED_ALARM_KEY: &[u8] = &[6];

// Queue version 1 storage lives under [5, 1, ...].
pub const QUEUE_STORAGE_VERSION: u8 = 1;
// Workflow version 1 storage lives under [6, 1, ...].
pub const WORKFLOW_STORAGE_VERSION: u8 = 1;
// Trace version 1 storage lives under [7, 1, ...].
pub const TRACES_STORAGE_VERSION: u8 = 1;

// Queue metadata lives at [5, 1, 1].
const QUEUE_NAMESPACE_METADATA: u8 = 1;
// Queue messages live under [5, 1, 2, ...message_id_be_u64].
const QUEUE_NAMESPACE_MESSAGES: u8 = 2;
const QUEUE_ID_BYTES: usize = 8;

// Prefix for all queue v1 storage: [5, 1].
pub const QUEUE_STORAGE_PREFIX: [u8; 2] = [QUEUE_PREFIX[0], QUEUE_STORAGE_VERSION];
// The single queue metadata record: [5, 1, 1].
pub const QUEUE_METADATA_KEY: [u8; 3] = [
	QUEUE_PREFIX[0],
	QUEUE_STORAGE_VERSION,
	QUEUE_NAMESPACE_METADATA,
];
// Prefix for queue messages: [5, 1, 2, ...message_id_be_u64].
pub const QUEUE_MESSAGES_PREFIX: [u8; 3] = [
	QUEUE_PREFIX[0],
	QUEUE_STORAGE_VERSION,
	QUEUE_NAMESPACE_MESSAGES,
];
// Prefix for workflow v1 storage: [6, 1, ...workflow_key].
pub const WORKFLOW_STORAGE_PREFIX: [u8; 2] = [WORKFLOW_PREFIX[0], WORKFLOW_STORAGE_VERSION];
// Prefix for trace v1 storage: [7, 1, ...trace_key].
pub const TRACES_STORAGE_PREFIX: [u8; 2] = [TRACES_PREFIX[0], TRACES_STORAGE_VERSION];

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"queue",
	"invalid_message_key",
	"Queue message key is invalid",
	"Queue message key is invalid: {reason}"
)]
struct QueueInvalidMessageKey {
	reason: String,
}

pub fn make_prefixed_key(key: &[u8]) -> Vec<u8> {
	concat_prefix(&KV_PREFIX, key)
}

pub fn remove_prefix_from_key(prefixed_key: &[u8]) -> &[u8] {
	&prefixed_key[KV_PREFIX.len()..]
}

pub fn make_workflow_key(key: &[u8]) -> Vec<u8> {
	concat_prefix(&WORKFLOW_STORAGE_PREFIX, key)
}

pub fn make_traces_key(key: &[u8]) -> Vec<u8> {
	concat_prefix(&TRACES_STORAGE_PREFIX, key)
}

pub fn make_connection_key(conn_id: &str) -> Vec<u8> {
	concat_prefix(&CONN_PREFIX, conn_id.as_bytes())
}

pub fn make_queue_message_key(id: u64) -> Vec<u8> {
	let mut key = Vec::with_capacity(QUEUE_MESSAGES_PREFIX.len() + QUEUE_ID_BYTES);
	key.extend_from_slice(&QUEUE_MESSAGES_PREFIX);
	key.extend_from_slice(&id.to_be_bytes());
	key
}

pub fn decode_queue_message_key(key: &[u8]) -> Result<u64> {
	if key.len() != QUEUE_MESSAGES_PREFIX.len() + QUEUE_ID_BYTES {
		return Err(invalid_queue_key("invalid length"));
	}
	if !key.starts_with(&QUEUE_MESSAGES_PREFIX) {
		return Err(invalid_queue_key("invalid prefix"));
	}

	let bytes: [u8; QUEUE_ID_BYTES] = key[QUEUE_MESSAGES_PREFIX.len()..]
		.try_into()
		.map_err(|_| invalid_queue_key("invalid id bytes"))?;
	Ok(u64::from_be_bytes(bytes))
}

fn concat_prefix(prefix: &[u8], suffix: &[u8]) -> Vec<u8> {
	let mut key = Vec::with_capacity(prefix.len() + suffix.len());
	key.extend_from_slice(prefix);
	key.extend_from_slice(suffix);
	key
}

fn invalid_queue_key(reason: &str) -> anyhow::Error {
	QueueInvalidMessageKey {
		reason: reason.to_owned(),
	}
	.build()
}
