use anyhow::{anyhow, Context, Result};
use rivetkit_client_protocol as wire;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use vbare::OwnedVersionedData;

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
		EncodingKind::Bare => {
			wire::versioned::HttpActionRequest::wrap_latest(wire::HttpActionRequest {
				args: serde_cbor::to_vec(&args.to_vec())?,
			})
			.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
		}
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
                <wire::versioned::HttpActionResponse as OwnedVersionedData>::deserialize_with_embedded_version(
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
	dedupe_key: Option<&str>,
	delay: Option<u64>,
) -> Result<Vec<u8>> {
	#[derive(Serialize)]
	struct JsonQueueRequest<'a, T: Serialize + ?Sized> {
		name: &'a str,
		body: &'a T,
		#[serde(rename = "dedupeKey", skip_serializing_if = "Option::is_none")]
		dedupe_key: Option<&'a str>,
		#[serde(skip_serializing_if = "Option::is_none")]
		delay: Option<u64>,
	}

	let request = JsonQueueRequest {
		name,
		body,
		dedupe_key,
		delay,
	};

	match encoding {
		EncodingKind::Json => Ok(serde_json::to_vec(&request)?),
		EncodingKind::Cbor => Ok(serde_cbor::to_vec(&request)?),
		EncodingKind::Bare => {
			wire::versioned::HttpQueueSendRequest::wrap_latest(wire::HttpQueueSendRequest {
				body: serde_cbor::to_vec(body)?,
				name: Some(name.to_owned()),
				dedupe_key: dedupe_key.map(str::to_owned),
				delay,
			})
			.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueSendReceipt {
	pub id: String,
	pub deduplicated: bool,
}

pub fn decode_http_queue_response(
	encoding: EncodingKind,
	payload: &[u8],
) -> Result<QueueSendReceipt> {
	let (id, deduplicated) = match encoding {
		EncodingKind::Json => {
			let value: JsonValue = serde_json::from_slice(payload)?;
			let id = value
				.get("receiptId")
				.and_then(JsonValue::as_str)
				.ok_or_else(|| anyhow!("queue response missing receiptId"))?
				.to_owned();
			let deduplicated = value
				.get("deduplicated")
				.and_then(JsonValue::as_bool)
				.ok_or_else(|| anyhow!("queue response missing deduplicated"))?;
			(id, deduplicated)
		}
		EncodingKind::Cbor => {
			let value: JsonValue = serde_cbor::from_slice(payload)?;
			let id = value
				.get("receiptId")
				.and_then(JsonValue::as_str)
				.ok_or_else(|| anyhow!("queue response missing receiptId"))?
				.to_owned();
			let deduplicated = value
				.get("deduplicated")
				.and_then(JsonValue::as_bool)
				.ok_or_else(|| anyhow!("queue response missing deduplicated"))?;
			(id, deduplicated)
		}
		EncodingKind::Bare => {
			let response =
                <wire::versioned::HttpQueueSendResponse as OwnedVersionedData>::deserialize_with_embedded_version(
                    payload,
                )
                .context("decode bare queue response")?;
			(response.receipt_id, response.deduplicated)
		}
	};
	Ok(QueueSendReceipt { id, deduplicated })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueMessageStatus {
	Queued {
		attempts: u64,
		created_at: u64,
	},
	Delayed {
		attempts: u64,
		created_at: u64,
		available_at: u64,
	},
	Processing {
		attempts: u64,
		created_at: u64,
		started_at: u64,
	},
	Retrying {
		attempts: u64,
		created_at: u64,
		available_at: u64,
	},
	Succeeded {
		attempts: u64,
		created_at: u64,
		completed_at: u64,
	},
	DeadLettered {
		attempts: u64,
		created_at: u64,
		failed_at: u64,
	},
	Consumed {
		created_at: u64,
		consumed_at: u64,
	},
	Unknown,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawQueueMessageStatus {
	state: String,
	attempts: Option<u64>,
	created_at: Option<u64>,
	available_at: Option<u64>,
	started_at: Option<u64>,
	completed_at: Option<u64>,
	failed_at: Option<u64>,
	consumed_at: Option<u64>,
}

pub fn decode_http_queue_status(
	encoding: EncodingKind,
	payload: &[u8],
) -> Result<QueueMessageStatus> {
	let status = match encoding {
		EncodingKind::Json => serde_json::from_slice(payload)?,
		EncodingKind::Cbor => serde_cbor::from_slice(payload)?,
		EncodingKind::Bare => {
			let response = <wire::versioned::HttpQueueStatusResponse as OwnedVersionedData>::deserialize_with_embedded_version(payload)
				.context("decode bare queue status response")?;
			RawQueueMessageStatus {
				state: response.state,
				attempts: response.attempts,
				created_at: response.created_at,
				available_at: response.available_at,
				started_at: response.started_at,
				completed_at: response.completed_at,
				failed_at: response.failed_at,
				consumed_at: response.consumed_at,
			}
		}
	};
	queue_message_status_from_raw(status)
}

fn queue_message_status_from_raw(status: RawQueueMessageStatus) -> Result<QueueMessageStatus> {
	let required = |value: Option<u64>, field: &str| {
		value.ok_or_else(|| anyhow!("queue status `{}` missing {field}", status.state))
	};
	let attempts = || required(status.attempts, "attempts");
	let created_at = || required(status.created_at, "createdAt");
	match status.state.as_str() {
		"queued" => Ok(QueueMessageStatus::Queued {
			attempts: attempts()?,
			created_at: created_at()?,
		}),
		"delayed" => Ok(QueueMessageStatus::Delayed {
			attempts: attempts()?,
			created_at: created_at()?,
			available_at: required(status.available_at, "availableAt")?,
		}),
		"processing" => Ok(QueueMessageStatus::Processing {
			attempts: attempts()?,
			created_at: created_at()?,
			started_at: required(status.started_at, "startedAt")?,
		}),
		"retrying" => Ok(QueueMessageStatus::Retrying {
			attempts: attempts()?,
			created_at: created_at()?,
			available_at: required(status.available_at, "availableAt")?,
		}),
		"succeeded" => Ok(QueueMessageStatus::Succeeded {
			attempts: attempts()?,
			created_at: created_at()?,
			completed_at: required(status.completed_at, "completedAt")?,
		}),
		"deadLettered" => Ok(QueueMessageStatus::DeadLettered {
			attempts: attempts()?,
			created_at: created_at()?,
			failed_at: required(status.failed_at, "failedAt")?,
		}),
		"consumed" => Ok(QueueMessageStatus::Consumed {
			created_at: created_at()?,
			consumed_at: required(status.consumed_at, "consumedAt")?,
		}),
		"unknown" => Ok(QueueMessageStatus::Unknown),
		state => Err(anyhow!("unknown queue status state `{state}`")),
	}
}

pub fn decode_http_error(
	encoding: EncodingKind,
	payload: &[u8],
) -> Result<(String, String, String, Option<JsonValue>)> {
	let decoded = match encoding {
		EncodingKind::Json => (|| {
			let value: JsonValue = serde_json::from_slice(payload)?;
			error_from_json_value(&value)
		})(),
		EncodingKind::Cbor => (|| {
			let value: JsonValue = serde_cbor::from_slice(payload)?;
			error_from_json_value(&value)
		})(),
		EncodingKind::Bare => (|| {
			let error =
                <wire::versioned::HttpResponseError as OwnedVersionedData>::deserialize_with_embedded_version(
                    payload,
                )
                .context("decode bare http error")?;
			let metadata = error
				.metadata
				.map(|payload| serde_cbor::from_slice(&payload))
				.transpose()?;
			Ok((error.group, error.code, error.message, metadata))
		})(),
	};

	if decoded.is_ok() || encoding == EncodingKind::Json {
		return decoded;
	}

	// Engine-edge failures (for example actor wake or routing failures) are
	// JSON because they are produced before the request reaches an actor that
	// can honor its requested encoding.
	let value: JsonValue = serde_json::from_slice(payload)?;
	error_from_json_value(&value)
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
				id: serde_bare::Uint(request.id),
				name: request.name.clone(),
				args: request.args.clone(),
			})
		}
		to_server::ToServerBody::SubscriptionRequest(request) => {
			wire::ToServerBody::SubscriptionRequest(wire::SubscriptionRequest {
				event_name: request.event_name.clone(),
				subscribe: request.subscribe,
			})
		}
	};

	wire::versioned::ToServer::wrap_latest(wire::ToServer { body })
		.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
}

