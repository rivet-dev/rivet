use super::actor_connect::send_inspector_message;
use super::http::authorization_bearer_token_map;
use super::inspector::*;
use super::websocket::{closing_websocket_handler, websocket_inspector_token};
use super::*;
use tracing::Instrument;

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
		_request: &HttpRequest,
		headers: &HashMap<String, String>,
	) -> Result<WebSocketHandler> {
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
		let attach_guard_slot = Arc::new(Mutex::new(None::<InspectorAttachGuard>));
		let on_open_instance = instance.clone();
		let on_open_dispatcher = dispatcher.clone();
		let on_open_slot = subscription_slot.clone();
		let on_open_overlay_slot = overlay_task_slot.clone();
		let on_open_attach_guard_slot = attach_guard_slot.clone();
		let on_message_instance = instance.clone();
		let on_message_dispatcher = dispatcher.clone();

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
						)
						.await;
				})
			}),
			on_close: Box::new(move |code, reason| {
				let slot = subscription_slot.clone();
				let overlay_slot = overlay_task_slot.clone();
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
							if let Err(error) = send_inspector_message(&open_sender, &message) {
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
					let overlay_task = tokio::spawn(
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

					let listener_dispatcher = on_open_dispatcher.clone();
					let listener_instance = on_open_instance.clone();
					let listener_sender = open_sender.clone();
					let subscription =
						on_open_instance
							.inspector
							.subscribe(Arc::new(move |signal| {
								// Keep forwarding persisted StateUpdated signals here.
								// Overlay broadcasts still carry unsaved in-memory state,
								// but explicit inspector PATCH saves only emit the
								// InspectorSignal path after the write completes.
								let dispatcher = listener_dispatcher.clone();
								let instance = listener_instance.clone();
								let sender = listener_sender.clone();
								let actor_id = instance.ctx.actor_id().to_owned();
								tokio::spawn(
									async move {
										match dispatcher
											.inspector_push_message_for_signal(&instance, signal)
											.await
										{
											Ok(Some(message)) => {
												if let Err(error) =
													send_inspector_message(&sender, &message)
												{
													tracing::error!(
														?error,
														?signal,
														"failed to push inspector websocket update"
													);
												}
											}
											Ok(None) => {}
											Err(error) => {
												tracing::error!(
													?error,
													?signal,
													"failed to build inspector websocket update"
												);
											}
										}
									}
									.instrument(tracing::info_span!(
										"inspector_ws",
										actor_id = %actor_id,
									)),
								);
							}));
					let mut guard = on_open_slot.lock();
					*guard = Some(subscription);
				})
			})),
		})
	}

	pub(super) async fn handle_inspector_websocket_message(
		&self,
		instance: &ActorTaskHandle,
		sender: &WebSocketSender,
		payload: &[u8],
	) {
		let response = match inspector_protocol::decode_client_message(payload) {
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
			match send_inspector_message(sender, &response) {
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
				instance
					.ctx
					.save_state(vec![StateDelta::ActorState(request.state)])
					.await
					.context("save inspector websocket state patch")?;
				Ok(None)
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
		}
	}

	async fn inspector_init_message(
		&self,
		instance: &ActorTaskHandle,
	) -> Result<InspectorServerMessage> {
		let (workflow_supported, workflow_history) =
			self.inspector_workflow_history_bytes(instance).await?;
		let queue_size = self.inspector_current_queue_size(instance).await?;
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
		InspectorServerMessage::Error(_) => "Error",
	}
}
