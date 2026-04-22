use super::dispatch::*;
use super::inspector::*;
use super::*;
use crate::error::ProtocolError;
use ::http;

impl RegistryDispatcher {
	pub(super) async fn handle_fetch(
		&self,
		actor_id: &str,
		request: HttpRequest,
	) -> Result<HttpResponse> {
		let instance = self.active_actor(actor_id).await?;
		if request.path == "/metrics" {
			return self.handle_metrics_fetch(&instance, &request);
		}
		let request = build_http_request(request).await?;
		if let Some(response) = self.handle_inspector_fetch(&instance, &request).await? {
			return Ok(response);
		}

		instance.ctx.cancel_sleep_timer();

		let rearm_sleep_after_request = |ctx: ActorContext| {
			let sleep_ctx = ctx.clone();
			ctx.wait_until(async move {
				sleep_ctx.wait_for_http_requests_idle().await;
				sleep_ctx.reset_sleep_timer();
			});
		};

		if let Some(route) = framework_http_route(request.uri().path())? {
			let response = self.handle_framework_fetch(&instance, request, route).await;
			rearm_sleep_after_request(instance.ctx.clone());
			return response;
		}

		let (reply_tx, reply_rx) = oneshot::channel();
		try_send_dispatch_command(
			&instance.dispatch,
			instance.factory.config().dispatch_command_inbox_capacity,
			"dispatch_http",
			DispatchCommand::Http {
				request,
				reply: reply_tx,
			},
			Some(instance.ctx.metrics()),
		)
		.context("send actor task HTTP dispatch command")?;

		match reply_rx
			.await
			.context("receive actor task HTTP dispatch reply")?
		{
			Ok(response) => {
				rearm_sleep_after_request(instance.ctx.clone());
				build_envoy_response(response)
			}
			Err(error) => {
				tracing::error!(actor_id, ?error, "actor request callback failed");
				rearm_sleep_after_request(instance.ctx.clone());
				Ok(inspector_anyhow_response(error))
			}
		}
	}

	async fn handle_framework_fetch(
		&self,
		instance: &ActorTaskHandle,
		request: Request,
		route: FrameworkHttpRoute,
	) -> Result<HttpResponse> {
		match route {
			FrameworkHttpRoute::Action(name) => {
				self.handle_action_fetch(instance, request, name).await
			}
			FrameworkHttpRoute::Queue(name) => {
				self.handle_queue_fetch(instance, request, name).await
			}
		}
	}

	async fn handle_action_fetch(
		&self,
		instance: &ActorTaskHandle,
		request: Request,
		action_name: String,
	) -> Result<HttpResponse> {
		let encoding = request_encoding(request.headers());
		if request.method() != http::Method::POST {
			return message_boundary_error_response(
				encoding,
				framework_error_status("actor", "method_not_allowed"),
				MethodNotAllowed {
					method: request.method().to_string(),
					path: request.uri().path().to_owned(),
				}
				.build(),
			);
		}

		let config = instance.factory.config();
		if request.body().len() > config.max_incoming_message_size as usize {
			return message_boundary_error_response(
				encoding,
				StatusCode::BAD_REQUEST,
				IncomingMessageTooLong.build(),
			);
		}

		let args = match decode_http_action_args(encoding, request.body()) {
			Ok(args) => args,
			Err(error) => {
				return message_boundary_error_response(
					encoding,
					StatusCode::BAD_REQUEST,
					error.context("decode HTTP action request"),
				);
			}
		};
		let conn_params = match http_conn_params(request.headers()) {
			Ok(params) => params,
			Err(error) => {
				return message_boundary_error_response(
					encoding,
					StatusCode::BAD_REQUEST,
					error.context("decode HTTP action connection params"),
				);
			}
		};
		let conn = match instance
			.ctx
			.connect_conn_with_request(conn_params, Some(request.clone()), async {
				Ok::<Vec<u8>, anyhow::Error>(Vec::new())
			})
			.await
		{
			Ok(conn) => conn,
			Err(error) => {
				return message_boundary_error_response(
					encoding,
					framework_anyhow_status(&error),
					error.context("connect HTTP action request"),
				);
			}
		};

		let dispatch_result = with_action_dispatch_timeout(
			config.action_timeout,
			dispatch_action_through_task(
				&instance.dispatch,
				config.dispatch_command_inbox_capacity,
				conn.clone(),
				action_name.clone(),
				args,
			),
		)
		.await;
		let disconnect_result = conn.disconnect(None).await;

		match dispatch_result {
			Ok(output) => {
				if let Err(error) = disconnect_result {
					tracing::warn!(
						actor_id = instance.actor_id,
						conn_id = conn.id(),
						?error,
						"failed to disconnect HTTP action connection"
					);
				}
				let response = encode_http_action_response(encoding, output)?;
				if response.body.as_ref().map(Vec::len).unwrap_or_default()
					> config.max_outgoing_message_size as usize
				{
					return message_boundary_error_response(
						encoding,
						StatusCode::BAD_REQUEST,
						OutgoingMessageTooLong.build(),
					);
				}
				Ok(response)
			}
			Err(error) => {
				if let Err(disconnect_error) = disconnect_result {
					tracing::warn!(
						actor_id = instance.actor_id,
						conn_id = conn.id(),
						?disconnect_error,
						"failed to disconnect HTTP action connection after error"
					);
				}
				framework_action_error_response(encoding, error)
			}
		}
	}

