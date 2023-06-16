#[allow(unused_imports)]
use chrono;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

/// A new entry to insert into the key-value database.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutEntry {
	/// A string reprenting a key in the key-value database. Key path components are split by a slash (e.g. `a/b/c` has the path components `["a", "b", "c"]`). Slashes can be escaped by using a forward slash (e.g. `a/b\/c/d` has the path components `["a", "b/c", "d"]`). See `rivet.api.kv.common#KeyComponents` for the structure of a `rivet.api.kv.common#Key` split by `/`.
	pub key: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub value: serde_json::Value,
}

/// Provided by watchable endpoints used in blocking loops.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WatchResponse {
	/// Index indicating the version of the data responded. Pas this to `rivet.common#WatchQuery` to block and wait for the next response.
	pub index: std::string::String,
}

/// A key-value entry.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KvEntry {
	/// A key separated into components.
	pub key: std::vec::Vec<std::string::String>,
	#[allow(missing_docs)] // documentation missing in model
	pub value: std::option::Option<serde_json::Value>,
	#[allow(missing_docs)] // documentation missing in model
	pub deleted: std::option::Option<bool>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutBatchRequest {
	/// A universally unique identifier.
	pub namespace_id: std::option::Option<std::string::String>,
	/// A list of entries to insert.
	pub entries: std::vec::Vec<PutEntry>,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteBatchRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetBatchRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutRequest {
	/// A universally unique identifier.
	pub namespace_id: std::option::Option<std::string::String>,
	/// Any JSON value to set the key to.
	pub key: std::string::String,
	#[allow(missing_docs)] // documentation missing in model
	pub value: serde_json::Value,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetRequest {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutBatchResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteBatchResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetBatchResponse {
	/// A list of key-value entries.
	pub entries: std::vec::Vec<KvEntry>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteResponse {}

#[allow(missing_docs)] // documentation missing in model
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetResponse {
	/// The key's JSON value.
	pub value: std::option::Option<serde_json::Value>,
	/// Whether or not the entry has been deleted. Only set when watching this endpoint.
	pub deleted: std::option::Option<bool>,
	/// Provided by watchable endpoints used in blocking loops.
	pub watch: WatchResponse,
}

