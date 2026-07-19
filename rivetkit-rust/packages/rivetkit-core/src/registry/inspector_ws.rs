use super::http::authorization_bearer_token_map;
use super::inspector::*;
use super::websocket::{closing_websocket_handler, websocket_inspector_token};
use super::*;
use std::future::Future;
use tracing::Instrument;

/// Inspector WebSockets originally did not require `protocol_version` on the
/// connection URL. Those clients embed a version header in every message, and
/// v5 was current when connection-level negotiation was introduced. Keep a
/// missing query parameter on that v5 framing so older deployed clients remain
/// compatible; current clients must send `protocol_version` once on the URL.
const LEGACY_INSPECTOR_VERSION: u16 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectorWireMode {
	Negotiated { version: u16 },
	LegacyEmbeddedV5,
}

impl InspectorWireMode {
	fn version(self) -> u16 {
		match self {
			Self::Negotiated { version } => version,
			Self::LegacyEmbeddedV5 => LEGACY_INSPECTOR_VERSION,
		}
	}

	fn supports_schedules(self) -> bool {
		self.version() >= 6
	}
}

fn inspector_wire_mode(request: &HttpRequest) -> Result<InspectorWireMode> {
	inspector_wire_mode_from_path(&request.path)
}

fn inspector_wire_mode_from_path(path: &str) -> Result<InspectorWireMode> {
	let url = Url::parse(&format!("http://inspector{path}"))
		.context("parse inspector websocket request url")?;
	let Some(value) = url
		.query_pairs()
		.find_map(|(name, value)| (name == "protocol_version").then_some(value.into_owned()))
	else {
		return Ok(InspectorWireMode::LegacyEmbeddedV5);
	};
	let version = value
		.parse::<u16>()
		.context("inspector protocol_version must be an unsigned integer")?;
	if version == 0 || version > inspector_protocol::PROTOCOL_VERSION {
		anyhow::bail!(
			"unsupported inspector protocol version {version}; supported range is 1..={}",
			inspector_protocol::PROTOCOL_VERSION
		);
	}
	Ok(InspectorWireMode::Negotiated { version })
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::sync::Notify;

	#[test]
	fn parses_inspector_protocol_query_parameter() {
		assert_eq!(
			inspector_wire_mode_from_path("/inspector/connect").unwrap(),
			InspectorWireMode::LegacyEmbeddedV5
		);
		assert_eq!(
			inspector_wire_mode_from_path("/inspector/connect?protocol_version=6").unwrap(),
			InspectorWireMode::Negotiated { version: 6 }
		);
		for path in [
			"/inspector/connect?protocol_version=",
			"/inspector/connect?protocol_version=wat",
			"/inspector/connect?protocol_version=0",
			"/inspector/connect?protocol_version=7",
		] {
			assert!(inspector_wire_mode_from_path(path).is_err(), "{path}");
		}
	}

	#[tokio::test]
	async fn delayed_inspector_queries_publish_in_signal_order() {
		let (sender, receiver) = mpsc::unbounded_channel();
		let release_first = Arc::new(Notify::new());
		let published = Arc::new(TokioMutex::new(Vec::new()));
		let task = tokio::spawn(process_inspector_signals_in_order(receiver, {
			let release_first = release_first.clone();
			let published = published.clone();
			move |signal| {
				let release_first = release_first.clone();
				let published = published.clone();
				async move {
					if signal == InspectorSignal::SchedulesUpdated {
						release_first.notified().await;
					}
					published.lock().await.push(signal);
					true
				}
			}
		}));

		sender.send(InspectorSignal::SchedulesUpdated).unwrap();
		sender.send(InspectorSignal::StateUpdated).unwrap();
		tokio::task::yield_now().await;
		assert!(published.lock().await.is_empty());

		release_first.notify_one();
		drop(sender);
		task.await.unwrap();
		assert_eq!(
			*published.lock().await,
			vec![
				InspectorSignal::SchedulesUpdated,
				InspectorSignal::StateUpdated
			]
		);
	}
}