fn decode_to_client_bare(payload: &[u8]) -> Result<to_client::ToClient> {
	let message =
		<wire::versioned::ToClient as OwnedVersionedData>::deserialize_with_embedded_version(
			payload,
		)
		.context("decode bare actor websocket response")?;

	let body = match message.body {
		wire::ToClientBody::Init(init) => to_client::ToClientBody::Init(to_client::Init {
			actor_id: init.actor_id,
			connection_id: init.connection_id,
			connection_token: None,
		}),
		wire::ToClientBody::Error(error) => to_client::ToClientBody::Error(to_client::Error {
			group: error.group,
			code: error.code,
			message: error.message,
			metadata: error.metadata,
			action_id: error.action_id.map(|id| id.0),
		}),
		wire::ToClientBody::ActionResponse(response) => {
			to_client::ToClientBody::ActionResponse(to_client::ActionResponse {
				id: response.id.0,
				output: response.output,
			})
		}
		wire::ToClientBody::Event(event) => to_client::ToClientBody::Event(to_client::Event {
			name: event.name,
			args: event.args,
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

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn bare_action_response_round_trips() {
		let payload = wire::versioned::HttpActionResponse::wrap_latest(wire::HttpActionResponse {
			output: serde_cbor::to_vec(&json!({ "ok": true })).unwrap(),
		})
		.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
		.unwrap();

		let output = decode_http_action_response(EncodingKind::Bare, &payload).unwrap();
		assert_eq!(output, json!({ "ok": true }));
	}

	#[test]
	fn bare_queue_request_has_embedded_version() {
		let payload = encode_http_queue_request(
			EncodingKind::Bare,
			"jobs",
			&json!({ "id": 1 }),
			Some("request-1"),
			Some(50),
		)
		.unwrap();
		assert_eq!(
			u16::from_le_bytes([payload[0], payload[1]]),
			wire::PROTOCOL_VERSION
		);
	}

	#[test]
	fn bare_request_accepts_json_engine_error() {
		let payload = serde_json::to_vec(&json!({
			"group": "guard",
			"code": "actor_wake_retries_exceeded",
			"message": "actor stopped before becoming ready",
			"metadata": { "wake_retries": 8 },
		}))
		.unwrap();

		let error = decode_http_error(EncodingKind::Bare, &payload).unwrap();
		assert_eq!(error.0, "guard");
		assert_eq!(error.1, "actor_wake_retries_exceeded");
		assert_eq!(error.2, "actor stopped before becoming ready");
		assert_eq!(error.3, Some(json!({ "wake_retries": 8 })));
	}

	#[test]
	fn queue_status_rejects_missing_state_fields() {
		let payload = serde_json::to_vec(&json!({
			"state": "succeeded",
			"attempts": 1,
			"createdAt": 10,
		}))
		.unwrap();

		let error = decode_http_queue_status(EncodingKind::Json, &payload).unwrap_err();
		assert!(error.to_string().contains("missing completedAt"));
	}

	#[test]
	fn queue_status_accepts_unknown_without_metadata() {
		let payload = serde_json::to_vec(&json!({ "state": "unknown" })).unwrap();

		assert_eq!(
			decode_http_queue_status(EncodingKind::Json, &payload).unwrap(),
			QueueMessageStatus::Unknown,
		);
	}
}
