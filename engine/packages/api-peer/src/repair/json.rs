//! JSON forms of Depot wire values at the repair HTTP boundary. Requests name volumes, rows and
//! positions with hex bytes and UUID strings; responses are the wire types themselves, with
//! every byte string and fixed byte array rendered as lowercase hex.

use super::Refusal;
use rivet_data::generated::{shared_depot_fs_read_v1 as read, shared_depot_types_v1 as wire};
use serde::Deserialize;
use serde_json::Value;

/// A wire value as JSON with its bytes in hex and its field names in camel case, as the
/// schemas spell them.
///
/// The wire types serialize byte strings and fixed byte arrays as arrays of numbers. Every
/// non-empty array whose elements are all integers from 0 to 255 is such a byte value in the
/// responses these routes return: no response type carries a list of small integers. An empty
/// byte string stays an empty array.
pub(crate) fn wire<T: serde::Serialize>(value: &T) -> Value {
	match serde_json::to_value(value) {
		Ok(value) => hexify(value),
		Err(error) => Value::String(format!("unserializable: {error}")),
	}
}

fn hexify(value: Value) -> Value {
	match value {
		Value::Array(items) => {
			let bytes = items
				.iter()
				.map(|item| {
					item.as_u64()
						.filter(|byte| *byte <= 255)
						.map(|byte| byte as u8)
				})
				.collect::<Option<Vec<u8>>>();
			match bytes {
				Some(bytes) if !bytes.is_empty() => Value::String(hex::encode(bytes)),
				Some(_) | None => Value::Array(items.into_iter().map(hexify).collect()),
			}
		}
		Value::Object(map) => Value::Object(
			map.into_iter()
				.map(|(key, value)| (camel_case(&key), hexify(value)))
				.collect(),
		),
		other => other,
	}
}

/// `volume_key` as `volumeKey`. Names without an underscore, such as enum variants, are kept.
fn camel_case(name: &str) -> String {
	let mut out = String::with_capacity(name.len());
	let mut upper = false;
	for character in name.chars() {
		if character == '_' {
			upper = true;
		} else if upper {
			out.extend(character.to_uppercase());
			upper = false;
		} else {
			out.push(character);
		}
	}
	out
}

pub(crate) fn unhex(value: &str, what: &str) -> Result<Vec<u8>, Refusal> {
	hex::decode(value).map_err(|_| Refusal::invalid(format!("{what} is not hex")))
}

pub(crate) fn uuid(value: &str, what: &str) -> Result<[u8; 16], Refusal> {
	uuid::Uuid::parse_str(value)
		.map(|id| *id.as_bytes())
		.map_err(|_| Refusal::invalid(format!("{what} is not a UUID")))
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VolumeKey {
	pub partition_key: String,
	#[serde(default)]
	pub name: Option<String>,
}

impl VolumeKey {
	pub(crate) fn wire(&self) -> Result<wire::VolumeKey, Refusal> {
		let key = wire::VolumeKey {
			partition_key: unhex(&self.partition_key, "partitionKey")?.into(),
			name: self
				.name
				.as_deref()
				.map(|name| unhex(name, "name"))
				.transpose()?
				.map(Into::into),
		};
		if key.partition_key.len() + key.name.as_ref().map_or(0, |name| name.len()) > 4096 {
			return Err(Refusal::invalid("volume key is longer than 4096 bytes"));
		}
		Ok(key)
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VolumeLogKey {
	pub volume_key: VolumeKey,
	pub volume_incarnation: String,
}

impl VolumeLogKey {
	pub(crate) fn wire(&self) -> Result<wire::VolumeLogKey, Refusal> {
		Ok(wire::VolumeLogKey {
			volume_key: self.volume_key.wire()?,
			volume_incarnation: uuid(&self.volume_incarnation, "volumeIncarnation")?,
		})
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Position {
	pub volume_seq: u64,
	pub client_epoch: u64,
}

impl Position {
	pub(crate) fn wire(&self) -> wire::VolumeLogPosition {
		wire::VolumeLogPosition {
			volume_seq: self.volume_seq,
			client_epoch: self.client_epoch,
		}
	}
}

/// A filesystem row address, named as on the wire.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum RowKey {
	FsFormatKey,
	FsInodeKey { ino: u64 },
	FsDentryKey { ino: u64, name: String },
	FsLinkKey { ino: u64, parent: u64, name: String },
	FsXattrKey { ino: u64, name: String },
	FsKvKey { ino: u64, key: String },
	FsOrphanKey { ino: u64 },
	FsChunkKey { ino: u64, index: u64 },
}

impl RowKey {
	pub(crate) fn wire(&self) -> Result<read::FsRowKey, Refusal> {
		Ok(match self {
			RowKey::FsFormatKey => read::FsRowKey::FsFormatKey,
			RowKey::FsInodeKey { ino } => {
				read::FsRowKey::FsInodeKey(read::FsInodeKey { ino: *ino })
			}
			RowKey::FsDentryKey { ino, name } => read::FsRowKey::FsDentryKey(read::FsDentryKey {
				ino: *ino,
				name: unhex(name, "name")?.into(),
			}),
			RowKey::FsLinkKey { ino, parent, name } => read::FsRowKey::FsLinkKey(read::FsLinkKey {
				ino: *ino,
				parent: *parent,
				name: unhex(name, "name")?.into(),
			}),
			RowKey::FsXattrKey { ino, name } => read::FsRowKey::FsXattrKey(read::FsXattrKey {
				ino: *ino,
				name: unhex(name, "name")?.into(),
			}),
			RowKey::FsKvKey { ino, key } => read::FsRowKey::FsKvKey(read::FsKvKey {
				ino: *ino,
				key: unhex(key, "key")?.into(),
			}),
			RowKey::FsOrphanKey { ino } => {
				read::FsRowKey::FsOrphanKey(read::FsOrphanKey { ino: *ino })
			}
			RowKey::FsChunkKey { ino, index } => read::FsRowKey::FsChunkKey(read::FsChunkKey {
				ino: *ino,
				index: *index,
			}),
		})
	}
}

/// A range of rows to scan, named as on the wire.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum RowRange {
	FsAllMetadataRows,
	FsMetadataRows {
		ino: u64,
	},
	FsDirectoryRows {
		ino: u64,
	},
	FsXattrRows {
		ino: u64,
	},
	FsKvRows {
		ino: u64,
		start: String,
		#[serde(default)]
		end: Option<String>,
	},
	FsOrphanRows,
}

impl RowRange {
	pub(crate) fn wire(&self) -> Result<read::FsRowRange, Refusal> {
		Ok(match self {
			RowRange::FsAllMetadataRows => read::FsRowRange::FsAllMetadataRows,
			RowRange::FsMetadataRows { ino } => {
				read::FsRowRange::FsMetadataRows(read::FsMetadataRows { ino: *ino })
			}
			RowRange::FsDirectoryRows { ino } => {
				read::FsRowRange::FsDirectoryRows(read::FsDirectoryRows { ino: *ino })
			}
			RowRange::FsXattrRows { ino } => {
				read::FsRowRange::FsXattrRows(read::FsXattrRows { ino: *ino })
			}
			RowRange::FsKvRows { ino, start, end } => read::FsRowRange::FsKvRows(read::FsKvRows {
				ino: *ino,
				start: unhex(start, "start")?.into(),
				end: end
					.as_deref()
					.map(|end| unhex(end, "end"))
					.transpose()?
					.map(Into::into),
			}),
			RowRange::FsOrphanRows => read::FsRowRange::FsOrphanRows,
		})
	}
}


