use gas::prelude::*;
use rivet_key_data::generated::pegboard_runner_address_v1;
use rivet_runner_protocol::protocol;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Runner {
	pub runner_id: Id,
	pub namespace_id: Id,
	pub datacenter: String,
	pub name: String,
	pub key: String,
	pub version: u32,
	pub total_slots: u32,
	pub remaining_slots: u32,
	pub addresses_http: StringHttpAddressHashableMap,
	pub addresses_tcp: StringTcpAddressHashableMap,
	pub addresses_udp: StringUdpAddressHashableMap,
	pub create_ts: i64,
	pub drain_ts: Option<i64>,
	pub stop_ts: Option<i64>,
	pub last_ping_ts: i64,
	pub last_connected_ts: Option<i64>,
	pub last_rtt: u32,
	pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

// HACK: We can't define ToSchema on HashableMap directly, so we have to define concrete types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringHttpAddressHashableMap(
	util::serde::HashableMap<String, pegboard_runner_address_v1::Http>,
);

impl From<util::serde::HashableMap<String, pegboard_runner_address_v1::Http>>
	for StringHttpAddressHashableMap
{
	fn from(value: util::serde::HashableMap<String, pegboard_runner_address_v1::Http>) -> Self {
		Self(value)
	}
}

impl Deref for StringHttpAddressHashableMap {
	type Target = util::serde::HashableMap<String, pegboard_runner_address_v1::Http>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl utoipa::ToSchema for StringHttpAddressHashableMap {}

impl utoipa::PartialSchema for StringHttpAddressHashableMap {
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
		utoipa::openapi::ObjectBuilder::new()
			.additional_properties(Some(protocol::RunnerAddressHttp::schema()))
			.into()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringTcpAddressHashableMap(
	util::serde::HashableMap<String, pegboard_runner_address_v1::Tcp>,
);

impl From<util::serde::HashableMap<String, pegboard_runner_address_v1::Tcp>>
	for StringTcpAddressHashableMap
{
	fn from(value: util::serde::HashableMap<String, pegboard_runner_address_v1::Tcp>) -> Self {
		Self(value)
	}
}

impl Deref for StringTcpAddressHashableMap {
	type Target = util::serde::HashableMap<String, pegboard_runner_address_v1::Tcp>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl utoipa::ToSchema for StringTcpAddressHashableMap {}

impl utoipa::PartialSchema for StringTcpAddressHashableMap {
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
		utoipa::openapi::ObjectBuilder::new()
			.additional_properties(Some(protocol::RunnerAddressTcp::schema()))
			.into()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringUdpAddressHashableMap(
	util::serde::HashableMap<String, pegboard_runner_address_v1::Udp>,
);

impl From<util::serde::HashableMap<String, pegboard_runner_address_v1::Udp>>
	for StringUdpAddressHashableMap
{
	fn from(value: util::serde::HashableMap<String, pegboard_runner_address_v1::Udp>) -> Self {
		Self(value)
	}
}

impl Deref for StringUdpAddressHashableMap {
	type Target = util::serde::HashableMap<String, pegboard_runner_address_v1::Udp>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl utoipa::ToSchema for StringUdpAddressHashableMap {}

impl utoipa::PartialSchema for StringUdpAddressHashableMap {
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
		utoipa::openapi::ObjectBuilder::new()
			.additional_properties(Some(protocol::RunnerAddressUdp::schema()))
			.into()
	}
}
