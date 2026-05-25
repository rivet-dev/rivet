use std::sync::{Arc, Weak};
use std::sync::atomic::Ordering;

use crate::async_counter::AsyncCounter;
use rivet_envoy_protocol as protocol;
use tokio::sync::oneshot;

use crate::context::SharedContext;
use crate::envoy::{ActorInfo, ToEnvoyMessage};
use crate::metrics::METRICS;
use crate::sqlite::{RemoteSqliteRequest, RemoteSqliteResponse, SqliteRequest, SqliteResponse};
use crate::tunnel::HibernatingWebSocketMetadata;
use crate::utils::tunnel_request_key;

/// Handle for interacting with the envoy from callbacks.
#[derive(Clone)]
pub struct EnvoyHandle {
	pub(crate) shared: Arc<SharedContext>,
	pub(crate) started_rx: tokio::sync::watch::Receiver<()>,
}

#[derive(Clone)]
pub struct EnvoyStatusHandle {
	shared: Weak<SharedContext>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerlessActorStart {
	pub actor_id: String,
	pub generation: u32,
}

impl EnvoyHandle {
	#[doc(hidden)]
	pub fn from_shared(shared: Arc<SharedContext>) -> Self {
		Self {
			shared,
			started_rx: tokio::sync::watch::channel(()).1,
		}
	}

	pub fn shutdown(&self, immediate: bool) {
		self.shared.shutting_down.store(true, Ordering::Release);

		if immediate {
			let _ = crate::envoy::send_to_envoy_tx(&self.shared, ToEnvoyMessage::Stop);
		} else {
			let _ = crate::envoy::send_to_envoy_tx(&self.shared, ToEnvoyMessage::Shutdown);
		}
	}

	/// True once the envoy loop has finished its cleanup block. Latched: stays
	/// true forever after the loop exits.
	pub fn is_stopped(&self) -> bool {
		*self.shared.stopped_tx.borrow()
	}

	/// Resolves when the envoy loop has finished its cleanup block.
	///
	/// Returning does NOT imply successful delivery of pending KV/SQLite/tunnel
	/// requests. The cleanup block errors out every outstanding request with
	/// `EnvoyShutdownError`. Callers needing durability must wait on individual
	/// request acks before invoking shutdown.
	///
	/// Latched: safe to call before, during, or after the envoy loop exits.
	/// A waiter arriving after the loop already exited resolves immediately.
	pub async fn wait_stopped(&self) {
		let mut rx = self.shared.stopped_tx.subscribe();
		if *rx.borrow_and_update() {
			return;
		}
		let _ = rx.changed().await;
	}

	/// Convenience: signal shutdown then await `wait_stopped`.
	pub async fn shutdown_and_wait(&self, immediate: bool) {
		self.shutdown(immediate);
		self.wait_stopped().await;
	}

	pub async fn get_protocol_metadata(&self) -> Option<protocol::ProtocolMetadata> {
		self.shared.protocol_metadata.lock().await.clone()
	}

	/// Threshold for `is_ping_healthy`.
	pub const PING_HEALTHY_THRESHOLD_MS: i64 = 20_000;

	/// Epoch ms timestamp of the most recent engine ping.
	pub fn last_ping_at_ms(&self) -> Option<i64> {
		let last = self.shared.last_ping_ts.load(Ordering::Acquire);
		if last == 0 {
			None
		} else {
			Some(last)
		}
	}

	/// Milliseconds since the most recent engine ping.
	pub fn last_ping_age_ms(&self) -> Option<i64> {
		self.last_ping_at_ms()
			.map(|last| crate::time::now_millis().saturating_sub(last))
	}

	/// True when the most recent engine ping timestamp is within `PING_HEALTHY_THRESHOLD_MS`.
	/// Fresh envoys start healthy until the threshold elapses without a ping.
	pub fn is_ping_healthy(&self) -> bool {
		self.last_ping_age_ms()
			.is_some_and(|age_ms| age_ms < Self::PING_HEALTHY_THRESHOLD_MS)
	}