	async fn handle_queue_fetch(
		&self,
		instance: &ActorTaskHandle,
		request: Request,
		queue_name: String,
	) -> Result<HttpResponse> {
		let encoding = request_encoding(request.headers());
		if request.method() != http::Method::POST {
			return message_boundary_error_response(
				encoding,
				framework_error_status("actor", "method_not_allowed"),
				MethodNotAllowed {
					method: request.method().to_string(),
					path: request.uri().path().to_owned(),
				}
				.build(),
			);
		}

		let config = instance.factory.config();
		if request.body().len() > config.max_incoming_message_size as usize {
			return message_boundary_error_response(
				encoding,
				StatusCode::BAD_REQUEST,
				IncomingMessageTooLong.build(),
			);
		}

		let queue_request = match decode_http_queue_request(encoding, request.body()) {
			Ok(queue_request) => queue_request,
			Err(error) => {
				return message_boundary_error_response(
					encoding,
					StatusCode::BAD_REQUEST,
					error.context("decode HTTP queue request"),
				);
			}
		};
		let conn_params = match http_conn_params(request.headers()) {
			Ok(params) => params,
			Err(error) => {
				return message_boundary_error_response(
					encoding,
					StatusCode::BAD_REQUEST,
					error.context("decode HTTP queue connection params"),
				);
			}
		};
		let conn = match instance
			.ctx
			.connect_conn_with_request(conn_params, Some(request.clone()), async {
				Ok::<Vec<u8>, anyhow::Error>(Vec::new())
			})
			.await
		{
			Ok(conn) => conn,
			Err(error) => {
				return message_boundary_error_response(
					encoding,
					framework_anyhow_status(&error),
					error.context("connect HTTP queue request"),
				);
			}
		};

		let (reply_tx, reply_rx) = oneshot::channel();
		let dispatch_result = try_send_dispatch_command(
			&instance.dispatch,
			config.dispatch_command_inbox_capacity,
			"dispatch_queue_send",
			DispatchCommand::QueueSend {
				name: queue_name,
				body: queue_request.body,
				conn: conn.clone(),
				request,
				wait: queue_request.wait,
				timeout_ms: queue_request.timeout,
				reply: reply_tx,
			},
			Some(instance.ctx.metrics()),
		);

		let queue_result = match dispatch_result {
			Ok(()) => {
				with_framework_action_timeout(config.action_timeout, async {
					reply_rx
						.await
						.context("receive actor task queue send reply")?
				})
				.await
			}
			Err(error) => Err(error),
		};
		let disconnect_result = conn.disconnect(None).await;

		match queue_result {
			Ok(result) => {
				if let Err(error) = disconnect_result {
					tracing::warn!(
						actor_id = instance.actor_id,
						conn_id = conn.id(),
						?error,
						"failed to disconnect HTTP queue connection"
					);
				}
				let response = encode_http_queue_response(encoding, result)?;
				if response.body.as_ref().map(Vec::len).unwrap_or_default()
					> config.max_outgoing_message_size as usize
				{
					return message_boundary_error_response(
						encoding,
						StatusCode::BAD_REQUEST,
						OutgoingMessageTooLong.build(),
					);
				}
				Ok(response)
			}
			Err(error) => {
				if let Err(disconnect_error) = disconnect_result {
					tracing::warn!(
						actor_id = instance.actor_id,
						conn_id = conn.id(),
						?disconnect_error,
						"failed to disconnect HTTP queue connection after error"
					);
				}
				message_boundary_error_response(encoding, framework_anyhow_status(&error), error)
			}
		}
	}

