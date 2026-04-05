use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use gas::prelude::*;
use rivet_types::actors::CrashPolicy;
use serde::Deserialize;

use super::matrix_param_deserializer::{MatrixParamDeserializer, MatrixParamValue};
use crate::errors;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorPathInfo {
	pub actor_id: String,
	pub token: Option<String>,
	pub stripped_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryActorPathInfo {
	pub query: QueryActorQuery,
	pub token: Option<String>,
	pub stripped_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryActorQuery {
	Get {
		namespace: String,
		name: String,
		key: Vec<String>,
	},
	GetOrCreate {
		namespace: String,
		name: String,
		runner_name: String,
		key: Vec<String>,
		input: Option<Vec<u8>>,
		region: Option<String>,
		crash_policy: Option<CrashPolicy>,
	},
}

#[derive(Debug, Clone)]
pub enum ParsedActorPath {
	Direct(ActorPathInfo),
	Query(QueryActorPathInfo),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum GatewayQueryMethod {
	Get,
	GetOrCreate,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GatewayQueryPathParams {
	namespace: String,
	name: String,
	method: GatewayQueryMethod,
	runner_name: Option<String>,
	key: Option<Vec<String>>,
	input: Option<String>,
	region: Option<String>,
	token: Option<String>,
	crash_policy: Option<CrashPolicy>,
}


/// Parse actor routing information from path.
/// Matches patterns:
/// - /gateway/{actor_id}/{...path}
/// - /gateway/{actor_id}@{token}/{...path}
/// - /gateway/{name};namespace={namespace};method={method};.../{...path}
///
/// Returns `Ok(None)` for paths that are not gateway paths or for malformed
/// direct paths (backwards-compatible silent fallthrough). Returns `Err` for
/// malformed query paths so clients receive actionable error messages.
pub fn parse_actor_path(path: &str) -> Result<Option<ParsedActorPath>> {
	let query_pos = path.find('?');
	let fragment_pos = path.find('#');

	let query_string = match (query_pos, fragment_pos) {
		(Some(q), Some(f)) if q < f => &path[q..f],
		(Some(q), None) => &path[q..],
		_ => "",
	};

	let base_path = match query_pos {
		Some(pos) => &path[..pos],
		None => match fragment_pos {
			Some(pos) => &path[..pos],
			None => path,
		},
	};

	if base_path.contains("//") {
		return Ok(None);
	}

	let segments: Vec<&str> = base_path.split('/').filter(|segment| !segment.is_empty()).collect();
	if segments.first().copied() != Some("gateway") {
		return Ok(None);
	}

	if let Some(name_segment) = segments.get(1).copied() {
		if name_segment.contains(';') {
			return parse_query_actor_path(name_segment, base_path, query_string)
				.map(|path| Some(ParsedActorPath::Query(path)));
		}
	}

	Ok(parse_direct_actor_path(base_path, query_string))
}

fn parse_direct_actor_path(base_path: &str, query_string: &str) -> Option<ParsedActorPath> {
	let segments: Vec<&str> = base_path.split('/').filter(|segment| !segment.is_empty()).collect();
	if segments.len() < 2 || segments[0] != "gateway" {
		return None;
	}

	let actor_segment = segments[1];
	if actor_segment.is_empty() {
		return None;
	}

	let (actor_id, token) = if let Some(at_pos) = actor_segment.find('@') {
		let raw_actor_id = &actor_segment[..at_pos];
		let raw_token = &actor_segment[at_pos + 1..];
		if raw_actor_id.is_empty() || raw_token.is_empty() {
			return None;
		}

		let actor_id = strict_percent_decode(raw_actor_id).ok()?;
		let token = strict_percent_decode(raw_token).ok()?;
		(actor_id, Some(token))
	} else {
		(strict_percent_decode(actor_segment).ok()?, None)
	};

	Some(ParsedActorPath::Direct(ActorPathInfo {
		actor_id,
		token,
		stripped_path: build_remaining_path(base_path, query_string, 2),
	}))
}

fn parse_query_actor_path(
	name_segment: &str,
	base_path: &str,
	query_string: &str,
) -> Result<QueryActorPathInfo> {
	if name_segment.contains('@') {
		return Err(errors::QueryPathTokenSyntax.build());
	}

	let params = parse_query_gateway_params(name_segment)?;
	let stripped_path = build_remaining_path(base_path, query_string, 2);

	Ok(QueryActorPathInfo {
		token: params.token.clone(),
		query: build_actor_query_from_gateway_params(params)?,
		stripped_path,
	})
}

fn parse_query_gateway_params(name_segment: &str) -> Result<GatewayQueryPathParams> {
	let params = GatewayQueryPathParams::deserialize(build_matrix_param_deserializer(
		name_segment,
	)?)
	.map_err(|err| {
		errors::QueryInvalidParams {
			detail: err.to_string(),
		}
		.build()
	})?;

	if matches!(params.method, GatewayQueryMethod::Get)
		&& (params.input.is_some() || params.region.is_some() || params.crash_policy.is_some() || params.runner_name.is_some())
	{
		return Err(errors::QueryGetDisallowedParams.build());
	}

	if matches!(params.method, GatewayQueryMethod::GetOrCreate) && params.runner_name.is_none() {
		return Err(errors::QueryMissingRunnerName.build());
	}

	Ok(params)
}

fn build_actor_query_from_gateway_params(params: GatewayQueryPathParams) -> Result<QueryActorQuery> {
	let key = params.key.unwrap_or_default();
	let input = params
		.input
		.as_deref()
		.map(decode_query_input)
		.transpose()?;

	let query = match params.method {
		GatewayQueryMethod::Get => QueryActorQuery::Get {
			namespace: params.namespace,
			name: params.name,
			key,
		},
		GatewayQueryMethod::GetOrCreate => QueryActorQuery::GetOrCreate {
			namespace: params.namespace,
			name: params.name,
			runner_name: params.runner_name.expect("runner_name validated as required for getOrCreate"),
			key,
			input,
			region: params.region,
			crash_policy: params.crash_policy,
		},
	};

	Ok(query)
}

/// Parse a name segment with matrix params into a `MatrixParamDeserializer`.
/// The segment format is `{name};param1=value1;param2=value2`.
fn build_matrix_param_deserializer(name_segment: &str) -> Result<MatrixParamDeserializer> {
	let mut parts = name_segment.splitn(2, ';');
	let raw_name = parts.next().unwrap_or("");
	let params_str = parts.next().unwrap_or("");

	let decoded_name = decode_matrix_param_value(raw_name, "name")?;
	if decoded_name.is_empty() {
		return Err(errors::QueryEmptyActorName.build());
	}

	let mut entries = vec![("name".to_string(), MatrixParamValue::String(decoded_name))];

	if !params_str.is_empty() {
		for raw_param in params_str.split(';') {
			let Some(equals_pos) = raw_param.find('=') else {
				return Err(errors::QueryParamMissingEquals {
					param: raw_param.to_string(),
				}
				.build());
			};

			let name = &raw_param[..equals_pos];
			let raw_value = &raw_param[equals_pos + 1..];

			if name == "name" {
				return Err(errors::QueryDuplicateParam {
					name: name.to_string(),
				}
				.build());
			}

			if !is_query_gateway_param_name(name) {
				return Err(errors::QueryUnknownParam {
					name: name.to_string(),
				}
				.build());
			}

			if entries.iter().any(|(existing_name, _)| existing_name == name) {
				return Err(errors::QueryDuplicateParam {
					name: name.to_string(),
				}
				.build());
			}

			entries.push((
				name.to_string(),
				parse_query_gateway_param_value(name, raw_value)?,
			));
		}
	}

	Ok(MatrixParamDeserializer { entries })
}

fn is_query_gateway_param_name(name: &str) -> bool {
	matches!(
		name,
		"namespace" | "method" | "runnerName" | "key" | "input" | "region" | "token" | "crashPolicy"
	)
}

fn parse_query_gateway_param_value(name: &str, raw_value: &str) -> Result<MatrixParamValue> {
	match name {
		"key" => Ok(MatrixParamValue::Seq(
			raw_value
				.split(',')
				.map(|component| decode_matrix_param_value(component, name))
				.collect::<Result<Vec<_>>>()?,
		)),
		_ => Ok(MatrixParamValue::String(decode_matrix_param_value(
			raw_value, name,
		)?)),
	}
}

fn decode_query_input(value: &str) -> Result<Vec<u8>> {
	if !value
		.bytes()
		.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
		|| value.len() % 4 == 1
	{
		return Err(errors::QueryInvalidBase64Input.build());
	}

	let bytes = URL_SAFE_NO_PAD
		.decode(value.as_bytes())
		.map_err(|_| errors::QueryInvalidBase64Input.build())?;

	validate_cbor(&bytes).map_err(|err| {
		errors::QueryInvalidCborInput {
			detail: err.to_string(),
		}
		.build()
	})?;

	Ok(bytes)
}

fn decode_matrix_param_value(raw_value: &str, name: &str) -> Result<String> {
	strict_percent_decode(raw_value).map_err(|_| {
		errors::QueryInvalidPercentEncoding {
			name: name.to_string(),
		}
		.build()
	})
}

fn strict_percent_decode(raw_value: &str) -> Result<String> {
	let bytes = raw_value.as_bytes();
	let mut decoded = Vec::with_capacity(bytes.len());
	let mut idx = 0;

	while idx < bytes.len() {
		if bytes[idx] == b'%' {
			if idx + 2 >= bytes.len() {
				bail!("incomplete percent-encoding");
			}

			let hi = decode_hex(bytes[idx + 1]).context("invalid percent-encoding")?;
			let lo = decode_hex(bytes[idx + 2]).context("invalid percent-encoding")?;
			decoded.push((hi << 4) | lo);
			idx += 3;
		} else {
			decoded.push(bytes[idx]);
			idx += 1;
		}
	}

	String::from_utf8(decoded).context("invalid utf-8 in percent-encoding")
}

fn decode_hex(byte: u8) -> Option<u8> {
	match byte {
		b'0'..=b'9' => Some(byte - b'0'),
		b'a'..=b'f' => Some(byte - b'a' + 10),
		b'A'..=b'F' => Some(byte - b'A' + 10),
		_ => None,
	}
}

fn build_remaining_path(base_path: &str, query_string: &str, consumed_segments: usize) -> String {
	let segments: Vec<&str> = base_path.split('/').filter(|segment| !segment.is_empty()).collect();

	let mut prefix_len = 0;
	for segment in segments.iter().take(consumed_segments) {
		prefix_len += 1 + segment.len();
	}

	let remaining_base = if prefix_len < base_path.len() {
		&base_path[prefix_len..]
	} else {
		"/"
	};

	if remaining_base.is_empty() || !remaining_base.starts_with('/') {
		format!("/{remaining_base}{query_string}")
	} else {
		format!("{remaining_base}{query_string}")
	}
}

fn validate_cbor(bytes: &[u8]) -> Result<()> {
	ciborium::from_reader::<ciborium::Value, _>(bytes).context("invalid cbor")?;
	Ok(())
}