	pub fn get_envoy_key(&self) -> &str {
		&self.shared.envoy_key
	}

	pub fn endpoint(&self) -> &str {
		&self.shared.config.endpoint
	}

	pub fn token(&self) -> Option<&str> {
		self.shared.config.token.as_deref()
	}

	pub fn namespace(&self) -> &str {
		&self.shared.config.namespace
	}

	pub fn active_actor_count(&self) -> usize {
		active_actor_count(&self.shared)
	}

	pub fn status_handle(&self) -> EnvoyStatusHandle {
		EnvoyStatusHandle {
			shared: Arc::downgrade(&self.shared),
		}
	}

	pub fn pool_name(&self) -> &str {
		&self.shared.config.pool_name
	}

	pub async fn started(&self) -> anyhow::Result<()> {
		self.started_rx
			.clone()
			.changed()
			.await
			.map_err(|_| anyhow::anyhow!("envoy stopped before startup completed"))?;
		Ok(())
	}

	pub fn sleep_actor(&self, actor_id: String, generation: Option<u32>) {
		let _ = crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::ActorIntent {
				actor_id,
				generation,
				intent: protocol::ActorIntent::ActorIntentSleep,
				error: None,
			},
		);
	}

	pub fn stop_actor(&self, actor_id: String, generation: Option<u32>, error: Option<String>) {
		let _ = crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::ActorIntent {
				actor_id,
				generation,
				intent: protocol::ActorIntent::ActorIntentStop,
				error,
			},
		);
	}

	pub fn destroy_actor(&self, actor_id: String, generation: Option<u32>) {
		let _ = crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::ActorIntent {
				actor_id,
				generation,
				intent: protocol::ActorIntent::ActorIntentStop,
				error: None,
			},
		);
	}

	pub async fn get_actor(&self, actor_id: &str, generation: Option<u32>) -> Option<ActorInfo> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::GetActor {
				actor_id: actor_id.to_string(),
				generation,
				response_tx: tx,
			},
		)
		.ok()?;
		rx.await.ok().flatten()
	}

	pub async fn wait_actor_registered_then_stopped(&self, actor_id: &str, generation: u32) {
		let mut registered = false;
		loop {
			let notified = self.shared.actors_notify.notified();
			if self.is_stopped() {
				return;
			}

			let actor_is_registered = self
				.shared
				.actors
				.read_async(actor_id, |_, generations| {
					generations.contains_sync(&generation)
				})
				.await
				.unwrap_or(false);

			if registered && !actor_is_registered {
				return;
			}
			if actor_is_registered {
				registered = true;
			}

			tokio::select! {
				_ = notified => {}
				_ = self.wait_stopped() => return,
			}
		}
	}

	pub fn http_request_counter(
		&self,
		actor_id: &str,
		generation: Option<u32>,
	) -> Option<Arc<AsyncCounter>> {
		self.shared.actors.read_sync(actor_id, |_, generations| {
			if let Some(generation) = generation {
				return generations.read_sync(&generation, |_, actor| {
					actor.active_http_request_count.clone()
				});
			}

			let mut best: Option<(u32, Arc<AsyncCounter>)> = None;
			generations.iter_sync(|generation, actor| {
				if !actor.handle.is_closed()
					&& best
						.as_ref()
						.is_none_or(|(best_generation, _)| generation > best_generation)
				{
					best = Some((*generation, actor.active_http_request_count.clone()));
				}
				true
			});
			best.map(|(_, counter)| counter)
		})?
	}

	pub async fn get_active_http_request_count(
		&self,
		actor_id: &str,
		generation: Option<u32>,
	) -> Option<usize> {
		self.http_request_counter(actor_id, generation)
			.map(|counter| counter.load())
	}

	pub fn hibernatable_connection_is_live(
		&self,
		actor_id: &str,
		_generation: Option<u32>,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
	) -> bool {
		let key = tunnel_request_key(&gateway_id, &request_id);
		if self
			.shared
			.live_tunnel_requests
			.read_sync(&key, |_, live_actor_id| live_actor_id == actor_id)
			.unwrap_or(false)
		{
			return true;
		}

		self.shared
			.pending_hibernation_restores
			.read_sync(actor_id, |_, entries| {
				entries
					.iter()
					.any(|entry| entry.gateway_id == gateway_id && entry.request_id == request_id)
			})
			.unwrap_or(false)
	}

	pub fn set_alarm(&self, actor_id: String, alarm_ts: Option<i64>, generation: Option<u32>) {
		self.set_alarm_with_ack(actor_id, alarm_ts, generation, None);
	}

	pub fn set_alarm_with_ack(
		&self,
		actor_id: String,
		alarm_ts: Option<i64>,
		generation: Option<u32>,
		ack_tx: Option<oneshot::Sender<()>>,
	) {
		let _ = crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::SetAlarm {
				actor_id,
				generation,
				alarm_ts,
				ack_tx,
			},
		);
	}

	pub async fn kv_get(
		&self,
		actor_id: String,
		keys: Vec<Vec<u8>>,
	) -> anyhow::Result<Vec<Option<Vec<u8>>>> {
		let request_keys = keys.clone();
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvGetRequest(protocol::KvGetRequest { keys }),
			)
			.await?;

		match response {
			protocol::KvResponseData::KvGetResponse(resp) => {
				let mut result = Vec::with_capacity(request_keys.len());
				for requested_key in &request_keys {
					let mut found = false;
					for (i, resp_key) in resp.keys.iter().enumerate() {
						if requested_key == resp_key {
							result.push(Some(resp.values[i].clone()));
							found = true;
							break;
						}
					}
					if !found {
						result.push(None);
					}
				}
				Ok(result)
			}
			protocol::KvResponseData::KvErrorResponse(e) => {
				anyhow::bail!("{}", e.message)
			}
			_ => anyhow::bail!("unexpected KV response type"),
		}
	}

	pub async fn kv_list_all(
		&self,
		actor_id: String,
		reverse: Option<bool>,
		limit: Option<u64>,
	) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvListRequest(protocol::KvListRequest {
					query: protocol::KvListQuery::KvListAllQuery,
					reverse,
					limit,
				}),
			)
			.await?;
		parse_list_response(response)
	}

	pub async fn kv_list_range(
		&self,
		actor_id: String,
		start: Vec<u8>,
		end: Vec<u8>,
		exclusive: bool,
		reverse: Option<bool>,
		limit: Option<u64>,
	) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvListRequest(protocol::KvListRequest {
					query: protocol::KvListQuery::KvListRangeQuery(protocol::KvListRangeQuery {
						start,
						end,
						exclusive,
					}),
					reverse,
					limit,
				}),
			)
			.await?;
		parse_list_response(response)
	}

	pub async fn kv_list_prefix(
		&self,
		actor_id: String,
		prefix: Vec<u8>,
		reverse: Option<bool>,
		limit: Option<u64>,
	) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvListRequest(protocol::KvListRequest {
					query: protocol::KvListQuery::KvListPrefixQuery(protocol::KvListPrefixQuery {
						key: prefix,
					}),
					reverse,
					limit,
				}),
			)
			.await?;
		parse_list_response(response)
	}

	pub async fn kv_put(
		&self,
		actor_id: String,
		entries: Vec<(Vec<u8>, Vec<u8>)>,
	) -> anyhow::Result<()> {
		let (keys, values): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvPutRequest(protocol::KvPutRequest { keys, values }),
			)
			.await?;
		match response {
			protocol::KvResponseData::KvPutResponse => Ok(()),
			protocol::KvResponseData::KvErrorResponse(e) => anyhow::bail!("{}", e.message),
			_ => anyhow::bail!("unexpected KV response type"),
		}
	}

	pub async fn kv_delete(&self, actor_id: String, keys: Vec<Vec<u8>>) -> anyhow::Result<()> {
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvDeleteRequest(protocol::KvDeleteRequest { keys }),
			)
			.await?;
		match response {
			protocol::KvResponseData::KvDeleteResponse => Ok(()),
			protocol::KvResponseData::KvErrorResponse(e) => anyhow::bail!("{}", e.message),
			_ => anyhow::bail!("unexpected KV response type"),
		}
	}

	pub async fn kv_delete_range(
		&self,
		actor_id: String,
		start: Vec<u8>,
		end: Vec<u8>,
	) -> anyhow::Result<()> {
		let response = self
			.send_kv_request(
				actor_id,
				protocol::KvRequestData::KvDeleteRangeRequest(protocol::KvDeleteRangeRequest {
					start,
					end,
				}),
			)
			.await?;
		match response {
			protocol::KvResponseData::KvDeleteResponse => Ok(()),
			protocol::KvResponseData::KvErrorResponse(e) => anyhow::bail!("{}", e.message),
			_ => anyhow::bail!("unexpected KV response type"),
		}
	}

	pub async fn kv_drop(&self, actor_id: String) -> anyhow::Result<()> {
		let response = self
			.send_kv_request(actor_id, protocol::KvRequestData::KvDropRequest)
			.await?;
		match response {
			protocol::KvResponseData::KvDropResponse => Ok(()),
			protocol::KvResponseData::KvErrorResponse(e) => anyhow::bail!("{}", e.message),
			_ => anyhow::bail!("unexpected KV response type"),
		}
	}

	pub async fn sqlite_get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> anyhow::Result<protocol::SqliteGetPagesResponse> {
		match self
			.send_sqlite_request(SqliteRequest::GetPages(request))
			.await?
		{
			SqliteResponse::GetPages(response) => Ok(response),
			_ => anyhow::bail!("unexpected sqlite get_pages response type"),
		}
	}

	pub async fn sqlite_commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> anyhow::Result<protocol::SqliteCommitResponse> {
		match self
			.send_sqlite_request(SqliteRequest::Commit(request))
			.await?
		{
			SqliteResponse::Commit(response) => Ok(response),
			_ => anyhow::bail!("unexpected sqlite commit response type"),
		}
	}

	pub async fn remote_sqlite_exec(
		&self,
		request: protocol::SqliteExecRequest,
	) -> anyhow::Result<protocol::SqliteExecResponse> {
		match self
			.send_remote_sqlite_request(RemoteSqliteRequest::Exec(request))
			.await?
		{
			RemoteSqliteResponse::Exec(response) => Ok(response),
			_ => anyhow::bail!("unexpected remote sqlite exec response type"),
		}
	}

	pub async fn remote_sqlite_execute(
		&self,
		request: protocol::SqliteExecuteRequest,
	) -> anyhow::Result<protocol::SqliteExecuteResponse> {
		match self
			.send_remote_sqlite_request(RemoteSqliteRequest::Execute(request))
			.await?
		{
			RemoteSqliteResponse::Execute(response) => Ok(response),
			_ => anyhow::bail!("unexpected remote sqlite execute response type"),
		}
	}

	pub fn restore_hibernating_requests(
		&self,
		actor_id: String,
		meta_entries: Vec<HibernatingWebSocketMetadata>,
	) {
		self.shared
			.pending_hibernation_restores
			.upsert_sync(actor_id, meta_entries);
	}

	pub(crate) fn take_pending_hibernation_restore(
		&self,
		actor_id: &str,
	) -> Option<Vec<HibernatingWebSocketMetadata>> {
		self.shared
			.pending_hibernation_restores
			.remove_sync(actor_id)
			.map(|(_, meta_entries)| meta_entries)
	}

	pub fn send_hibernatable_ws_message_ack(
		&self,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		client_message_index: u16,
	) {
		let _ = crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::HwsAck {
				gateway_id,
				request_id,
				envoy_message_index: client_message_index,
			},
		);
	}

	/// Inject a serverless start payload into the envoy.
	/// The payload is a u16 LE protocol version followed by a serialized ToEnvoy message.
	pub async fn start_serverless_actor(&self, payload: &[u8]) -> anyhow::Result<()> {
		tracing::debug!(
			envoy_key = %self.shared.envoy_key,
			payload_len = payload.len(),
			"received serverless start request"
		);
		let (message, _) = decode_serverless_actor_start_payload(payload)?;

		// Wait for envoy to be started before injecting
		self.started().await?;

		tracing::debug!(
			envoy_key = %self.shared.envoy_key,
			data = crate::stringify::stringify_to_envoy(&message),
			"received serverless start"
		);
		crate::envoy::send_to_envoy_tx(&self.shared, ToEnvoyMessage::ConnMessage { message })
			.map_err(|_| anyhow::anyhow!("envoy channel closed"))?;

		Ok(())
	}

	pub fn decode_serverless_actor_start(
		&self,
		payload: &[u8],
	) -> anyhow::Result<ServerlessActorStart> {
		let (_, actor_start) = decode_serverless_actor_start_payload(payload)?;
		Ok(actor_start)
	}
}