	fn handle_metrics_fetch(
		&self,
		instance: &ActorTaskHandle,
		request: &HttpRequest,
	) -> Result<HttpResponse> {
		if !request_has_bearer_token(request, self.inspector_token.as_deref()) {
			return Ok(unauthorized_response());
		}

		let mut headers = HashMap::new();
		headers.insert(
			http::header::CONTENT_TYPE.to_string(),
			instance.ctx.metrics_content_type().to_owned(),
		);

		Ok(HttpResponse {
			status: http::StatusCode::OK.as_u16(),
			headers,
			body: Some(
				instance
					.ctx
					.render_metrics()
					.context("render actor prometheus metrics")?
					.into_bytes(),
			),
			body_stream: None,
		})
	}
}

pub(super) enum FrameworkHttpRoute {
	Action(String),
	Queue(String),
}

pub(super) struct DecodedHttpQueueRequest {
	body: Vec<u8>,
	wait: bool,
	timeout: Option<u64>,
}

pub(super) fn framework_http_route(path: &str) -> Result<Option<FrameworkHttpRoute>> {
	if let Some(segment) = single_path_segment(path, "/action/") {
		return Ok(Some(FrameworkHttpRoute::Action(
			percent_decode_path_segment(segment)?,
		)));
	}
	if let Some(segment) = single_path_segment(path, "/queue/") {
		return Ok(Some(FrameworkHttpRoute::Queue(
			percent_decode_path_segment(segment)?,
		)));
	}
	Ok(None)
}

pub(super) fn single_path_segment<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
	let segment = path.strip_prefix(prefix)?;
	(!segment.is_empty() && !segment.contains('/')).then_some(segment)
}

pub(super) fn percent_decode_path_segment(segment: &str) -> Result<String> {
	let bytes = segment.as_bytes();
	let mut out = Vec::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		if bytes[i] == b'%' {
			if i + 2 >= bytes.len() {
				return Err(invalid_path_segment("incomplete percent escape"));
			}
			let hi = hex_value(bytes[i + 1])
				.ok_or_else(|| invalid_path_segment("invalid percent escape"))?;
			let lo = hex_value(bytes[i + 2])
				.ok_or_else(|| invalid_path_segment("invalid percent escape"))?;
			out.push((hi << 4) | lo);
			i += 3;
		} else {
			out.push(bytes[i]);
			i += 1;
		}
	}
	String::from_utf8(out).context("path segment is not valid utf-8")
}

fn invalid_path_segment(reason: &str) -> anyhow::Error {
	ProtocolError::InvalidHttpRequest {
		field: "path segment".to_owned(),
		reason: reason.to_owned(),
	}
	.build()
}

pub(super) fn hex_value(byte: u8) -> Option<u8> {
	match byte {
		b'0'..=b'9' => Some(byte - b'0'),
		b'a'..=b'f' => Some(byte - b'a' + 10),
		b'A'..=b'F' => Some(byte - b'A' + 10),
		_ => None,
	}
}

