use serde::Serialize;

#[derive(Clone, PartialEq, ::prost::Message, Serialize)]
pub struct Metadata {
	#[prost(string, tag = "1")]
	pub kv_version: ::prost::alloc::string::String,
	#[prost(int64, tag = "2")]
	pub create_ts: i64,
}
