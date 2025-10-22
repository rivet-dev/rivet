use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    #[serde(rename = "actorId")]
    pub actor_id: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "connectionToken")]
    pub connection_token: String,
}

// Used for connection errors (both during initialization and afterwards)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub group: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<u8>>,
    #[serde(rename = "actionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub id: u64,
    pub output: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub args: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", content = "val")]
pub enum ToClientBody {
    Init(Init),
    Error(Error),
    ActionResponse(ActionResponse),
    Event(Event),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToClient {
    pub body: ToClientBody,
}