use anyhow::*;
use async_trait::async_trait;
use rivet_runner_protocol as rp;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

use super::protocol;

/// Configuration passed to actor when it starts
#[derive(Clone)]
pub struct ActorConfig {
	pub actor_id: String,
	pub generation: u32,
	pub name: String,
	pub key: Option<String>,
	pub create_ts: i64,
	pub input: Option<Vec<u8>>,

	/// Channel to send events to the runner
	pub event_tx: mpsc::UnboundedSender<ActorEvent>,

	/// Channel to send KV requests to the runner
	pub kv_request_tx: mpsc::UnboundedSender<KvRequest>,
}

impl ActorConfig {
	pub fn new(
		config: &rp::ActorConfig,
		actor_id: String,
		generation: u32,
		event_tx: mpsc::UnboundedSender<ActorEvent>,
		kv_request_tx: mpsc::UnboundedSender<KvRequest>,
	) -> Self {
		ActorConfig {
			actor_id,
			generation,
			name: config.name.clone(),
			key: config.key.clone(),
			create_ts: config.create_ts,
			input: config.input.as_ref().map(|i| i.to_vec()),
			event_tx,
			kv_request_tx,
		}
	}
}

impl ActorConfig {
	/// Send a sleep intent
	pub fn send_sleep_intent(&self) {
		let event = protocol::make_actor_intent(
			&self.actor_id,
			self.generation,
			rp::ActorIntent::ActorIntentSleep,
		);
		self.send_event(event);
	}

	/// Send a stop intent
	pub fn send_stop_intent(&self) {
		let event = protocol::make_actor_intent(
			&self.actor_id,
			self.generation,
			rp::ActorIntent::ActorIntentStop,
		);
		self.send_event(event);
	}

	/// Set an alarm to wake at specified timestamp (milliseconds)
	pub fn send_set_alarm(&self, alarm_ts: i64) {
		let event = protocol::make_set_alarm(&self.actor_id, self.generation, Some(alarm_ts));
		self.send_event(event);
	}

	/// Clear the alarm
	pub fn send_clear_alarm(&self) {
		let event = protocol::make_set_alarm(&self.actor_id, self.generation, None);
		self.send_event(event);
	}

	/// Send a custom event
	fn send_event(&self, event: rp::Event) {
		let actor_event = ActorEvent {
			actor_id: self.actor_id.clone(),
			generation: self.generation,
			event,
		};
		let _ = self.event_tx.send(actor_event);
	}

	/// Send a KV get request
	pub async fn send_kv_get(&self, keys: Vec<Vec<u8>>) -> Result<rp::KvGetResponse> {
		let (response_tx, response_rx) = oneshot::channel();
		let request = KvRequest {
			actor_id: self.actor_id.clone(),
			data: rp::KvRequestData::KvGetRequest(rp::KvGetRequest { keys }),
			response_tx,
		};
		self.kv_request_tx
			.send(request)
			.map_err(|_| anyhow!("failed to send KV get request"))?;
		let response: rp::KvResponseData = response_rx
			.await
			.map_err(|_| anyhow!("KV get request response channel closed"))?;

		match response {
			rp::KvResponseData::KvGetResponse(data) => Ok(data),
			rp::KvResponseData::KvErrorResponse(err) => {
				Err(anyhow!("KV get failed: {}", err.message))
			}
			_ => Err(anyhow!("unexpected response type for KV get")),
		}
	}

	/// Send a KV list request
	pub async fn send_kv_list(
		&self,
		query: rp::KvListQuery,
		reverse: Option<bool>,
		limit: Option<u64>,
	) -> Result<rp::KvListResponse> {
		let (response_tx, response_rx) = oneshot::channel();
		let request = KvRequest {
			actor_id: self.actor_id.clone(),
			data: rp::KvRequestData::KvListRequest(rp::KvListRequest {
				query,
				reverse,
				limit,
			}),
			response_tx,
		};
		self.kv_request_tx
			.send(request)
			.map_err(|_| anyhow!("failed to send KV list request"))?;
		let response: rp::KvResponseData = response_rx
			.await
			.map_err(|_| anyhow!("KV list request response channel closed"))?;

		match response {
			rp::KvResponseData::KvListResponse(data) => Ok(data),
			rp::KvResponseData::KvErrorResponse(err) => {
				Err(anyhow!("KV list failed: {}", err.message))
			}
			_ => Err(anyhow!("unexpected response type for KV list")),
		}
	}