fn send_inspector_message(
	sender: &WebSocketSender,
	message: &InspectorServerMessage,
	wire_mode: InspectorWireMode,
) -> Result<()> {
	let payload = match wire_mode {
		InspectorWireMode::Negotiated { version } => {
			inspector_protocol::encode_server_payload(message, version)?
		}
		InspectorWireMode::LegacyEmbeddedV5 => {
			inspector_protocol::encode_server_message_embedded(message, LEGACY_INSPECTOR_VERSION)?
		}
	};
	sender.send(payload, true);
	Ok(())
}

async fn process_inspector_signals_in_order<F, Fut>(
	mut receiver: mpsc::UnboundedReceiver<InspectorSignal>,
	mut process: F,
) where
	F: FnMut(InspectorSignal) -> Fut,
	Fut: Future<Output = bool>,
{
	while let Some(signal) = receiver.recv().await {
		if !process(signal).await {
			break;
		}
	}
}

/// Aborts the wrapped task on drop. Ensures the overlay task cannot outlive
/// the websocket handler even if `on_close` never fires (for example when the
/// handler is dropped due to actor teardown rather than a clean close frame).
struct AbortOnDropTask(JoinHandle<()>);

impl Drop for AbortOnDropTask {
	fn drop(&mut self) {
		self.0.abort();
	}
}

