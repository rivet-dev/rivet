use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub id: u64,
    pub name: String,
    pub args: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    #[serde(rename = "eventName")]
    pub event_name: String,
    pub subscribe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", content = "val")]
pub enum ToServerBody {
    ActionRequest(ActionRequest),
    SubscriptionRequest(SubscriptionRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToServer {
    pub body: ToServerBody,
}
