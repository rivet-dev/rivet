use anyhow::{anyhow, Context, Result};
use rivetkit_client_protocol as wire;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use vbare::Versioned;

use crate::EncodingKind;

use super::{to_client, to_server};

pub fn encode_to_server(encoding: EncodingKind, value: &to_server::ToServer) -> Result<Vec<u8>> {
	match encoding {
		EncodingKind::Json => Ok(serde_json::to_vec(&to_server_json_value(value)?)?),
		EncodingKind::Cbor => Ok(serde_cbor::to_vec(&to_server_json_value(value)?)?),
		EncodingKind::Bare => encode_to_server_bare(value),
	}
}

pub fn decode_to_client(encoding: EncodingKind, payload: &[u8]) -> Result<to_client::ToClient> {
	match encoding {
		EncodingKind::Json => {
			let value: JsonValue =
				serde_json::from_slice(payload).context("decode actor websocket json response")?;
			to_client_from_json_value(&value)
		}
		EncodingKind::Cbor => {
			let value: JsonValue =
				serde_cbor::from_slice(payload).context("decode actor websocket cbor response")?;
			to_client_from_json_value(&value)
		}
		EncodingKind::Bare => decode_to_client_bare(payload),
	}
}

pub fn encode_http_action_request(encoding: EncodingKind, args: &[JsonValue]) -> Result<Vec<u8>> {
	match encoding {
		EncodingKind::Json => Ok(serde_json::to_vec(&json!({ "args": args }))?),
		EncodingKind::Cbor => Ok(serde_cbor::to_vec(&json!({ "args": args }))?),
		EncodingKind::Bare => wire::versioned::HttpActionRequest::serialize_with_embedded_version(
			&wire::HttpActionRequest {
				args: serde_cbor::to_vec(&args.to_vec())?.into(),
			},
			wire::PROTOCOL_VERSION,
		),
	}
}

pub fn decode_http_action_response(encoding: EncodingKind, payload: &[u8]) -> Result<JsonValue> {
	match encoding {
		EncodingKind::Json => {
			let value: JsonValue = serde_json::from_slice(payload)?;
			value
				.get("output")
				.cloned()
				.ok_or_else(|| anyhow!("action response missing output"))
		}
		EncodingKind::Cbor => {
			let value: JsonValue = serde_cbor::from_slice(payload)?;
			value
				.get("output")
				.cloned()
				.ok_or_else(|| anyhow!("action response missing output"))
		}
		EncodingKind::Bare => {
			let response =
                <wire::versioned::HttpActionResponse as Versioned>::deserialize_with_embedded_version(
                    payload,
                )
                .context("decode bare action response")?;
			Ok(serde_cbor::from_slice(&response.output)?)
		}
	}
}

