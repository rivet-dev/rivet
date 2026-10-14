use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = AuthApplicationTokenCreateRequest)]
pub struct CreateRequest {
	pub audience: String,
	pub purpose: String,
	pub duration: Option<u64>,
	pub subject: Option<String>,
	pub claims: Map<String, Value>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[schema(as = AuthApplicationTokenCreateResponse)]
pub struct CreateResponse {
	pub token: String,
	pub issued_ts: i64,
	pub expires_ts: i64,
}