pub(super) fn http_conn_params(headers: &http::HeaderMap) -> Result<Vec<u8>> {
	let Some(raw) = headers
		.get("x-rivet-conn-params")
		.and_then(|value| value.to_str().ok())
	else {
		return Ok(Vec::new());
	};
	let value: JsonValue = serde_json::from_str(raw).context("parse x-rivet-conn-params header")?;
	encode_json_as_cbor(&value)
}

pub(super) fn authorization_bearer_token(headers: &http::HeaderMap) -> Option<&str> {
	headers
		.get(http::header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(bearer_token_from_authorization)
}

pub(super) fn authorization_bearer_token_map(headers: &HashMap<String, String>) -> Option<&str> {
	headers
		.iter()
		.find(|(name, _)| name.eq_ignore_ascii_case(http::header::AUTHORIZATION.as_str()))
		.and_then(|(_, value)| bearer_token_from_authorization(value))
}

pub(super) async fn build_http_request(request: HttpRequest) -> Result<Request> {
	let mut body = request.body.unwrap_or_default();
	if let Some(mut body_stream) = request.body_stream {
		while let Some(chunk) = body_stream.recv().await {
			body.extend_from_slice(&chunk);
		}
	}

	let request_path = normalize_actor_request_path(&request.path);
	Request::from_parts(&request.method, &request_path, request.headers, body)
		.with_context(|| format!("build actor request for `{}`", request.path))
}

pub(super) fn normalize_actor_request_path(path: &str) -> String {
	let Some(stripped) = path.strip_prefix("/request") else {
		return path.to_owned();
	};

	if stripped.is_empty() {
		return "/".to_owned();
	}

	match stripped.as_bytes().first() {
		Some(b'/') | Some(b'?') => stripped.to_owned(),
		_ => path.to_owned(),
	}
}

pub(super) fn build_envoy_response(response: Response) -> Result<HttpResponse> {
	let (status, headers, body) = response.to_parts();

	Ok(HttpResponse {
		status,
		headers,
		body: Some(body),
		body_stream: None,
	})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HttpResponseEncoding {
	Json,
	Cbor,
	Bare,
}

pub(super) fn request_encoding(headers: &http::HeaderMap) -> HttpResponseEncoding {
	headers
		.get("x-rivet-encoding")
		.and_then(|value| value.to_str().ok())
		.map(|value| match value {
			"cbor" => HttpResponseEncoding::Cbor,
			"bare" => HttpResponseEncoding::Bare,
			_ => HttpResponseEncoding::Json,
		})
		.unwrap_or(HttpResponseEncoding::Json)
}

pub(super) fn message_boundary_error_response(
	encoding: HttpResponseEncoding,
	status: StatusCode,
	error: anyhow::Error,
) -> Result<HttpResponse> {
	let error = RivetError::extract(&error);
	let body = serialize_http_response_error(
		encoding,
		error.group(),
		error.code(),
		error.message(),
		None,
	)?;

	Ok(HttpResponse {
		status: status.as_u16(),
		headers: HashMap::from([(
			http::header::CONTENT_TYPE.to_string(),
			content_type_for_encoding(encoding).to_owned(),
		)]),
		body: Some(body),
		body_stream: None,
	})
}

pub(super) fn content_type_for_encoding(encoding: HttpResponseEncoding) -> &'static str {
	match encoding {
		HttpResponseEncoding::Json => "application/json",
		HttpResponseEncoding::Cbor | HttpResponseEncoding::Bare => "application/octet-stream",
	}
}

pub(super) fn serialize_http_response_error(
	encoding: HttpResponseEncoding,
	group: &str,
	code: &str,
	message: &str,
	metadata: Option<&JsonValue>,
) -> Result<Vec<u8>> {
	let mut json_body = json!({
		"group": group,
		"code": code,
		"message": message,
	});
	if let Some(metadata) = metadata {
		json_body["metadata"] = metadata.clone();
	}

	match encoding {
		HttpResponseEncoding::Json => Ok(serde_json::to_vec(&json_body)?),
		HttpResponseEncoding::Cbor => {
			let mut out = Vec::new();
			ciborium::into_writer(&json_body, &mut out)?;
			Ok(out)
		}
		HttpResponseEncoding::Bare => {
			let metadata = metadata
				.map(|value| {
					let mut out = Vec::new();
					ciborium::into_writer(value, &mut out)?;
					Ok::<Vec<u8>, anyhow::Error>(out)
				})
				.transpose()?;
			client_protocol::versioned::HttpResponseError::wrap_latest(
				client_protocol::HttpResponseError {
					group: group.to_owned(),
					code: code.to_owned(),
					message: message.to_owned(),
					metadata,
				},
			)
			.serialize_with_embedded_version(client_protocol::PROTOCOL_VERSION)
		}
	}
}

pub(super) fn decode_http_action_args(
	encoding: HttpResponseEncoding,
	body: &[u8],
) -> Result<Vec<u8>> {
	match encoding {
		HttpResponseEncoding::Json => {
			let request: HttpActionRequestJson =
				serde_json::from_slice(body).context("decode json HTTP action request")?;
			let args = match request.args {
				JsonValue::Array(args) => args,
				_ => Vec::new(),
			};
			encode_json_as_cbor(&args)
		}
		HttpResponseEncoding::Cbor => {
			let request: HttpActionRequestJson = ciborium::from_reader(Cursor::new(body))
				.context("decode cbor HTTP action request")?;
			let args = match request.args {
				JsonValue::Array(args) => args,
				_ => Vec::new(),
			};
			encode_json_as_cbor(&args)
		}
		HttpResponseEncoding::Bare => {
			let request =
				<client_protocol::versioned::HttpActionRequest as OwnedVersionedData>::deserialize_with_embedded_version(body)
					.context("decode bare HTTP action request")?;
			Ok(request.args)
		}
	}
}

pub(super) fn decode_http_queue_request(
	encoding: HttpResponseEncoding,
	body: &[u8],
) -> Result<DecodedHttpQueueRequest> {
	match encoding {
		HttpResponseEncoding::Json => {
			let request: HttpQueueSendRequestJson =
				serde_json::from_slice(body).context("decode json HTTP queue request")?;
			Ok(DecodedHttpQueueRequest {
				body: encode_json_as_cbor(&request.body)?,
				wait: request.wait.unwrap_or(false),
				timeout: request.timeout,
			})
		}
		HttpResponseEncoding::Cbor => {
			let request: HttpQueueSendRequestJson = ciborium::from_reader(Cursor::new(body))
				.context("decode cbor HTTP queue request")?;
			Ok(DecodedHttpQueueRequest {
				body: encode_json_as_cbor(&request.body)?,
				wait: request.wait.unwrap_or(false),
				timeout: request.timeout,
			})
		}
		HttpResponseEncoding::Bare => {
			let request =
				<client_protocol::versioned::HttpQueueSendRequest as OwnedVersionedData>::deserialize_with_embedded_version(body)
					.context("decode bare HTTP queue request")?;
			Ok(DecodedHttpQueueRequest {
				body: request.body,
				wait: request.wait.unwrap_or(false),
				timeout: request.timeout,
			})
		}
	}
}

pub(super) fn encode_http_action_response(
	encoding: HttpResponseEncoding,
	output: Vec<u8>,
) -> Result<HttpResponse> {
	let body = match encoding {
		HttpResponseEncoding::Json => serde_json::to_vec(&json!({
			"output": decode_cbor_json_or_null(&output),
		}))?,
		HttpResponseEncoding::Cbor => {
			let mut out = Vec::new();
			ciborium::into_writer(
				&json!({
					"output": decode_cbor_json_or_null(&output),
				}),
				&mut out,
			)?;
			out
		}
		HttpResponseEncoding::Bare => client_protocol::versioned::HttpActionResponse::wrap_latest(
			client_protocol::HttpActionResponse { output },
		)
		.serialize_with_embedded_version(client_protocol::PROTOCOL_VERSION)?,
	};
	Ok(HttpResponse {
		status: StatusCode::OK.as_u16(),
		headers: HashMap::from([(
			http::header::CONTENT_TYPE.to_string(),
			content_type_for_encoding(encoding).to_owned(),
		)]),
		body: Some(body),
		body_stream: None,
	})
}

pub(super) fn encode_http_queue_response(
	encoding: HttpResponseEncoding,
	result: QueueSendResult,
) -> Result<HttpResponse> {
	let body = match encoding {
		HttpResponseEncoding::Json => {
			let mut value = serde_json::Map::new();
			value.insert("status".to_owned(), json!(result.status.as_str()));
			if let Some(response) = result.response {
				value.insert("response".to_owned(), decode_cbor_json_or_null(&response));
			}
			serde_json::to_vec(&JsonValue::Object(value))?
		}
		HttpResponseEncoding::Cbor => {
			let mut value = serde_json::Map::new();
			value.insert("status".to_owned(), json!(result.status.as_str()));
			if let Some(response) = result.response {
				value.insert("response".to_owned(), decode_cbor_json_or_null(&response));
			}
			let mut out = Vec::new();
			ciborium::into_writer(&JsonValue::Object(value), &mut out)?;
			out
		}
		HttpResponseEncoding::Bare => {
			client_protocol::versioned::HttpQueueSendResponse::wrap_latest(
				client_protocol::HttpQueueSendResponse {
					status: result.status.as_str().to_owned(),
					response: result.response,
				},
			)
			.serialize_with_embedded_version(client_protocol::PROTOCOL_VERSION)?
		}
	};
	Ok(HttpResponse {
		status: StatusCode::OK.as_u16(),
		headers: HashMap::from([(
			http::header::CONTENT_TYPE.to_string(),
			content_type_for_encoding(encoding).to_owned(),
		)]),
		body: Some(body),
		body_stream: None,
	})
}

pub(super) fn framework_action_error_response(
	encoding: HttpResponseEncoding,
	error: ActionDispatchError,
) -> Result<HttpResponse> {
	let status = framework_error_status(&error.group, &error.code);
	Ok(HttpResponse {
		status: status.as_u16(),
		headers: HashMap::from([(
			http::header::CONTENT_TYPE.to_string(),
			content_type_for_encoding(encoding).to_owned(),
		)]),
		body: Some(serialize_http_response_error(
			encoding,
			&error.group,
			&error.code,
			&error.message,
			error.metadata.as_ref(),
		)?),
		body_stream: None,
	})
}

pub(super) fn framework_anyhow_status(error: &anyhow::Error) -> StatusCode {
	let error = RivetError::extract(error);
	framework_error_status(error.group(), error.code())
}

pub(super) fn framework_error_status(group: &str, code: &str) -> StatusCode {
	match (group, code) {
		("auth", "forbidden") => StatusCode::FORBIDDEN,
		("actor", "action_not_found") => StatusCode::NOT_FOUND,
		("actor", "action_timed_out") => StatusCode::REQUEST_TIMEOUT,
		("actor", "invalid_request") => StatusCode::BAD_REQUEST,
		("actor", "method_not_allowed") => StatusCode::METHOD_NOT_ALLOWED,
		("message", "incoming_too_long" | "outgoing_too_long") => StatusCode::BAD_REQUEST,
		("queue", _) => StatusCode::BAD_REQUEST,
		_ => StatusCode::INTERNAL_SERVER_ERROR,
	}
}

pub(super) fn unauthorized_response() -> HttpResponse {
	HttpResponse {
		status: http::StatusCode::UNAUTHORIZED.as_u16(),
		headers: HashMap::new(),
		body: Some(Vec::new()),
		body_stream: None,
	}
}

pub(super) fn request_has_bearer_token(
	request: &HttpRequest,
	configured_token: Option<&str>,
) -> bool {
	let Some(configured_token) = configured_token else {
		return false;
	};

	request.headers.iter().any(|(name, value)| {
		name.eq_ignore_ascii_case(http::header::AUTHORIZATION.as_str())
			&& bearer_token_from_authorization(value) == Some(configured_token)
	})
}

fn bearer_token_from_authorization(value: &str) -> Option<&str> {
	let value = value.trim_start();
	let scheme = value.get(..6)?;
	if !scheme.eq_ignore_ascii_case("bearer") {
		return None;
	}

	let rest = value.get(6..)?;
	if !rest.chars().next().is_some_and(char::is_whitespace) {
		return None;
	}

	let token = rest.trim_start();
	if token.is_empty() { None } else { Some(token) }
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::time::Duration;

	use super::{
		HttpRequest, HttpResponseEncoding, authorization_bearer_token,
		authorization_bearer_token_map, framework_action_error_response,
		message_boundary_error_response, request_encoding, request_has_bearer_token,
		workflow_dispatch_result,
	};
	use crate::actor::action::ActionDispatchError;
	use crate::error::ActorLifecycle as ActorLifecycleError;
	use http::StatusCode;
	use rivet_error::RivetError;
	use serde_json::json;

	#[derive(RivetError)]
	#[error("message", "incoming_too_long", "Incoming message too long")]
	struct IncomingMessageTooLong;

	#[derive(RivetError)]
	#[error("message", "outgoing_too_long", "Outgoing message too long")]
	struct OutgoingMessageTooLong;

	#[test]
	fn workflow_dispatch_result_marks_handled_workflow_as_enabled() {
		assert_eq!(
			workflow_dispatch_result(Ok(Some(vec![1, 2, 3])))
				.expect("workflow dispatch should succeed"),
			(true, Some(vec![1, 2, 3])),
		);
		assert_eq!(
			workflow_dispatch_result(Ok(None)).expect("workflow dispatch should succeed"),
			(true, None),
		);
	}

	#[test]
	fn workflow_dispatch_result_treats_dropped_reply_as_disabled() {
		assert_eq!(
			workflow_dispatch_result(Err(ActorLifecycleError::DroppedReply.build()))
				.expect("dropped reply should map to workflow disabled"),
			(false, None),
		);
	}

	#[test]
	fn workflow_dispatch_result_preserves_non_dropped_reply_errors() {
		let error = workflow_dispatch_result(Err(ActorLifecycleError::Destroying.build()))
			.expect_err("non-dropped reply errors should be preserved");
		let error = rivet_error::RivetError::extract(&error);
		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "destroying");
	}

	#[test]
	fn inspector_error_status_maps_action_timeout_to_408() {
		assert_eq!(
			super::inspector_error_status("actor", "action_timed_out"),
			StatusCode::REQUEST_TIMEOUT,
		);
	}

	#[test]
	fn authorization_bearer_token_accepts_case_insensitive_scheme_and_whitespace() {
		let mut headers = http::HeaderMap::new();
		headers.insert(
			http::header::AUTHORIZATION,
			"bearer   test-token".parse().unwrap(),
		);

		assert_eq!(authorization_bearer_token(&headers), Some("test-token"));

		let map = HashMap::from([(
			http::header::AUTHORIZATION.as_str().to_owned(),
			"BEARER\ttest-token".to_owned(),
		)]);
		assert_eq!(authorization_bearer_token_map(&map), Some("test-token"));
	}

	#[test]
	fn request_has_bearer_token_uses_same_authorization_parser() {
		let request = HttpRequest {
			method: "GET".to_owned(),
			path: "/metrics".to_owned(),
			headers: HashMap::from([(
				http::header::AUTHORIZATION.as_str().to_owned(),
				"Bearer   configured".to_owned(),
			)]),
			body: Some(Vec::new()),
			body_stream: None,
		};

		assert!(request_has_bearer_token(&request, Some("configured")));
		assert!(!request_has_bearer_token(&request, Some("other")));
	}

	#[tokio::test]
	async fn action_dispatch_timeout_returns_structured_error() {
		let error = super::with_action_dispatch_timeout(Duration::from_millis(1), async {
			tokio::time::sleep(Duration::from_secs(60)).await;
			Ok::<Vec<u8>, ActionDispatchError>(Vec::new())
		})
		.await
		.expect_err("timeout should return an action dispatch error");

		assert_eq!(error.group, "actor");
		assert_eq!(error.code, "action_timed_out");
		assert_eq!(error.message, "Action timed out");
	}

	#[tokio::test]
	async fn framework_action_timeout_returns_structured_error() {
		let error = super::with_framework_action_timeout(Duration::from_millis(1), async {
			tokio::time::sleep(Duration::from_secs(60)).await;
			Ok::<(), anyhow::Error>(())
		})
		.await
		.expect_err("timeout should return a framework error");
		let error = RivetError::extract(&error);

		assert_eq!(error.group(), "actor");
		assert_eq!(error.code(), "action_timed_out");
		assert_eq!(error.message(), "Action timed out");
	}

	#[test]
	fn framework_action_error_response_maps_timeout_to_408() {
		let response = framework_action_error_response(
			HttpResponseEncoding::Json,
			ActionDispatchError {
				group: "actor".to_owned(),
				code: "action_timed_out".to_owned(),
				message: "Action timed out".to_owned(),
				metadata: None,
			},
		)
		.expect("timeout error response should serialize");

		assert_eq!(response.status, StatusCode::REQUEST_TIMEOUT.as_u16());
		assert_eq!(
			response.body,
			Some(
				serde_json::to_vec(&json!({
					"group": "actor",
					"code": "action_timed_out",
					"message": "Action timed out",
				}))
				.expect("json body should encode")
			)
		);
	}

	#[test]
	fn message_boundary_error_response_defaults_to_json() {
		let response = message_boundary_error_response(
			HttpResponseEncoding::Json,
			StatusCode::BAD_REQUEST,
			IncomingMessageTooLong.build(),
		)
		.expect("json response should serialize");

		assert_eq!(response.status, StatusCode::BAD_REQUEST.as_u16());
		assert_eq!(
			response.headers.get(http::header::CONTENT_TYPE.as_str()),
			Some(&"application/json".to_owned())
		);
		assert_eq!(
			response.body,
			Some(
				serde_json::to_vec(&json!({
					"group": "message",
					"code": "incoming_too_long",
					"message": "Incoming message too long",
				}))
				.expect("json body should encode")
			)
		);
	}

	#[test]
	fn request_encoding_reads_cbor_header() {
		let mut headers = http::HeaderMap::new();
		headers.insert("x-rivet-encoding", "cbor".parse().unwrap());

		assert_eq!(request_encoding(&headers), HttpResponseEncoding::Cbor);
	}

	#[test]
	fn message_boundary_error_response_serializes_bare_v3() {
		use vbare::OwnedVersionedData;

		let response = message_boundary_error_response(
			HttpResponseEncoding::Bare,
			StatusCode::BAD_REQUEST,
			OutgoingMessageTooLong.build(),
		)
		.expect("bare response should serialize");

		assert_eq!(
			response.headers.get(http::header::CONTENT_TYPE.as_str()),
			Some(&"application/octet-stream".to_owned())
		);

		let body = response.body.expect("bare response should include body");
		let decoded =
			<rivetkit_client_protocol::versioned::HttpResponseError as OwnedVersionedData>::deserialize_with_embedded_version(&body)
				.expect("bare error should decode");
		assert_eq!(decoded.group, "message");
		assert_eq!(decoded.code, "outgoing_too_long");
		assert_eq!(decoded.message, "Outgoing message too long");
		assert_eq!(decoded.metadata, None);
	}
}
