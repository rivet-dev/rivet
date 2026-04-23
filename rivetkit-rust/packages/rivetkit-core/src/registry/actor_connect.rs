use super::inspector::{decode_cbor_json, encode_json_as_cbor};
use super::*;
use crate::error::ProtocolError;

pub(super) fn send_inspector_message(
	sender: &WebSocketSender,
	message: &InspectorServerMessage,
) -> Result<()> {
	let payload = inspector_protocol::encode_server_message(message)?;
	sender.send(payload, true);
	Ok(())
}

pub(super) fn send_actor_connect_message(
	sender: &WebSocketSender,
	encoding: ActorConnectEncoding,
	message: &ActorConnectToClient,
	max_outgoing_message_size: usize,
) -> std::result::Result<(), ActorConnectSendError> {
	match encoding {
		ActorConnectEncoding::Json => {
			let payload = encode_actor_connect_message_json(message)
				.map_err(ActorConnectSendError::Encode)?;
			if payload.len() > max_outgoing_message_size {
				return Err(ActorConnectSendError::OutgoingTooLong);
			}
			sender.send_text(&payload);
		}
		ActorConnectEncoding::Cbor => {
			let payload = encode_actor_connect_message_cbor(message)
				.map_err(ActorConnectSendError::Encode)?;
			if payload.len() > max_outgoing_message_size {
				return Err(ActorConnectSendError::OutgoingTooLong);
			}
			sender.send(payload, true);
		}
		ActorConnectEncoding::Bare => {
			let payload =
				encode_actor_connect_message(message).map_err(ActorConnectSendError::Encode)?;
			if payload.len() > max_outgoing_message_size {
				return Err(ActorConnectSendError::OutgoingTooLong);
			}
			sender.send(payload, true);
		}
	}
	Ok(())
}

pub(super) fn encode_actor_connect_message(message: &ActorConnectToClient) -> Result<Vec<u8>> {
	let body = match message {
		ActorConnectToClient::Init(payload) => {
			client_protocol::ToClientBody::Init(client_protocol::Init {
				actor_id: payload.actor_id.clone(),
				connection_id: payload.connection_id.clone(),
			})
		}
		ActorConnectToClient::Error(payload) => {
			client_protocol::ToClientBody::Error(client_protocol::Error {
				group: payload.group.clone(),
				code: payload.code.clone(),
				message: payload.message.clone(),
				metadata: payload
					.metadata
					.as_ref()
					.map(|metadata| metadata.as_ref().to_vec()),
				action_id: payload.action_id.map(serde_bare::Uint),
			})
		}
		ActorConnectToClient::ActionResponse(payload) => {
			client_protocol::ToClientBody::ActionResponse(client_protocol::ActionResponse {
				id: serde_bare::Uint(payload.id),
				output: payload.output.as_ref().to_vec(),
			})
		}
		ActorConnectToClient::Event(payload) => {
			client_protocol::ToClientBody::Event(client_protocol::Event {
				name: payload.name.clone(),
				args: payload.args.as_ref().to_vec(),
			})
		}
	};

	client_protocol::versioned::ToClient::wrap_latest(client_protocol::ToClient { body })
		.serialize_with_embedded_version(client_protocol::PROTOCOL_VERSION)
}

pub(super) fn encode_actor_connect_message_json(message: &ActorConnectToClient) -> Result<String> {
	serde_json::to_string(&actor_connect_message_json_value(message)?)
		.context("encode actor websocket message as json")
}

pub(super) fn encode_actor_connect_message_cbor(message: &ActorConnectToClient) -> Result<Vec<u8>> {
	encode_actor_connect_message_cbor_manual(message)
}

pub(super) fn actor_connect_message_json_value(
	message: &ActorConnectToClient,
) -> Result<JsonValue> {
	let body = match message {
		ActorConnectToClient::Init(payload) => json!({
			"tag": "Init",
			"val": {
				"actorId": payload.actor_id.clone(),
				"connectionId": payload.connection_id.clone(),
			},
		}),
		ActorConnectToClient::Error(payload) => {
			let mut value = serde_json::Map::from_iter([
				("group".to_owned(), JsonValue::String(payload.group.clone())),
				("code".to_owned(), JsonValue::String(payload.code.clone())),
				(
					"message".to_owned(),
					JsonValue::String(payload.message.clone()),
				),
			]);
			if let Some(metadata) = payload.metadata.as_ref() {
				value.insert("metadata".to_owned(), decode_cbor_json(metadata.as_ref())?);
			}
			value.insert(
				"actionId".to_owned(),
				payload
					.action_id
					.map(json_compat_bigint)
					.unwrap_or(JsonValue::Null),
			);
			JsonValue::Object(serde_json::Map::from_iter([
				("tag".to_owned(), JsonValue::String("Error".to_owned())),
				("val".to_owned(), JsonValue::Object(value)),
			]))
		}
		ActorConnectToClient::ActionResponse(payload) => json!({
			"tag": "ActionResponse",
			"val": {
				"id": json_compat_bigint(payload.id),
				"output": decode_cbor_json(payload.output.as_ref())?,
			},
		}),
		ActorConnectToClient::Event(payload) => json!({
			"tag": "Event",
			"val": {
				"name": payload.name.clone(),
				"args": decode_cbor_json(payload.args.as_ref())?,
			},
		}),
	};
	Ok(json!({ "body": body }))
}