impl EnvoyStatusHandle {
	pub fn last_ping_at_ms(&self) -> Option<i64> {
		self.shared.upgrade().and_then(|shared| {
			let last = shared.last_ping_ts.load(Ordering::Acquire);
			if last == 0 {
				None
			} else {
				Some(last)
			}
		})
	}

	pub fn last_ping_age_ms(&self) -> Option<i64> {
		self.last_ping_at_ms()
			.map(|last| crate::time::now_millis().saturating_sub(last))
	}

	pub fn is_ping_healthy(&self) -> bool {
		self.last_ping_age_ms()
			.is_some_and(|age_ms| age_ms < EnvoyHandle::PING_HEALTHY_THRESHOLD_MS)
	}

	pub fn active_actor_count(&self) -> Option<usize> {
		self.shared
			.upgrade()
			.map(|shared| active_actor_count(&shared))
	}
}

fn active_actor_count(shared: &SharedContext) -> usize {
	let mut count = 0;
	shared.actors.iter_sync(|_, generations| {
		generations.iter_sync(|_, actor| {
			if !actor.handle.is_closed() {
				count += 1;
			}
			true
		});
		true
	});
	count
}

fn decode_serverless_actor_start_payload(
	payload: &[u8],
) -> anyhow::Result<(protocol::ToEnvoy, ServerlessActorStart)> {
	use vbare::OwnedVersionedData;

	if payload.len() < 2 {
		anyhow::bail!("serverless start payload too short");
	}

	let version = u16::from_le_bytes([payload[0], payload[1]]);
	if version != protocol::PROTOCOL_VERSION {
		anyhow::bail!(
			"serverless start payload does not match protocol version: {version} vs {}",
			protocol::PROTOCOL_VERSION
		);
	}

	let message = match crate::protocol::versioned::ToEnvoy::deserialize(&payload[2..], version) {
		Ok(message) => message,
		Err(err) if version == protocol::PROTOCOL_VERSION => {
			tracing::debug!(
				?err,
				"serverless start payload failed current-version decode, retrying as v1-compatible body"
			);
			crate::protocol::versioned::ToEnvoy::deserialize(
				&payload[2..],
				protocol::PROTOCOL_VERSION - 1,
			)?
		}
		Err(err) => return Err(err),
	};

	let protocol::ToEnvoy::ToEnvoyCommands(ref commands) = message else {
		anyhow::bail!("invalid serverless payload: expected ToEnvoyCommands");
	};
	if commands.len() != 1 {
		anyhow::bail!("invalid serverless payload: expected exactly 1 command");
	}
	if !matches!(commands[0].inner, protocol::Command::CommandStartActor(_)) {
		anyhow::bail!("invalid serverless payload: expected CommandStartActor");
	}

	let actor_start = ServerlessActorStart {
		actor_id: commands[0].checkpoint.actor_id.clone(),
		generation: commands[0].checkpoint.generation,
	};

	Ok((message, actor_start))
}