impl RegistryDispatcher {
	pub(super) async fn handle_inspector_websocket(
		self: &Arc<Self>,
		actor_id: &str,
		instance: Arc<ActorTaskHandle>,
		request: &HttpRequest,
		headers: &HashMap<String, String>,
	) -> Result<WebSocketHandler> {
		let wire_mode = match inspector_wire_mode(request) {
			Ok(wire_mode) => wire_mode,
			Err(error) => {
				tracing::warn!(actor_id, ?error, "rejecting invalid inspector protocol version");
				return Ok(closing_websocket_handler(
					1002,
					"inspector.invalid_protocol_version",
				));
			}
		};
		tracing::info!(actor_id, "inspector WS: handler invoked, verifying auth");
		if InspectorAuth::new()
			.verify(
				&instance.ctx,
				websocket_inspector_token(headers)
					.or_else(|| authorization_bearer_token_map(headers)),
			)
			.await
			.is_err()
		{
			tracing::warn!(
				actor_id,
				"rejecting inspector websocket without a valid token"
			);
			return Ok(closing_websocket_handler(1008, "inspector.unauthorized"));
		}
		tracing::info!(actor_id, "inspector WS: auth passed, building handler");

		let dispatcher = self.clone();
		// Forced-sync: inspector websocket slots are filled/cleared inside
		// synchronous callback setup/teardown and moved out before awaiting.
		let subscription_slot = Arc::new(Mutex::new(None::<InspectorSubscription>));
		let overlay_task_slot = Arc::new(Mutex::new(None::<AbortOnDropTask>));
		let publisher_task_slot = Arc::new(Mutex::new(None::<AbortOnDropTask>));
		let attach_guard_slot = Arc::new(Mutex::new(None::<InspectorAttachGuard>));
		let on_open_instance = instance.clone();
		let on_open_dispatcher = dispatcher.clone();
		let on_open_slot = subscription_slot.clone();
		let on_open_overlay_slot = overlay_task_slot.clone();
		let on_open_publisher_slot = publisher_task_slot.clone();
		let on_open_attach_guard_slot = attach_guard_slot.clone();
		let on_message_instance = instance.clone();
		let on_message_dispatcher = dispatcher.clone();
		let on_message_wire_mode = wire_mode;

		let on_message_actor_id = actor_id.to_owned();
		let on_close_actor_id = actor_id.to_owned();
		let on_open_actor_id = actor_id.to_owned();
		Ok(WebSocketHandler {
			on_message: Box::new(move |message: WebSocketMessage| {
				let dispatcher = on_message_dispatcher.clone();
				let instance = on_message_instance.clone();
				let actor_id = on_message_actor_id.clone();
				Box::pin(async move {
					tracing::info!(
						actor_id = %actor_id,
						bytes = message.data.len(),
						"inspector WS: on_message fired"
					);
					dispatcher
						.handle_inspector_websocket_message(
							&instance,
							&message.sender,
							&message.data,
							on_message_wire_mode,
						)
						.await;
				})
			}),
			on_close: Box::new(move |code, reason| {
				let slot = subscription_slot.clone();
				let overlay_slot = overlay_task_slot.clone();
				let publisher_slot = publisher_task_slot.clone();
				let attach_slot = attach_guard_slot.clone();
				let actor_id = on_close_actor_id.clone();
				Box::pin(async move {
					tracing::info!(
						actor_id = %actor_id,
						?code,
						?reason,
						"inspector WS: on_close fired"
					);
					let mut guard = slot.lock();
					guard.take();
					let mut overlay_guard = overlay_slot.lock();
					overlay_guard.take();
					let mut publisher_guard = publisher_slot.lock();
					publisher_guard.take();
					let mut attach_guard = attach_slot.lock();
					attach_guard.take();
				})
			}),
			on_open: Some(Box::new(move |open_sender| {
				let actor_id = on_open_actor_id.clone();
				Box::pin(async move {
					tracing::info!(actor_id = %actor_id, "inspector WS: on_open fired, building init");
					match on_open_dispatcher
						.inspector_init_message(&on_open_instance)
						.await
					{
						Ok(message) => {
							if let Err(error) =
								send_inspector_message(&open_sender, &message, wire_mode)
							{
								tracing::error!(?error, "failed to send inspector init message");
								open_sender
									.close(Some(1011), Some("inspector.init_error".to_owned()));
								return;
							}
							tracing::info!(actor_id = %actor_id, "inspector WS: init message sent");
						}
						Err(error) => {
							tracing::error!(?error, "failed to build inspector init message");
							open_sender.close(Some(1011), Some("inspector.init_error".to_owned()));
							return;
						}
					}

					let Some(attach_guard) = on_open_instance.ctx.inspector_attach() else {
						tracing::error!("inspector runtime missing during websocket attach");
						open_sender.close(Some(1011), Some("inspector.runtime_missing".to_owned()));
						return;
					};
					let Some(mut overlay_rx) = on_open_instance.ctx.subscribe_inspector() else {
						tracing::error!(
							"inspector overlay runtime missing during websocket attach"
						);
						open_sender.close(Some(1011), Some("inspector.runtime_missing".to_owned()));
						return;
					};
					{
						let mut guard = on_open_attach_guard_slot.lock();
						*guard = Some(attach_guard);
					}
					let overlay_sender = open_sender.clone();
					let overlay_actor_id = on_open_instance.ctx.actor_id().to_owned();
					let overlay_task = RuntimeSpawner::spawn(
						async move {
							loop {
								match overlay_rx.recv().await {
									Ok(payload) => match decode_inspector_overlay_state(&payload) {
										Ok(Some(state)) => {
											if let Err(error) = send_inspector_message(
												&overlay_sender,
												&InspectorServerMessage::StateUpdated(
													inspector_protocol::StateUpdated { state },
												),
												wire_mode,
											) {
												tracing::error!(
													?error,
													"failed to push inspector overlay update"
												);
												break;
											}
										}
										Ok(None) => {}
										Err(error) => {
											tracing::error!(
												?error,
												"failed to decode inspector overlay update"
											);
										}
									},
									Err(broadcast::error::RecvError::Lagged(skipped)) => {
										tracing::warn!(
											skipped,
											"inspector overlay subscriber lagged; waiting for next sync"
										);
									}
									Err(broadcast::error::RecvError::Closed) => break,
								}
							}
						}
						.instrument(tracing::info_span!(
							"inspector_ws",
							actor_id = %overlay_actor_id,
						)),
					);
					let mut overlay_guard = on_open_overlay_slot.lock();
					*overlay_guard = Some(AbortOnDropTask(overlay_task));

					// Inspector signals can arrive faster than the underlying snapshot
					// queries complete. Process them through one publisher so an older,
					// slower query can never overwrite a newer schedule snapshot.
					let (signal_tx, signal_rx) = mpsc::unbounded_channel();
					let publisher_dispatcher = on_open_dispatcher.clone();
					let publisher_instance = on_open_instance.clone();
					let publisher_sender = open_sender.clone();
					let publisher_actor_id = on_open_instance.ctx.actor_id().to_owned();
					let publisher_task = RuntimeSpawner::spawn(
						async move {
							process_inspector_signals_in_order(signal_rx, move |signal| {
								let dispatcher = publisher_dispatcher.clone();
								let instance = publisher_instance.clone();
								let sender = publisher_sender.clone();
								async move {
									if signal == InspectorSignal::SchedulesUpdated
										&& !wire_mode.supports_schedules()
									{
										return true;
									}
									match dispatcher
										.inspector_push_message_for_signal(&instance, signal)
										.await
									{
										Ok(Some(message)) => {
											if let Err(error) =
												send_inspector_message(&sender, &message, wire_mode)
											{
												tracing::error!(
													?error,
													?signal,
													"failed to push inspector websocket update"
												);
												return false;
											}
										}
										Ok(None) => {}
										Err(error) => tracing::error!(
											?error,
											?signal,
											"failed to build inspector websocket update"
										),
									}
									true
								}
							})
							.await;
						}
						.instrument(tracing::info_span!(
							"inspector_ws",
							actor_id = %publisher_actor_id,
						)),
					);
					{
						let mut publisher_guard = on_open_publisher_slot.lock();
						*publisher_guard = Some(AbortOnDropTask(publisher_task));
					}

					let subscription =
						on_open_instance
							.inspector
							.subscribe(Arc::new(move |signal| {
								// Keep forwarding persisted StateUpdated signals here.
								// Overlay broadcasts still carry unsaved in-memory state,
								// but explicit inspector PATCH saves only emit the
								// InspectorSignal path after the write completes.
								let _ = signal_tx.send(signal);
							}));
					let mut guard = on_open_slot.lock();
					*guard = Some(subscription);
				})
			})),
		})
	}