pub fn encode_http_queue_request<T: Serialize>(
	encoding: EncodingKind,
	name: &str,
	body: &T,
	wait: bool,
	timeout: Option<u64>,
) -> Result<Vec<u8>> {
	#[derive(Serialize)]
	struct JsonQueueRequest<'a, T: Serialize + ?Sized> {
		name: &'a str,
		body: &'a T,
		wait: bool,
		#[serde(skip_serializing_if = "Option::is_none")]
		timeout: Option<u64>,
	}

	let request = JsonQueueRequest {
		name,
		body,
		wait,
		timeout,
	};

	match encoding {
		EncodingKind::Json => Ok(serde_json::to_vec(&request)?),
		EncodingKind::Cbor => Ok(serde_cbor::to_vec(&request)?),
		EncodingKind::Bare => {
			wire::versioned::HttpQueueSendRequest::serialize_with_embedded_version(
				&wire::HttpQueueSendRequest {
					body: serde_cbor::to_vec(body)?.into(),
					name: Some(name.to_owned().into()),
					wait: Some(wait),
					timeout,
				},
				wire::PROTOCOL_VERSION,
			)
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueSendStatus {
	Completed,
	TimedOut,
	Other(String),
}

#[derive(Debug, Clone)]
pub struct QueueSendResult {
	pub status: QueueSendStatus,
	pub response: Option<JsonValue>,
}

pub fn decode_http_queue_response(
	encoding: EncodingKind,
	payload: &[u8],
) -> Result<QueueSendResult> {
	let (status, response) = match encoding {
		EncodingKind::Json => {
			let value: JsonValue = serde_json::from_slice(payload)?;
			let status = value
				.get("status")
				.and_then(JsonValue::as_str)
				.ok_or_else(|| anyhow!("queue response missing status"))?
				.to_owned();
			let response = value.get("response").cloned();
			(status, response)
		}
		EncodingKind::Cbor => {
			let value: JsonValue = serde_cbor::from_slice(payload)?;
			let status = value
				.get("status")
				.and_then(JsonValue::as_str)
				.ok_or_else(|| anyhow!("queue response missing status"))?
				.to_owned();
			let response = value.get("response").cloned();
			(status, response)
		}
		EncodingKind::Bare => {
			let response =
                <wire::versioned::HttpQueueSendResponse as Versioned>::deserialize_with_embedded_version(
                    payload,
                )
                .context("decode bare queue response")?;
			let body = response
				.response
				.map(|payload| serde_cbor::from_slice(&payload))
				.transpose()?;
			(response.status.to_string(), body)
		}
	};

	let status = match status.as_str() {
		"completed" => QueueSendStatus::Completed,
		"timedOut" => QueueSendStatus::TimedOut,
		_ => QueueSendStatus::Other(status),
	};

	Ok(QueueSendResult { status, response })
}

pub fn decode_http_error(
	encoding: EncodingKind,
	payload: &[u8],
) -> Result<(String, String, String, Option<JsonValue>)> {
	match encoding {
		EncodingKind::Json => {
			let value: JsonValue = serde_json::from_slice(payload)?;
			error_from_json_value(&value)
		}
		EncodingKind::Cbor => {
			let value: JsonValue = serde_cbor::from_slice(payload)?;
			error_from_json_value(&value)
		}
		EncodingKind::Bare => {
			// Routing-level errors from the gateway (e.g. actor not found) are
			// emitted as JSON regardless of the client encoding, so fall back to
			// parsing JSON when the payload is not valid BARE.
			match <wire::versioned::HttpResponseError as Versioned>::deserialize_with_embedded_version(
				payload,
			) {
				Ok(error) => {
					let metadata = error
						.metadata
						.map(|payload| serde_cbor::from_slice(&payload))
						.transpose()?;
					Ok((error.group.to_string(), error.code.to_string(), error.message.to_string(), metadata))
				}
				Err(bare_err) => serde_json::from_slice::<JsonValue>(payload)
					.ok()
					.and_then(|value| error_from_json_value(&value).ok())
					.ok_or(bare_err)
					.context("decode bare http error"),
			}
		}
	}
}

fn to_server_json_value(value: &to_server::ToServer) -> Result<JsonValue> {
	let body = match &value.body {
		to_server::ToServerBody::ActionRequest(request) => json!({
			"tag": "ActionRequest",
			"val": {
				"id": request.id,
				"name": request.name,
				"args": serde_cbor::from_slice::<JsonValue>(&request.args)
					.context("decode websocket action args for json/cbor transport")?,
			},
		}),
		to_server::ToServerBody::SubscriptionRequest(request) => json!({
			"tag": "SubscriptionRequest",
			"val": {
				"eventName": request.event_name,
				"subscribe": request.subscribe,
			},
		}),
	};
	Ok(json!({ "body": body }))
}

fn to_client_from_json_value(value: &JsonValue) -> Result<to_client::ToClient> {
	let body = value
		.get("body")
		.and_then(JsonValue::as_object)
		.ok_or_else(|| anyhow!("actor websocket response missing body"))?;
	let tag = body
		.get("tag")
		.and_then(JsonValue::as_str)
		.ok_or_else(|| anyhow!("actor websocket response missing tag"))?;
	let value = body
		.get("val")
		.and_then(JsonValue::as_object)
		.ok_or_else(|| anyhow!("actor websocket response missing val"))?;

	let body = match tag {
		"Init" => to_client::ToClientBody::Init(to_client::Init {
			actor_id: json_string(value, "actorId")?,
			connection_id: json_string(value, "connectionId")?,
			connection_token: value
				.get("connectionToken")
				.and_then(JsonValue::as_str)
				.map(ToOwned::to_owned),
		}),
		"Error" => to_client::ToClientBody::Error(to_client::Error {
			group: json_string(value, "group")?,
			code: json_string(value, "code")?,
			message: json_string(value, "message")?,
			metadata: value.get("metadata").map(serde_cbor::to_vec).transpose()?,
			action_id: value.get("actionId").map(parse_json_u64).transpose()?,
		}),
		"ActionResponse" => to_client::ToClientBody::ActionResponse(to_client::ActionResponse {
			id: parse_json_u64(
				value
					.get("id")
					.ok_or_else(|| anyhow!("action response missing id"))?,
			)?,
			output: serde_cbor::to_vec(
				value
					.get("output")
					.ok_or_else(|| anyhow!("action response missing output"))?,
			)?,
		}),
		"Event" => to_client::ToClientBody::Event(to_client::Event {
			name: json_string(value, "name")?,
			args: serde_cbor::to_vec(
				value
					.get("args")
					.ok_or_else(|| anyhow!("event response missing args"))?,
			)?,
		}),
		other => return Err(anyhow!("unknown actor websocket response tag `{other}`")),
	};

	Ok(to_client::ToClient { body })
}

fn encode_to_server_bare(value: &to_server::ToServer) -> Result<Vec<u8>> {
	let body = match &value.body {
		to_server::ToServerBody::ActionRequest(request) => {
			wire::ToServerBody::ActionRequest(wire::ActionRequest {
				id: request.id,
				name: request.name.clone().into(),
				args: request.args.clone().into(),
			})
		}
		to_server::ToServerBody::SubscriptionRequest(request) => {
			wire::ToServerBody::SubscriptionRequest(wire::SubscriptionRequest {
				event_name: request.event_name.clone().into(),
				subscribe: request.subscribe,
			})
		}
	};

	wire::versioned::ToServer::serialize_with_embedded_version(
		&wire::ToServer { body },
		wire::PROTOCOL_VERSION,
	)
}

fn decode_to_client_bare(payload: &[u8]) -> Result<to_client::ToClient> {
	let message =
		<wire::versioned::ToClient as Versioned>::deserialize_with_embedded_version(payload)
			.context("decode bare actor websocket response")?;

	let body = match message.body {
		wire::ToClientBody::Init(init) => to_client::ToClientBody::Init(to_client::Init {
			actor_id: init.actor_id.to_string(),
			connection_id: init.connection_id.to_string(),
			connection_token: None,
		}),
		wire::ToClientBody::Error(error) => to_client::ToClientBody::Error(to_client::Error {
			group: error.group.to_string(),
			code: error.code.to_string(),
			message: error.message.to_string(),
			metadata: error.metadata.map(|value| value.to_vec()),
			action_id: error.action_id,
		}),
		wire::ToClientBody::ActionResponse(response) => {
			to_client::ToClientBody::ActionResponse(to_client::ActionResponse {
				id: response.id,
				output: response.output.to_vec(),
			})
		}
		wire::ToClientBody::Event(event) => to_client::ToClientBody::Event(to_client::Event {
			name: event.name.to_string(),
			args: event.args.to_vec(),
		}),
	};

	Ok(to_client::ToClient { body })
}

fn json_string(value: &serde_json::Map<String, JsonValue>, key: &str) -> Result<String> {
	value
		.get(key)
		.and_then(JsonValue::as_str)
		.map(ToOwned::to_owned)
		.ok_or_else(|| anyhow!("json object missing string field `{key}`"))
}

fn parse_json_u64(value: &JsonValue) -> Result<u64> {
	match value {
		JsonValue::Number(number) => number
			.as_u64()
			.ok_or_else(|| anyhow!("json number is not an unsigned integer")),
		JsonValue::Array(values) if values.len() == 2 => {
			let tag = values[0]
				.as_str()
				.ok_or_else(|| anyhow!("json bigint tag is not a string"))?;
			let raw = values[1]
				.as_str()
				.ok_or_else(|| anyhow!("json bigint value is not a string"))?;
			if tag != "$BigInt" {
				return Err(anyhow!("unsupported json bigint tag `{tag}`"));
			}
			raw.parse::<u64>().context("parse json bigint")
		}
		_ => Err(anyhow!("invalid json unsigned integer")),
	}
}

fn error_from_json_value(value: &JsonValue) -> Result<(String, String, String, Option<JsonValue>)> {
	let value = value
		.as_object()
		.ok_or_else(|| anyhow!("http error response is not an object"))?;
	Ok((
		json_string(value, "group")?,
		json_string(value, "code")?,
		json_string(value, "message")?,
		value.get("metadata").cloned(),
	))
}


