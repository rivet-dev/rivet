use anyhow::{Context, Result, anyhow};
use serde_json::{Value as JsonValue, json};

use crate::EncodingKind;

use super::{to_client, to_server};

const CURRENT_VERSION: u16 = 3;

pub fn encode_to_server(
    encoding: EncodingKind,
    value: &to_server::ToServer,
) -> Result<Vec<u8>> {
    match encoding {
        EncodingKind::Json => Ok(serde_json::to_vec(&to_server_json_value(value)?)?),
        EncodingKind::Cbor => Ok(serde_cbor::to_vec(&to_server_json_value(value)?)?),
        EncodingKind::Bare => encode_to_server_bare(value),
    }
}

pub fn decode_to_client(
    encoding: EncodingKind,
    payload: &[u8],
) -> Result<to_client::ToClient> {
    match encoding {
        EncodingKind::Json => {
            let value: JsonValue = serde_json::from_slice(payload)
                .context("decode actor websocket json response")?;
            to_client_from_json_value(&value)
        }
        EncodingKind::Cbor => {
            let value: JsonValue = serde_cbor::from_slice(payload)
                .context("decode actor websocket cbor response")?;
            to_client_from_json_value(&value)
        }
        EncodingKind::Bare => decode_to_client_bare(payload),
    }
}

pub fn encode_http_action_request(
    encoding: EncodingKind,
    args: &[JsonValue],
) -> Result<Vec<u8>> {
    match encoding {
        EncodingKind::Json => Ok(serde_json::to_vec(&json!({ "args": args }))?),
        EncodingKind::Cbor => Ok(serde_cbor::to_vec(&json!({ "args": args }))?),
        EncodingKind::Bare => {
            let mut out = versioned();
            write_data(&mut out, &serde_cbor::to_vec(&args.to_vec())?);
            Ok(out)
        }
    }
}

pub fn decode_http_action_response(
    encoding: EncodingKind,
    payload: &[u8],
) -> Result<JsonValue> {
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
            let mut cursor = BareCursor::versioned(payload)?;
            let output = cursor.read_data().context("decode action response output")?;
            cursor.finish()?;
            Ok(serde_cbor::from_slice(&output)?)
        }
    }
}