	async fn handle_inspector_websocket_message(
		&self,
		instance: &ActorTaskHandle,
		sender: &WebSocketSender,
		payload: &[u8],
		wire_mode: InspectorWireMode,
	) {
		let decoded = match wire_mode {
			InspectorWireMode::Negotiated { version } => {
				inspector_protocol::decode_client_payload(payload, version)
			}
			InspectorWireMode::LegacyEmbeddedV5 => inspector_protocol::decode_client_message(payload),
		};
		let response = match decoded {
			Ok(message) => {
				tracing::info!(
					actor_id = %instance.ctx.actor_id(),
					message_kind = client_message_kind(&message),
					payload_len = payload.len(),
					"inspector WS: decoded client message"
				);
				match self
					.process_inspector_websocket_message(instance, message)
					.await
				{
					Ok(response) => {
						tracing::info!(
							actor_id = %instance.ctx.actor_id(),
							response_kind = response.as_ref().map(server_message_kind).unwrap_or("None"),
							"inspector WS: processed client message"
						);
						response
					}
					Err(error) => {
						tracing::warn!(
							actor_id = %instance.ctx.actor_id(),
							?error,
							"inspector WS: process_inspector_websocket_message returned error"
						);
						Some(InspectorServerMessage::Error(
							inspector_protocol::ErrorMessage {
								message: error.to_string(),
							},
						))
					}
				}
			}
			Err(error) => {
				tracing::warn!(
					actor_id = %instance.ctx.actor_id(),
					payload_len = payload.len(),
					?error,
					"inspector WS: failed to decode client message"
				);
				Some(InspectorServerMessage::Error(
					inspector_protocol::ErrorMessage {
						message: error.to_string(),
					},
				))
			}
		};

		if let Some(response) = response {
			match send_inspector_message(sender, &response, wire_mode) {
				Ok(()) => tracing::debug!(
					actor_id = %instance.ctx.actor_id(),
					response_kind = server_message_kind(&response),
					"inspector WS: sent response"
				),
				Err(error) => tracing::error!(
					?error,
					response_kind = server_message_kind(&response),
					"failed to send inspector websocket response"
				),
			}
		}
	}

