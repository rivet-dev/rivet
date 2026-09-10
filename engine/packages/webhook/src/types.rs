use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebhookEventType {
	#[serde(rename = "runner_pool.error")]
	RunnerPoolError,
	#[serde(rename = "runner_pool.healthy")]
	RunnerPoolHealthy,
	#[serde(rename = "actor.http_request")]
	ActorHttpRequest,
}

impl WebhookEventType {
	pub fn is_webhook_safe(&self) -> bool {
		match self {
			WebhookEventType::RunnerPoolError => true,
			WebhookEventType::RunnerPoolHealthy => true,
			WebhookEventType::ActorHttpRequest => false,
		}
	}

	pub fn as_cloudevents_type(&self) -> &'static str {
		match self {
			WebhookEventType::RunnerPoolError => "dev.rivet.runner_pool.error",
			WebhookEventType::RunnerPoolHealthy => "dev.rivet.runner_pool.healthy",
			WebhookEventType::ActorHttpRequest => "dev.rivet.actor.http_request",
		}
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			WebhookEventType::RunnerPoolError => "runner_pool.error",
			WebhookEventType::RunnerPoolHealthy => "runner_pool.healthy",
			WebhookEventType::ActorHttpRequest => "actor.http_request",
		}
	}
}

impl From<rivet_data::generated::webhook_config_v1::WebhookEventType> for WebhookEventType {
	fn from(value: rivet_data::generated::webhook_config_v1::WebhookEventType) -> Self {
		match value {
			rivet_data::generated::webhook_config_v1::WebhookEventType::RunnerPoolError => {
				WebhookEventType::RunnerPoolError
			}
			rivet_data::generated::webhook_config_v1::WebhookEventType::RunnerPoolHealthy => {
				WebhookEventType::RunnerPoolHealthy
			}
			rivet_data::generated::webhook_config_v1::WebhookEventType::ActorHttpRequest => {
				WebhookEventType::ActorHttpRequest
			}
		}
	}
}

impl From<WebhookEventType> for rivet_data::generated::webhook_config_v1::WebhookEventType {
	fn from(value: WebhookEventType) -> Self {
		match value {
			WebhookEventType::RunnerPoolError => {
				rivet_data::generated::webhook_config_v1::WebhookEventType::RunnerPoolError
			}
			WebhookEventType::RunnerPoolHealthy => {
				rivet_data::generated::webhook_config_v1::WebhookEventType::RunnerPoolHealthy
			}
			WebhookEventType::ActorHttpRequest => {
				rivet_data::generated::webhook_config_v1::WebhookEventType::ActorHttpRequest
			}
		}
	}
}

impl From<rivet_data::generated::webhook_delivery_v1::WebhookEventType> for WebhookEventType {
	fn from(value: rivet_data::generated::webhook_delivery_v1::WebhookEventType) -> Self {
		match value {
			rivet_data::generated::webhook_delivery_v1::WebhookEventType::RunnerPoolError => {
				WebhookEventType::RunnerPoolError
			}
			rivet_data::generated::webhook_delivery_v1::WebhookEventType::RunnerPoolHealthy => {
				WebhookEventType::RunnerPoolHealthy
			}
			rivet_data::generated::webhook_delivery_v1::WebhookEventType::ActorHttpRequest => {
				WebhookEventType::ActorHttpRequest
			}
		}
	}
}

impl From<WebhookEventType> for rivet_data::generated::webhook_delivery_v1::WebhookEventType {
	fn from(value: WebhookEventType) -> Self {
		match value {
			WebhookEventType::RunnerPoolError => {
				rivet_data::generated::webhook_delivery_v1::WebhookEventType::RunnerPoolError
			}
			WebhookEventType::RunnerPoolHealthy => {
				rivet_data::generated::webhook_delivery_v1::WebhookEventType::RunnerPoolHealthy
			}
			WebhookEventType::ActorHttpRequest => {
				rivet_data::generated::webhook_delivery_v1::WebhookEventType::ActorHttpRequest
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookConfig {
	pub url: String,
	pub headers: HashMap<String, String>,
	pub subscriptions: Vec<WebhookEventType>,
}

impl From<rivet_data::generated::webhook_config_v1::Data> for WebhookConfig {
	fn from(value: rivet_data::generated::webhook_config_v1::Data) -> Self {
		WebhookConfig {
			url: value.url,
			headers: value.headers,
			subscriptions: value.subscriptions.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<WebhookConfig> for rivet_data::generated::webhook_config_v1::Data {
	fn from(value: WebhookConfig) -> Self {
		rivet_data::generated::webhook_config_v1::Data {
			url: value.url,
			headers: value.headers,
			subscriptions: value.subscriptions.into_iter().map(Into::into).collect(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
	Pending,
	Succeeded,
	Failed,
}

impl From<rivet_data::generated::webhook_delivery_v1::DeliveryStatus> for DeliveryStatus {
	fn from(value: rivet_data::generated::webhook_delivery_v1::DeliveryStatus) -> Self {
		match value {
			rivet_data::generated::webhook_delivery_v1::DeliveryStatus::Pending => {
				DeliveryStatus::Pending
			}
			rivet_data::generated::webhook_delivery_v1::DeliveryStatus::Succeeded => {
				DeliveryStatus::Succeeded
			}
			rivet_data::generated::webhook_delivery_v1::DeliveryStatus::Failed => {
				DeliveryStatus::Failed
			}
		}
	}
}

impl From<DeliveryStatus> for rivet_data::generated::webhook_delivery_v1::DeliveryStatus {
	fn from(value: DeliveryStatus) -> Self {
		match value {
			DeliveryStatus::Pending => {
				rivet_data::generated::webhook_delivery_v1::DeliveryStatus::Pending
			}
			DeliveryStatus::Succeeded => {
				rivet_data::generated::webhook_delivery_v1::DeliveryStatus::Succeeded
			}
			DeliveryStatus::Failed => {
				rivet_data::generated::webhook_delivery_v1::DeliveryStatus::Failed
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookEvent {
	pub event_type: WebhookEventType,
	/// CloudEvents `subject`: which resource within the source the event is about, such as the
	/// runner name for a runner pool event.
	pub subject: Option<String>,
	/// CloudEvents `data`.
	pub data: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryRecord {
	pub payload: String,
	pub status: DeliveryStatus,
	pub attempt_count: u32,
	pub last_error: Option<String>,
	pub created_at: i64,
	pub event_type: WebhookEventType,
	pub subject: Option<String>,
}

impl From<rivet_data::generated::webhook_delivery_v1::Data> for DeliveryRecord {
	fn from(value: rivet_data::generated::webhook_delivery_v1::Data) -> Self {
		DeliveryRecord {
			payload: value.payload,
			status: value.status.into(),
			attempt_count: value.attempt_count,
			last_error: value.last_error,
			created_at: value.created_at,
			event_type: value.event_type.into(),
			subject: value.subject,
		}
	}
}

impl From<DeliveryRecord> for rivet_data::generated::webhook_delivery_v1::Data {
	fn from(value: DeliveryRecord) -> Self {
		rivet_data::generated::webhook_delivery_v1::Data {
			payload: value.payload,
			status: value.status.into(),
			attempt_count: value.attempt_count,
			last_error: value.last_error,
			created_at: value.created_at,
			event_type: value.event_type.into(),
			subject: value.subject,
		}
	}
}