	/// Send a KV put request
	pub async fn send_kv_put(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<()> {
		let (response_tx, response_rx) = oneshot::channel();
		let request = KvRequest {
			actor_id: self.actor_id.clone(),
			data: rp::KvRequestData::KvPutRequest(rp::KvPutRequest { keys, values }),
			response_tx,
		};

		self.kv_request_tx
			.send(request)
			.map_err(|_| anyhow!("failed to send KV put request"))?;

		let response: rp::KvResponseData = response_rx
			.await
			.map_err(|_| anyhow!("KV put request response channel closed"))?;

		match response {
			rp::KvResponseData::KvPutResponse => Ok(()),
			rp::KvResponseData::KvErrorResponse(err) => {
				Err(anyhow!("KV put failed: {}", err.message))
			}
			_ => Err(anyhow!("unexpected response type for KV put")),
		}
	}

	/// Send a KV delete request
	pub async fn send_kv_delete(&self, keys: Vec<Vec<u8>>) -> Result<()> {
		let (response_tx, response_rx) = oneshot::channel();
		let request = KvRequest {
			actor_id: self.actor_id.clone(),
			data: rp::KvRequestData::KvDeleteRequest(rp::KvDeleteRequest { keys }),
			response_tx,
		};
		self.kv_request_tx
			.send(request)
			.map_err(|_| anyhow!("failed to send KV delete request"))?;
		let response: rp::KvResponseData = response_rx
			.await
			.map_err(|_| anyhow!("KV delete request response channel closed"))?;

		match response {
			rp::KvResponseData::KvDeleteResponse => Ok(()),
			rp::KvResponseData::KvErrorResponse(err) => {
				Err(anyhow!("KV delete failed: {}", err.message))
			}
			_ => Err(anyhow!("unexpected response type for KV delete")),
		}
	}

	/// Send a KV drop request
	pub async fn send_kv_drop(&self) -> Result<()> {
		let (response_tx, response_rx) = oneshot::channel();
		let request = KvRequest {
			actor_id: self.actor_id.clone(),
			data: rp::KvRequestData::KvDropRequest,
			response_tx,
		};
		self.kv_request_tx
			.send(request)
			.map_err(|_| anyhow!("failed to send KV drop request"))?;
		let response: rp::KvResponseData = response_rx
			.await
			.map_err(|_| anyhow!("KV drop request response channel closed"))?;

		match response {
			rp::KvResponseData::KvDropResponse => Ok(()),
			rp::KvResponseData::KvErrorResponse(err) => {
				Err(anyhow!("KV drop failed: {}", err.message))
			}
			_ => Err(anyhow!("unexpected response type for KV drop")),
		}
	}
}

/// Result of actor start operation
#[derive(Debug, Clone)]
pub enum ActorStartResult {
	/// Send ActorStateRunning immediately
	Running,
	/// Wait specified duration before sending running
	Delay(Duration),
	/// Never send running (simulates timeout)
	Timeout,
	/// Crash immediately with exit code
	Crash { code: i32, message: String },
}

/// Result of actor stop operation
#[derive(Debug, Clone)]
pub enum ActorStopResult {
	/// Stop successfully (exit code 0)
	Success,
	/// Wait before stopping
	Delay(Duration),
	/// Crash with exit code
	Crash { code: i32, message: String },
}

/// Trait for test actors that can be controlled programmatically
#[async_trait]
pub trait TestActor: Send + Sync {
	/// Called when actor receives start command
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult>;

	/// Called when actor receives stop command
	async fn on_stop(&mut self) -> Result<ActorStopResult>;

	/// Called when actor receives alarm wake signal
	async fn on_alarm(&mut self) -> Result<()> {
		tracing::debug!("actor received alarm (default no-op)");
		Ok(())
	}

	/// Called when actor receives wake signal (from sleep)
	async fn on_wake(&mut self) -> Result<()> {
		tracing::debug!("actor received wake (default no-op)");
		Ok(())
	}

	/// Get actor's name for logging
	fn name(&self) -> &str {
		"TestActor"
	}
}

/// Events that actors can send directly via the event channel
#[derive(Debug, Clone)]
pub struct ActorEvent {
	pub actor_id: String,
	pub generation: u32,
	pub event: rp::Event,
}

/// KV requests that actors can send to the runner
pub struct KvRequest {
	pub actor_id: String,
	pub data: rp::KvRequestData,
	pub response_tx: oneshot::Sender<rp::KvResponseData>,
}