	async fn process_inspector_websocket_message(
		&self,
		instance: &ActorTaskHandle,
		message: inspector_protocol::ClientMessage,
	) -> Result<Option<InspectorServerMessage>> {
		match message {
			inspector_protocol::ClientMessage::PatchStateRequest(request) => {
				let state = request.state;
				instance
					.ctx
					.save_state(vec![StateDelta::ActorState(state.clone())])
					.await
					.context("save inspector websocket state patch")?;
				Ok(Some(InspectorServerMessage::StateUpdated(
					inspector_protocol::StateUpdated { state },
				)))
			}
			inspector_protocol::ClientMessage::StateRequest(request) => {
				Ok(Some(InspectorServerMessage::StateResponse(
					self.inspector_state_response(instance, request.id),
				)))
			}
			inspector_protocol::ClientMessage::ConnectionsRequest(request) => {
				Ok(Some(InspectorServerMessage::ConnectionsResponse(
					inspector_protocol::ConnectionsResponse {
						rid: request.id,
						connections: inspector_wire_connections(&instance.ctx),
					},
				)))
			}
			inspector_protocol::ClientMessage::ActionRequest(request) => {
				tracing::info!(
					rid = ?request.id,
					action_name = %request.name,
					args_len = request.args.len(),
					"inspector WS: ActionRequest received"
				);
				let output = self
					.execute_inspector_action_bytes(instance, &request.name, request.args)
					.await
					.map_err(ActionDispatchError::into_anyhow)?;
				tracing::info!(
					rid = ?request.id,
					action_name = %request.name,
					output_len = output.len(),
					"inspector WS: ActionResponse ready to send"
				);
				Ok(Some(InspectorServerMessage::ActionResponse(
					inspector_protocol::ActionResponse {
						rid: request.id,
						output,
					},
				)))
			}
			inspector_protocol::ClientMessage::RpcsListRequest(request) => Ok(Some(
				InspectorServerMessage::RpcsListResponse(inspector_protocol::RpcsListResponse {
					rid: request.id,
					rpcs: inspector_rpcs(instance),
				}),
			)),
			inspector_protocol::ClientMessage::TraceQueryRequest(request) => {
				Ok(Some(InspectorServerMessage::TraceQueryResponse(
					inspector_protocol::TraceQueryResponse {
						rid: request.id,
						payload: Vec::new(),
					},
				)))
			}
			inspector_protocol::ClientMessage::QueueRequest(request) => {
				let status = self
					.inspector_queue_status(
						instance,
						inspector_protocol::clamp_queue_limit(request.limit),
					)
					.await?;
				Ok(Some(InspectorServerMessage::QueueResponse(
					inspector_protocol::QueueResponse {
						rid: request.id,
						status,
					},
				)))
			}
			inspector_protocol::ClientMessage::WorkflowHistoryRequest(request) => {
				let (workflow_supported, history) =
					self.inspector_workflow_history_bytes(instance).await?;
				Ok(Some(InspectorServerMessage::WorkflowHistoryResponse(
					inspector_protocol::WorkflowHistoryResponse {
						rid: request.id,
						history,
						is_workflow_enabled: workflow_supported,
					},
				)))
			}
			inspector_protocol::ClientMessage::WorkflowReplayRequest(request) => {
				let (workflow_supported, history) = self
					.inspector_workflow_replay_bytes(instance, request.entry_id)
					.await?;
				Ok(Some(InspectorServerMessage::WorkflowReplayResponse(
					inspector_protocol::WorkflowReplayResponse {
						rid: request.id,
						history,
						is_workflow_enabled: workflow_supported,
					},
				)))
			}
			inspector_protocol::ClientMessage::DatabaseSchemaRequest(request) => {
				let schema = self.inspector_database_schema_bytes(&instance.ctx).await?;
				Ok(Some(InspectorServerMessage::DatabaseSchemaResponse(
					inspector_protocol::DatabaseSchemaResponse {
						rid: request.id,
						schema,
					},
				)))
			}
			inspector_protocol::ClientMessage::DatabaseTableRowsRequest(request) => {
				let result = self
					.inspector_database_rows_bytes(
						&instance.ctx,
						&request.table,
						request.limit.0.min(u64::from(u32::MAX)) as u32,
						request.offset.0.min(u64::from(u32::MAX)) as u32,
					)
					.await?;
				Ok(Some(InspectorServerMessage::DatabaseTableRowsResponse(
					inspector_protocol::DatabaseTableRowsResponse {
						rid: request.id,
						result,
					},
				)))
			}
			inspector_protocol::ClientMessage::SchedulesRequest(request) => {
				Ok(Some(InspectorServerMessage::SchedulesResponse(
					inspector_protocol::SchedulesResponse {
						rid: request.id,
						schedules: self.inspector_schedules(instance).await?,
					},
				)))
			}
			inspector_protocol::ClientMessage::ScheduleHistoryRequest(request) => {
				let limit = request.limit.0.clamp(1, 1_000) as i64;
				let history = instance
					.ctx
					.cron_history(&request.schedule_id, Some(limit))
					.await?
					.into_iter()
					.map(inspector_wire_schedule_fire)
					.collect();
				Ok(Some(InspectorServerMessage::ScheduleHistoryResponse(
					inspector_protocol::ScheduleHistoryResponse {
						rid: request.id,
						schedule_id: request.schedule_id,
						history,
					},
				)))
			}
			inspector_protocol::ClientMessage::ScheduleDeleteRequest(request) => {
				let deleted = match request.kind.as_str() {
					"at" => instance.ctx.cancel_schedule(&request.schedule_id).await?,
					"cron" | "every" => instance.ctx.cron_delete(&request.schedule_id).await?,
					kind => anyhow::bail!("invalid inspector schedule kind {kind:?}"),
				};
				Ok(Some(InspectorServerMessage::ScheduleDeleteResponse(
					inspector_protocol::ScheduleDeleteResponse {
						rid: request.id,
						schedule_id: request.schedule_id,
						deleted,
					},
				)))
			}
		}
	}