pub(super) fn decode_actor_connect_message(
	payload: &[u8],
	encoding: ActorConnectEncoding,
) -> Result<ActorConnectToServer> {
	match encoding {
		ActorConnectEncoding::Json => {
			let envelope: JsonValue =
				serde_json::from_slice(payload).context("decode actor websocket json request")?;
			actor_connect_request_from_json_value(&envelope)
		}
		ActorConnectEncoding::Cbor => {
			let envelope: ActorConnectToServerJsonEnvelope =
				ciborium::from_reader(Cursor::new(payload))
					.context("decode actor websocket cbor request")?;
			actor_connect_request_from_json(envelope)
		}
		ActorConnectEncoding::Bare => decode_actor_connect_message_bare(payload),
	}
}

pub(super) fn actor_connect_request_from_json(
	envelope: ActorConnectToServerJsonEnvelope,
) -> Result<ActorConnectToServer> {
	match envelope.body {
		ActorConnectToServerJsonBody::ActionRequest(request) => Ok(
			ActorConnectToServer::ActionRequest(ActorConnectActionRequest {
				id: request.id,
				name: request.name,
				args: ByteBuf::from(
					encode_json_as_cbor(&request.args)
						.context("encode actor websocket action request args")?,
				),
			}),
		),
		ActorConnectToServerJsonBody::SubscriptionRequest(request) => {
			Ok(ActorConnectToServer::SubscriptionRequest(request))
		}
	}
}

pub(super) fn actor_connect_request_from_json_value(
	envelope: &JsonValue,
) -> Result<ActorConnectToServer> {
	let body = envelope
		.get("body")
		.and_then(JsonValue::as_object)
		.ok_or_else(|| invalid_actor_connect("body", "missing object"))?;
	let tag = body
		.get("tag")
		.and_then(JsonValue::as_str)
		.ok_or_else(|| invalid_actor_connect("tag", "missing string"))?;
	let value = body
		.get("val")
		.and_then(JsonValue::as_object)
		.ok_or_else(|| invalid_actor_connect("val", "missing object"))?;

	match tag {
		"ActionRequest" => Ok(ActorConnectToServer::ActionRequest(
			ActorConnectActionRequest {
				id: parse_json_compat_u64(
					value
						.get("id")
						.ok_or_else(|| invalid_actor_connect("id", "missing value"))?,
				)?,
				name: value
					.get("name")
					.and_then(JsonValue::as_str)
					.ok_or_else(|| invalid_actor_connect("name", "missing string"))?
					.to_owned(),
				args: ByteBuf::from(encode_json_as_cbor(
					value
						.get("args")
						.ok_or_else(|| invalid_actor_connect("args", "missing value"))?,
				)?),
			},
		)),
		"SubscriptionRequest" => Ok(ActorConnectToServer::SubscriptionRequest(
			ActorConnectSubscriptionRequest {
				event_name: value
					.get("eventName")
					.and_then(JsonValue::as_str)
					.ok_or_else(|| invalid_actor_connect("eventName", "missing string"))?
					.to_owned(),
				subscribe: value
					.get("subscribe")
					.and_then(JsonValue::as_bool)
					.ok_or_else(|| invalid_actor_connect("subscribe", "missing boolean"))?,
			},
		)),
		other => Err(invalid_actor_connect(
			"tag",
			format!("unknown tag `{other}`"),
		)),
	}
}

pub(super) fn json_compat_bigint(value: u64) -> JsonValue {
	JsonValue::Array(vec![
		JsonValue::String("$BigInt".to_owned()),
		JsonValue::String(value.to_string()),
	])
}

pub(super) fn parse_json_compat_u64(value: &JsonValue) -> Result<u64> {
	match value {
		JsonValue::Number(number) => number
			.as_u64()
			.ok_or_else(|| invalid_actor_connect("bigint", "not an unsigned integer")),
		JsonValue::Array(values) if values.len() == 2 => {
			let tag = values[0]
				.as_str()
				.ok_or_else(|| invalid_actor_connect("bigint tag", "not a string"))?;
			let raw = values[1]
				.as_str()
				.ok_or_else(|| invalid_actor_connect("bigint value", "not a string"))?;
			if tag != "$BigInt" {
				return Err(invalid_actor_connect(
					"bigint tag",
					format!("unsupported compat tag `{tag}`"),
				));
			}
			raw.parse::<u64>()
				.context("parse actor websocket json bigint")
		}
		_ => Err(invalid_actor_connect("bigint", "invalid value")),
	}
}

fn invalid_actor_connect(field: &str, reason: impl Into<String>) -> anyhow::Error {
	ProtocolError::InvalidActorConnectRequest {
		field: field.to_owned(),
		reason: reason.into(),
	}
	.build()
}

