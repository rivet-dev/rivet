use rivet_auth_policy::OwnedGrant;
use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueRequest {
	pub namespace_id: Id,
	pub grants: Vec<OwnedGrant<Id, Id>>,
	pub expires_no_later_than_ts: i64,
	pub subject: Option<String>,
	pub issuer_token_id: Id,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueResponse {
	pub token: String,
	pub issued_ts: i64,
	pub expires_ts: i64,
	pub kid: String,
	pub jti: String,
}