impl EnvoyHandle {
	async fn send_kv_request(
		&self,
		actor_id: String,
		data: protocol::KvRequestData,
	) -> anyhow::Result<protocol::KvResponseData> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::KvRequest {
				actor_id,
				data,
				response_tx: tx,
			},
		)
		.map_err(|_| anyhow::anyhow!("envoy channel closed"))?;
		rx.await
			.map_err(|_| anyhow::anyhow!("kv response channel closed"))?
	}

	async fn send_sqlite_request(&self, request: SqliteRequest) -> anyhow::Result<SqliteResponse> {
		let kind = request.kind();
		let total_start = crate::time::Instant::now();
		let submit_start = crate::time::Instant::now();
		let (tx, rx) = tokio::sync::oneshot::channel();
		crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::SqliteRequest {
				request,
				response_tx: tx,
			},
		)
		.map_err(|_| anyhow::anyhow!("envoy channel closed"))?;
		let submit_elapsed = submit_start.elapsed();
		METRICS
			.sqlite_request_submit_duration_seconds
			.with_label_values(&[kind])
			.observe(submit_elapsed.as_secs_f64());

		let wait_start = crate::time::Instant::now();
		let result = rx
			.await
			.map_err(|_| anyhow::anyhow!("sqlite response channel closed"))?;
		let wait_elapsed = wait_start.elapsed();
		METRICS
			.sqlite_request_wait_duration_seconds
			.with_label_values(&[kind])
			.observe(wait_elapsed.as_secs_f64());
		METRICS
			.sqlite_request_total_duration_seconds
			.with_label_values(&[kind])
			.observe(total_start.elapsed().as_secs_f64());
		result
	}

	async fn send_remote_sqlite_request(
		&self,
		request: RemoteSqliteRequest,
	) -> anyhow::Result<RemoteSqliteResponse> {
		let kind = request.kind();
		let total_start = crate::time::Instant::now();
		let submit_start = crate::time::Instant::now();
		let (tx, rx) = tokio::sync::oneshot::channel();
		crate::envoy::send_to_envoy_tx(
			&self.shared,
			ToEnvoyMessage::RemoteSqliteRequest {
				request,
				response_tx: tx,
			},
		)
		.map_err(|_| anyhow::anyhow!("envoy channel closed"))?;
		let submit_elapsed = submit_start.elapsed();
		METRICS
			.sqlite_request_submit_duration_seconds
			.with_label_values(&[kind])
			.observe(submit_elapsed.as_secs_f64());

		let wait_start = crate::time::Instant::now();
		let result = rx
			.await
			.map_err(|_| anyhow::anyhow!("remote sqlite response channel closed"))?;
		let wait_elapsed = wait_start.elapsed();
		METRICS
			.sqlite_request_wait_duration_seconds
			.with_label_values(&[kind])
			.observe(wait_elapsed.as_secs_f64());
		METRICS
			.sqlite_request_total_duration_seconds
			.with_label_values(&[kind])
			.observe(total_start.elapsed().as_secs_f64());
		result
	}
}

fn parse_list_response(
	response: protocol::KvResponseData,
) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
	match response {
		protocol::KvResponseData::KvListResponse(resp) => {
			Ok(resp.keys.into_iter().zip(resp.values).collect())
		}
		protocol::KvResponseData::KvErrorResponse(e) => anyhow::bail!("{}", e.message),
		_ => anyhow::bail!("unexpected KV response type"),
	}
}