pub(super) fn encode_actor_connect_message_cbor_manual(
	message: &ActorConnectToClient,
) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	cbor_write_map_len(&mut encoded, 1);
	cbor_write_string(&mut encoded, "body");

	match message {
		ActorConnectToClient::Init(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "Init");
			cbor_write_string(&mut encoded, "val");
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "actorId");
			cbor_write_string(&mut encoded, &payload.actor_id);
			cbor_write_string(&mut encoded, "connectionId");
			cbor_write_string(&mut encoded, &payload.connection_id);
		}
		ActorConnectToClient::Error(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "Error");
			cbor_write_string(&mut encoded, "val");
			let mut field_count = 3usize;
			if payload.metadata.is_some() {
				field_count += 1;
			}
			field_count += 1;
			cbor_write_map_len(&mut encoded, field_count);
			cbor_write_string(&mut encoded, "group");
			cbor_write_string(&mut encoded, &payload.group);
			cbor_write_string(&mut encoded, "code");
			cbor_write_string(&mut encoded, &payload.code);
			cbor_write_string(&mut encoded, "message");
			cbor_write_string(&mut encoded, &payload.message);
			if let Some(metadata) = payload.metadata.as_ref() {
				cbor_write_string(&mut encoded, "metadata");
				encoded.extend_from_slice(metadata.as_ref());
			}
			if let Some(action_id) = payload.action_id {
				cbor_write_string(&mut encoded, "actionId");
				cbor_write_u64_force_64(&mut encoded, action_id);
			} else {
				cbor_write_string(&mut encoded, "actionId");
				cbor_write_null(&mut encoded);
			}
		}
		ActorConnectToClient::ActionResponse(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "ActionResponse");
			cbor_write_string(&mut encoded, "val");
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "id");
			cbor_write_u64_force_64(&mut encoded, payload.id);
			cbor_write_string(&mut encoded, "output");
			encoded.extend_from_slice(payload.output.as_ref());
		}
		ActorConnectToClient::Event(payload) => {
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "tag");
			cbor_write_string(&mut encoded, "Event");
			cbor_write_string(&mut encoded, "val");
			cbor_write_map_len(&mut encoded, 2);
			cbor_write_string(&mut encoded, "name");
			cbor_write_string(&mut encoded, &payload.name);
			cbor_write_string(&mut encoded, "args");
			encoded.extend_from_slice(payload.args.as_ref());
		}
	}

	Ok(encoded)
}

pub(super) fn decode_actor_connect_message_bare(payload: &[u8]) -> Result<ActorConnectToServer> {
	let message =
		<client_protocol::versioned::ToServer as OwnedVersionedData>::deserialize_with_embedded_version(
			payload,
		)
		.context("decode actor websocket bare request")?;

	match message.body {
		client_protocol::ToServerBody::ActionRequest(request) => Ok(
			ActorConnectToServer::ActionRequest(ActorConnectActionRequest {
				id: request.id.0,
				name: request.name,
				args: ByteBuf::from(request.args),
			}),
		),
		client_protocol::ToServerBody::SubscriptionRequest(request) => Ok(
			ActorConnectToServer::SubscriptionRequest(ActorConnectSubscriptionRequest {
				event_name: request.event_name,
				subscribe: request.subscribe,
			}),
		),
	}
}

pub(super) fn cbor_write_type_and_len(buffer: &mut Vec<u8>, major: u8, len: usize) {
	match len {
		0..=23 => buffer.push((major << 5) | (len as u8)),
		24..=0xff => {
			buffer.push((major << 5) | 24);
			buffer.push(len as u8);
		}
		0x100..=0xffff => {
			buffer.push((major << 5) | 25);
			buffer.extend_from_slice(&(len as u16).to_be_bytes());
		}
		0x1_0000..=0xffff_ffff => {
			buffer.push((major << 5) | 26);
			buffer.extend_from_slice(&(len as u32).to_be_bytes());
		}
		_ => {
			buffer.push((major << 5) | 27);
			buffer.extend_from_slice(&(len as u64).to_be_bytes());
		}
	}
}

pub(super) fn cbor_write_map_len(buffer: &mut Vec<u8>, len: usize) {
	cbor_write_type_and_len(buffer, 5, len);
}

pub(super) fn cbor_write_string(buffer: &mut Vec<u8>, value: &str) {
	cbor_write_type_and_len(buffer, 3, value.len());
	buffer.extend_from_slice(value.as_bytes());
}

pub(super) fn cbor_write_u64_force_64(buffer: &mut Vec<u8>, value: u64) {
	buffer.push(0x1b);
	buffer.extend_from_slice(&value.to_be_bytes());
}

pub(super) fn cbor_write_null(buffer: &mut Vec<u8>) {
	buffer.push(0xf6);
}

pub(super) fn action_dispatch_error_response(
	error: ActionDispatchError,
	action_id: u64,
) -> ActorConnectError {
	let metadata = error
		.metadata
		.as_ref()
		.and_then(|metadata| encode_json_as_cbor(metadata).ok().map(ByteBuf::from));
	ActorConnectError {
		group: error.group,
		code: error.code,
		message: error.message,
		metadata,
		action_id: Some(action_id),
	}
}