	async fn inspector_init_message(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<InspectorServerMessage> {
		let (workflow_supported, workflow_history) =
			self.inspector_workflow_history_bytes(instance).await?;
		let queue_size = self.inspector_current_queue_size(instance).await?;
		let schedules = self.inspector_schedules(instance).await?;
		let is_state_enabled = instance.ctx.has_state();
		Ok(InspectorServerMessage::Init(
			inspector_protocol::InitMessage {
				connections: inspector_wire_connections(&instance.ctx),
				state: inspector_state_payload(&instance.ctx, is_state_enabled),
				is_state_enabled,
				rpcs: inspector_rpcs(instance),
				is_database_enabled: instance.ctx.sql().is_enabled(),
				queue_size: serde_bare::Uint(queue_size),
				workflow_history,
				is_workflow_enabled: workflow_supported,
				tab_config: inspector_wire_tab_config(instance.factory.config()),
				schedules,
			},
		))
	}

	fn inspector_state_response(
		&self,
		instance: &ActorTaskHandle,
		rid: serde_bare::Uint,
	) -> inspector_protocol::StateResponse {
		let is_state_enabled = instance.ctx.has_state();
		inspector_protocol::StateResponse {
			rid,
			state: inspector_state_payload(&instance.ctx, is_state_enabled),
			is_state_enabled,
		}
	}

	async fn inspector_queue_status(
		&self,
		instance: &ActorTaskHandle,
		limit: u32,
	) -> Result<inspector_protocol::QueueStatus> {
		let messages = instance
			.ctx
			.queue()
			.inspect_messages()
			.await
			.context("list inspector queue messages")?;
		let queue_size = messages.len().try_into().unwrap_or(u32::MAX);
		let truncated = messages.len() > limit as usize;
		let messages = messages
			.into_iter()
			.take(limit as usize)
			.map(|message| inspector_protocol::QueueMessageSummary {
				id: serde_bare::Uint(message.id),
				name: message.name,
				created_at_ms: serde_bare::Uint(
					u64::try_from(message.created_at).unwrap_or_default(),
				),
			})
			.collect();

		Ok(inspector_protocol::QueueStatus {
			size: serde_bare::Uint(u64::from(queue_size)),
			max_size: serde_bare::Uint(u64::from(instance.ctx.queue().max_size())),
			messages,
			truncated,
		})
	}

	async fn inspector_current_queue_size(&self, instance: &ActorTaskHandle) -> Result<u64> {
		Ok(instance
			.ctx
			.queue()
			.inspect_messages()
			.await
			.context("list inspector queue messages for queue size")?
			.len()
			.try_into()
			.unwrap_or(u64::MAX))
	}

	async fn inspector_schedules(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<Vec<inspector_protocol::Schedule>> {
		let mut schedules = instance
			.ctx
			.list_scheduled_events()
			.await?
			.into_iter()
			.map(|schedule| inspector_protocol::Schedule {
				id: schedule.id,
				name: None,
				kind: "at".to_owned(),
				action: schedule.action,
				args: schedule.args,
				next_run_at: timestamp_to_uint(schedule.run_at),
				last_run_at: None,
				expression: None,
				timezone: None,
				interval_ms: None,
				max_history: None,
			})
			.collect::<Vec<_>>();
		schedules.extend(
			instance
				.ctx
				.cron_list()
				.await?
				.into_iter()
				.map(|schedule| inspector_protocol::Schedule {
					id: schedule.name.clone(),
					name: Some(schedule.name),
					kind: schedule.kind.as_str().to_owned(),
					action: schedule.action,
					args: schedule.args,
					next_run_at: timestamp_to_uint(schedule.next_run_at),
					last_run_at: schedule.last_run_at.map(timestamp_to_uint),
					expression: schedule.expression,
					timezone: schedule.timezone,
					interval_ms: schedule.interval_ms.map(timestamp_to_uint),
					max_history: Some(timestamp_to_uint(schedule.max_history)),
				}),
		);
		schedules.sort_by(|left, right| {
			left.next_run_at
				.cmp(&right.next_run_at)
				.then_with(|| left.id.cmp(&right.id))
		});
		Ok(schedules)
	}

	async fn inspector_push_message_for_signal(
		&self,
		instance: &ActorTaskHandle,
		signal: InspectorSignal,
	) -> Result<Option<InspectorServerMessage>> {
		match signal {
			InspectorSignal::StateUpdated => Ok(Some(InspectorServerMessage::StateUpdated(
				inspector_protocol::StateUpdated {
					state: instance.ctx.state(),
				},
			))),
			InspectorSignal::ConnectionsUpdated => {
				Ok(Some(InspectorServerMessage::ConnectionsUpdated(
					inspector_protocol::ConnectionsUpdated {
						connections: inspector_wire_connections(&instance.ctx),
					},
				)))
			}
			InspectorSignal::QueueUpdated => Ok(Some(InspectorServerMessage::QueueUpdated(
				inspector_protocol::QueueUpdated {
					queue_size: serde_bare::Uint(
						self.inspector_current_queue_size(instance).await?,
					),
				},
			))),
			InspectorSignal::WorkflowHistoryUpdated => {
				let (_, history) = self.inspector_workflow_history_bytes(instance).await?;
				Ok(history.map(|history| {
					InspectorServerMessage::WorkflowHistoryUpdated(
						inspector_protocol::WorkflowHistoryUpdated { history },
					)
				}))
			}
			InspectorSignal::SchedulesUpdated => Ok(Some(
				InspectorServerMessage::SchedulesUpdated(inspector_protocol::SchedulesUpdated {
					schedules: self.inspector_schedules(instance).await?,
				}),
			)),
		}
	}
}

/// Returns the actor state bytes for inspector wire payloads, or `None` when
/// state is disabled or has not been initialized yet. Sending `Some(empty)`
/// would cause the inspector frontend to attempt a CBOR decode of zero bytes
/// and fail with "Unexpected end of CBOR data".
fn inspector_state_payload(ctx: &ActorContext, is_state_enabled: bool) -> Option<Vec<u8>> {
	if !is_state_enabled {
		return None;
	}
	let state = ctx.state();
	if state.is_empty() { None } else { Some(state) }
}

fn client_message_kind(message: &inspector_protocol::ClientMessage) -> &'static str {
	use inspector_protocol::ClientMessage as C;
	match message {
		C::PatchStateRequest(_) => "PatchStateRequest",
		C::StateRequest(_) => "StateRequest",
		C::ConnectionsRequest(_) => "ConnectionsRequest",
		C::ActionRequest(_) => "ActionRequest",
		C::RpcsListRequest(_) => "RpcsListRequest",
		C::TraceQueryRequest(_) => "TraceQueryRequest",
		C::QueueRequest(_) => "QueueRequest",
		C::WorkflowHistoryRequest(_) => "WorkflowHistoryRequest",
		C::WorkflowReplayRequest(_) => "WorkflowReplayRequest",
		C::DatabaseSchemaRequest(_) => "DatabaseSchemaRequest",
		C::DatabaseTableRowsRequest(_) => "DatabaseTableRowsRequest",
		C::SchedulesRequest(_) => "SchedulesRequest",
		C::ScheduleHistoryRequest(_) => "ScheduleHistoryRequest",
		C::ScheduleDeleteRequest(_) => "ScheduleDeleteRequest",
	}
}

fn server_message_kind(message: &InspectorServerMessage) -> &'static str {
	match message {
		InspectorServerMessage::Init(_) => "Init",
		InspectorServerMessage::StateResponse(_) => "StateResponse",
		InspectorServerMessage::StateUpdated(_) => "StateUpdated",
		InspectorServerMessage::ConnectionsResponse(_) => "ConnectionsResponse",
		InspectorServerMessage::ConnectionsUpdated(_) => "ConnectionsUpdated",
		InspectorServerMessage::ActionResponse(_) => "ActionResponse",
		InspectorServerMessage::RpcsListResponse(_) => "RpcsListResponse",
		InspectorServerMessage::TraceQueryResponse(_) => "TraceQueryResponse",
		InspectorServerMessage::QueueResponse(_) => "QueueResponse",
		InspectorServerMessage::QueueUpdated(_) => "QueueUpdated",
		InspectorServerMessage::WorkflowHistoryResponse(_) => "WorkflowHistoryResponse",
		InspectorServerMessage::WorkflowHistoryUpdated(_) => "WorkflowHistoryUpdated",
		InspectorServerMessage::WorkflowReplayResponse(_) => "WorkflowReplayResponse",
		InspectorServerMessage::DatabaseSchemaResponse(_) => "DatabaseSchemaResponse",
		InspectorServerMessage::DatabaseTableRowsResponse(_) => "DatabaseTableRowsResponse",
		InspectorServerMessage::SchedulesResponse(_) => "SchedulesResponse",
		InspectorServerMessage::SchedulesUpdated(_) => "SchedulesUpdated",
		InspectorServerMessage::ScheduleHistoryResponse(_) => "ScheduleHistoryResponse",
		InspectorServerMessage::ScheduleDeleteResponse(_) => "ScheduleDeleteResponse",
		InspectorServerMessage::Error(_) => "Error",
	}
}

fn timestamp_to_uint(value: i64) -> serde_bare::Uint {
	serde_bare::Uint(value.max(0) as u64)
}

fn inspector_wire_schedule_fire(
	fire: crate::actor::schedule::CronFire,
) -> inspector_protocol::ScheduleFire {
	inspector_protocol::ScheduleFire {
		action: fire.action,
		scheduled_at: timestamp_to_uint(fire.scheduled_at),
		fired_at: timestamp_to_uint(fire.fired_at),
		finished_at: fire.finished_at.map(timestamp_to_uint),
		result: fire.result,
		error: fire.error.map(|error| inspector_protocol::ScheduleError {
			group: error.group,
			code: error.code,
			message: error.message,
			metadata: error
				.metadata
				.and_then(|metadata| encode_json_as_cbor(&metadata).ok()),
		}),
	}
}
