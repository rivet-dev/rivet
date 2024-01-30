use proto::backend::pkg::*;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::ApiTryFrom;

#[derive(Clone, Debug)]
pub struct DestructuredKvEntry {
	pub key: String,
	pub value: Option<Vec<u8>>,
	pub msg_ts: i64,
}

impl ApiTryFrom<kv::list::response::Entry> for models::KvEntry {
	type Error = GlobalError;

	fn api_try_from(value: kv::list::response::Entry) -> GlobalResult<models::KvEntry> {
		Ok(models::KvEntry {
			key: value.key,
			value: value
				.value
				.map(|v| serde_json::from_slice(&v))
				.transpose()?,
			deleted: None,
		})
	}
}

impl ApiTryFrom<kv::msg::update::Message> for models::KvEntry {
	type Error = GlobalError;

	fn api_try_from(value: kv::msg::update::Message) -> GlobalResult<models::KvEntry> {
		let deleted = value.value.is_some().then(|| true);

		Ok(models::KvEntry {
			key: value.key,
			value: value
				.value
				.map(|v| serde_json::from_slice(&v))
				.transpose()?,
			deleted,
		})
	}
}

impl ApiTryFrom<DestructuredKvEntry> for models::KvEntry {
	type Error = GlobalError;

	fn api_try_from(value: DestructuredKvEntry) -> GlobalResult<models::KvEntry> {
		let deleted = value.value.is_some().then(|| true);

		Ok(models::KvEntry {
			key: value.key,
			value: value
				.value
				.map(|v| serde_json::from_slice(&v))
				.transpose()?,
			deleted,
		})
	}
}
