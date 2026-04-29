use anyhow::{Context, Result, bail};
use gas::prelude::Id;
use serde::{Deserialize, Serialize};
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use super::subjects::SqliteCompactSubject;

pub type Ups = universalpubsub::PubSub;

pub const SQLITE_COMPACT_PAYLOAD_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqliteCompactPayload {
	pub actor_id: String,
	pub namespace_id: Option<Id>,
	pub actor_name: Option<String>,
	pub commit_bytes_since_rollup: u64,
	pub read_bytes_since_rollup: u64,
}

enum VersionedSqliteCompactPayload {
	V1(SqliteCompactPayload),
}

impl OwnedVersionedData for VersionedSqliteCompactPayload {
	type Latest = SqliteCompactPayload;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite compact payload version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_compact_payload(payload: SqliteCompactPayload) -> Result<Vec<u8>> {
	VersionedSqliteCompactPayload::wrap_latest(payload)
		.serialize_with_embedded_version(SQLITE_COMPACT_PAYLOAD_VERSION)
		.context("encode sqlite compact payload")
}

pub fn decode_compact_payload(payload: &[u8]) -> Result<SqliteCompactPayload> {
	VersionedSqliteCompactPayload::deserialize_with_embedded_version(payload)
		.context("decode sqlite compact payload")
}

pub fn publish_compact_trigger(ups: &Ups, actor_id: &str) {
	publish_compact_payload(
		ups,
		SqliteCompactPayload {
			actor_id: actor_id.to_string(),
			namespace_id: None,
			actor_name: None,
			commit_bytes_since_rollup: 0,
			read_bytes_since_rollup: 0,
		},
	);
}

pub fn publish_compact_payload(ups: &Ups, payload: SqliteCompactPayload) {
	let ups = ups.clone();
	let actor_id = payload.actor_id.clone();

	tokio::spawn(async move {
		let payload = match encode_compact_payload(payload) {
			Ok(payload) => payload,
			Err(err) => {
				tracing::error!(?err, actor_id = %actor_id, "failed to encode sqlite compact trigger");
				return;
			}
		};

		if let Err(err) = ups
			.publish(SqliteCompactSubject, &payload, PublishOpts::one())
			.await
		{
			tracing::warn!(?err, actor_id = %actor_id, "failed to publish sqlite compact trigger");
		}
	});
}