pub fn encode_http_queue_request(
    encoding: EncodingKind,
    name: &str,
    body: &JsonValue,
    wait: bool,
    timeout: Option<u64>,
) -> Result<Vec<u8>> {
    match encoding {
        EncodingKind::Json => {
            let mut value = json!({ "name": name, "body": body, "wait": wait });
            if let Some(timeout) = timeout {
                value["timeout"] = json!(timeout);
            }
            Ok(serde_json::to_vec(&value)?)
        }
        EncodingKind::Cbor => {
            let mut value = json!({ "name": name, "body": body, "wait": wait });
            if let Some(timeout) = timeout {
                value["timeout"] = json!(timeout);
            }
            Ok(serde_cbor::to_vec(&value)?)
        }
        EncodingKind::Bare => {
            let mut out = versioned();
            write_data(&mut out, &serde_cbor::to_vec(body)?);
            write_optional_string(&mut out, Some(name));
            write_optional_bool(&mut out, Some(wait));
            write_optional_u64(&mut out, timeout);
            Ok(out)
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
            let mut cursor = BareCursor::versioned(payload)?;
            let status = cursor.read_string().context("decode queue status")?;
            let response = cursor
                .read_optional_data()
                .context("decode queue response")?
                .map(|payload| serde_cbor::from_slice(&payload))
                .transpose()?;
            cursor.finish()?;
            (status, response)
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
            let mut cursor = BareCursor::versioned(payload)?;
            let group = cursor.read_string().context("decode error group")?;
            let code = cursor.read_string().context("decode error code")?;
            let message = cursor.read_string().context("decode error message")?;
            let metadata = cursor
                .read_optional_data()
                .context("decode error metadata")?
                .map(|payload| serde_cbor::from_slice(&payload))
                .transpose()?;
            cursor.finish()?;
            Ok((group, code, message, metadata))
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
        "ActionResponse" => to_client::ToClientBody::ActionResponse(
            to_client::ActionResponse {
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
            },
        ),
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
    let mut out = versioned();
    match &value.body {
        to_server::ToServerBody::ActionRequest(request) => {
            out.push(0);
            write_uint(&mut out, request.id);
            write_string(&mut out, &request.name);
            write_data(&mut out, &request.args);
        }
        to_server::ToServerBody::SubscriptionRequest(request) => {
            out.push(1);
            write_string(&mut out, &request.event_name);
            write_bool(&mut out, request.subscribe);
        }
    }
    Ok(out)
}

fn decode_to_client_bare(payload: &[u8]) -> Result<to_client::ToClient> {
    let mut cursor = BareCursor::versioned(payload)?;
    let tag = cursor.read_u8().context("decode actor websocket tag")?;
    let body = match tag {
        0 => to_client::ToClientBody::Init(to_client::Init {
            actor_id: cursor.read_string().context("decode init actor id")?,
            connection_id: cursor.read_string().context("decode init connection id")?,
            connection_token: None,
        }),
        1 => to_client::ToClientBody::Error(to_client::Error {
            group: cursor.read_string().context("decode error group")?,
            code: cursor.read_string().context("decode error code")?,
            message: cursor.read_string().context("decode error message")?,
            metadata: cursor.read_optional_data().context("decode error metadata")?,
            action_id: cursor.read_optional_uint().context("decode error action id")?,
        }),
        2 => to_client::ToClientBody::ActionResponse(to_client::ActionResponse {
            id: cursor.read_uint().context("decode action response id")?,
            output: cursor.read_data().context("decode action response output")?,
        }),
        3 => to_client::ToClientBody::Event(to_client::Event {
            name: cursor.read_string().context("decode event name")?,
            args: cursor.read_data().context("decode event args")?,
        }),
        _ => return Err(anyhow!("unknown actor websocket response tag {tag}")),
    };
    cursor.finish()?;
    Ok(to_client::ToClient { body })
}

fn versioned() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&CURRENT_VERSION.to_le_bytes());
    out
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn write_uint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_data(out: &mut Vec<u8>, value: &[u8]) {
    write_uint(out, value.len() as u64);
    out.extend_from_slice(value);
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_data(out, value.as_bytes());
}

fn write_optional_string(out: &mut Vec<u8>, value: Option<&str>) {
    write_bool(out, value.is_some());
    if let Some(value) = value {
        write_string(out, value);
    }
}

fn write_optional_bool(out: &mut Vec<u8>, value: Option<bool>) {
    write_bool(out, value.is_some());
    if let Some(value) = value {
        write_bool(out, value);
    }
}

fn write_optional_u64(out: &mut Vec<u8>, value: Option<u64>) {
    write_bool(out, value.is_some());
    if let Some(value) = value {
        write_u64(out, value);
    }
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

fn error_from_json_value(
    value: &JsonValue,
) -> Result<(String, String, String, Option<JsonValue>)> {
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

struct BareCursor<'a> {
    payload: &'a [u8],
    offset: usize,
}

impl<'a> BareCursor<'a> {
    fn versioned(payload: &'a [u8]) -> Result<Self> {
        if payload.len() < 2 {
            return Err(anyhow!("payload too short for embedded version"));
        }
        let version = u16::from_le_bytes([payload[0], payload[1]]);
        if version != CURRENT_VERSION {
            return Err(anyhow!(
                "unsupported embedded version {version}; expected {CURRENT_VERSION}"
            ));
        }
        Ok(Self {
            payload: &payload[2..],
            offset: 0,
        })
    }

    fn finish(&self) -> Result<()> {
        if self.offset == self.payload.len() {
            Ok(())
        } else {
            Err(anyhow!("remaining bytes after bare decode"))
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        let value = *self
            .payload
            .get(self.offset)
            .ok_or_else(|| anyhow!("unexpected end of input"))?;
        self.offset += 1;
        Ok(value)
    }

    fn read_bool(&mut self) -> Result<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(anyhow!("invalid bool value {value}")),
        }
    }

    fn read_uint(&mut self) -> Result<u64> {
        let mut result = 0u64;
        let mut shift = 0u32;
        let mut byte_count = 0u8;
        loop {
            let byte = self.read_u8()?;
            byte_count += 1;
            result = result
                .checked_add(u64::from(byte & 0x7f) << shift)
                .ok_or_else(|| anyhow!("uint overflow"))?;
            if byte & 0x80 == 0 {
                if byte_count > 1 && byte == 0 {
                    return Err(anyhow!("non-canonical uint"));
                }
                return Ok(result);
            }
            shift += 7;
            if shift >= 64 || byte_count >= 10 {
                return Err(anyhow!("uint overflow"));
            }
        }
    }

    fn read_u64(&mut self) -> Result<u64> {
        let end = self.offset + 8;
        let bytes = self
            .payload
            .get(self.offset..end)
            .ok_or_else(|| anyhow!("unexpected end of input"))?;
        self.offset = end;
        Ok(u64::from_le_bytes(bytes.try_into()?))
    }

    fn read_data(&mut self) -> Result<Vec<u8>> {
        let len = usize::try_from(self.read_uint()?).context("bare data length overflow")?;
        let end = self.offset + len;
        let bytes = self
            .payload
            .get(self.offset..end)
            .ok_or_else(|| anyhow!("unexpected end of input"))?
            .to_vec();
        self.offset = end;
        Ok(bytes)
    }

    fn read_string(&mut self) -> Result<String> {
        String::from_utf8(self.read_data()?).context("bare string is not utf-8")
    }

    fn read_optional_data(&mut self) -> Result<Option<Vec<u8>>> {
        if self.read_bool()? {
            Ok(Some(self.read_data()?))
        } else {
            Ok(None)
        }
    }

    fn read_optional_uint(&mut self) -> Result<Option<u64>> {
        if self.read_bool()? {
            Ok(Some(self.read_uint()?))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    fn read_optional_u64(&mut self) -> Result<Option<u64>> {
        if self.read_bool()? {
            Ok(Some(self.read_u64()?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn bare_action_response_round_trips() {
        let mut payload = versioned();
        write_data(&mut payload, &serde_cbor::to_vec(&json!({ "ok": true })).unwrap());

        let output = decode_http_action_response(EncodingKind::Bare, &payload).unwrap();
        assert_eq!(output, json!({ "ok": true }));
    }

    #[test]
    fn bare_queue_request_has_embedded_version() {
        let payload = encode_http_queue_request(
            EncodingKind::Bare,
            "jobs",
            &json!({ "id": 1 }),
            true,
            Some(50),
        )
        .unwrap();
        assert_eq!(u16::from_le_bytes([payload[0], payload[1]]), CURRENT_VERSION);
    }
}
