use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use gas::prelude::*;
use rivet_types::actors::CrashPolicy;
use serde::Deserialize;

use crate::errors;

/// The `rvt-` query parameter prefix is reserved for Rivet gateway routing.
/// All query parameters with this prefix are stripped before forwarding
/// requests to the actor, so actors will never see them.
const RVT_PREFIX: &str = "rvt-";

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
		pool_name: String,
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

/// Parsed rvt-* query parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RvtParams {
	namespace: String,
	method: String,
	#[serde(default)]
	runner: Option<String>,
	#[serde(default)]
	pool: Option<String>,
	#[serde(default)]
	key: Option<String>,
	#[serde(default)]
	input: Option<String>,
	#[serde(default)]
	region: Option<String>,
	#[serde(default, rename = "crash-policy")]
	crash_policy: Option<String>,
	#[serde(default)]
	token: Option<String>,
}

/// Parse actor routing information from path.
/// Matches patterns:
/// - /gateway/{actor_id}/{...path}
/// - /gateway/{actor_id}@{token}/{...path}
/// - /gateway/{name}/{...path}?rvt-namespace=...&rvt-method=...
///
/// Returns `Ok(None)` for paths that are not gateway paths or for malformed
/// direct paths (backwards-compatible silent fallthrough). Returns `Err` for
/// malformed query paths so clients receive actionable error messages.
pub fn parse_actor_path(path: &str) -> Result<Option<ParsedActorPath>> {
	// Extract base path and raw query from the original path string directly,
	// without running through a URL parser, to preserve actor query params
	// byte-for-byte (no re-encoding of %20, +, etc.).
	let (base_path, raw_query) = split_path_and_query(path);

	if base_path.contains("//") {
		return Ok(None);
	}

	let segments: Vec<&str> = base_path
		.split('/')
		.filter(|segment| !segment.is_empty())
		.collect();
	if segments.first().copied() != Some("gateway") {
		return Ok(None);
	}

	let raw_query_str = raw_query.unwrap_or("");

	// Check if any raw query param key starts with rvt-.
	let has_rvt = !raw_query_str.is_empty()
		&& raw_query_str
			.split('&')
			.any(|part| part.split('=').next().unwrap_or("").starts_with(RVT_PREFIX));

	if has_rvt {
		let rvt_params = extract_rvt_params_from_raw_query(raw_query_str)?;
		let actor_query_string = strip_rvt_query_params(raw_query_str);
		return parse_query_actor_path(base_path, &segments, &rvt_params, &actor_query_string)
			.map(|path| Some(ParsedActorPath::Query(path)));
	}

	// Direct path: pass the raw query string through unchanged.
	let raw_query_string = match raw_query {
		Some(q) => format!("?{q}"),
		None => String::new(),
	};
	Ok(parse_direct_actor_path(
		base_path,
		&segments,
		&raw_query_string,
	))
}

fn parse_direct_actor_path(
	base_path: &str,
	segments: &[&str],
	raw_query_string: &str,
) -> Option<ParsedActorPath> {
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

		let actor_id = urlencoding::decode(raw_actor_id).ok()?.into_owned();
		let token = urlencoding::decode(raw_token).ok()?.into_owned();
		(actor_id, Some(token))
	} else {
		(urlencoding::decode(actor_segment).ok()?.into_owned(), None)
	};

	let remaining_path = build_remaining_path(base_path, raw_query_string, 2);

	Some(ParsedActorPath::Direct(ActorPathInfo {
		actor_id,
		token,
		stripped_path: remaining_path,
	}))
}

fn parse_query_actor_path(
	base_path: &str,
	segments: &[&str],
	rvt_params: &[(String, String)],
	actor_query_string: &str,
) -> Result<QueryActorPathInfo> {
	let name_segment = segments
		.get(1)
		.copied()
		.ok_or_else(|| errors::QueryEmptyActorName.build())?;

	if name_segment.contains('@') {
		return Err(errors::QueryPathTokenSyntax.build());
	}

	let name = urlencoding::decode(name_segment).map_err(|_| {
		errors::QueryInvalidPercentEncoding {
			name: "name".to_string(),
		}
		.build()
	})?;

	if name.is_empty() {
		return Err(errors::QueryEmptyActorName.build());
	}

	let rvt = extract_rvt_params(rvt_params)?;
	let stripped_path = build_remaining_path(base_path, actor_query_string, 2);

	Ok(QueryActorPathInfo {
		token: rvt.token.clone(),
		query: build_actor_query(&name, rvt)?,
		stripped_path,
	})
}

/// Extract and validate rvt-* params from pre-parsed query pairs.
fn extract_rvt_params(rvt_params: &[(String, String)]) -> Result<RvtParams> {
	let mut map = serde_json::Map::new();

	for (raw_key, value) in rvt_params {
		let stripped = raw_key
			.strip_prefix(RVT_PREFIX)
			.expect("rvt_params should only contain rvt- prefixed keys");

		if map.contains_key(stripped) {
			return Err(errors::QueryDuplicateParam {
				name: raw_key.clone(),
			}
			.build());
		}
		map.insert(
			stripped.to_string(),
			serde_json::Value::String(value.clone()),
		);
	}

	serde_json::from_value(serde_json::Value::Object(map)).map_err(|e| {
		errors::QueryInvalidParams {
			detail: e.to_string(),
		}
		.build()
	})
}

/// Split a comma-separated key string into components.
/// Missing or empty key yields an empty vec.
fn split_key(raw: Option<&str>) -> Vec<String> {
	match raw {
		None | Some("") => Vec::new(),
		Some(s) => s.split(',').map(String::from).collect(),
	}
}

fn build_actor_query(name: &str, rvt: RvtParams) -> Result<QueryActorQuery> {
	let key = split_key(rvt.key.as_deref());

	match rvt.method.as_str() {
		"get" => {
			if rvt.input.is_some()
				|| rvt.region.is_some()
				|| rvt.crash_policy.is_some()
				|| rvt.runner.is_some()
			{
				return Err(errors::QueryGetDisallowedParams.build());
			}

			Ok(QueryActorQuery::Get {
				namespace: rvt.namespace,
				name: name.to_string(),
				key,
			})
		}
		"getOrCreate" => {
			let pool_name = rvt
				.pool
				.or(rvt.runner)
				.ok_or_else(|| errors::QueryMissingPool.build())?;

			let input = rvt.input.as_deref().map(decode_query_input).transpose()?;

			let crash_policy = rvt
				.crash_policy
				.as_deref()
				.map(parse_crash_policy)
				.transpose()?;

			Ok(QueryActorQuery::GetOrCreate {
				namespace: rvt.namespace,
				name: name.to_string(),
				pool_name,
				key,
				input,
				region: rvt.region,
				crash_policy,
			})
		}
		other => Err(errors::QueryInvalidParams {
			detail: format!("unknown method: {other}, expected get or getOrCreate"),
		}
		.build()),
	}
}

fn parse_crash_policy(value: &str) -> Result<CrashPolicy> {
	match value {
		"restart" => Ok(CrashPolicy::Restart),
		"sleep" => Ok(CrashPolicy::Sleep),
		"destroy" => Ok(CrashPolicy::Destroy),
		other => Err(errors::QueryInvalidParams {
			detail: format!("unknown crash policy: {other}, expected restart, sleep, or destroy"),
		}
		.build()),
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

/// Split a path into the base path and the raw query string (without `?`).
/// Fragments are stripped.
fn split_path_and_query(path: &str) -> (&str, Option<&str>) {
	let path = match path.find('#') {
		Some(pos) => &path[..pos],
		None => path,
	};
	match path.find('?') {
		Some(pos) => (&path[..pos], Some(&path[pos + 1..])),
		None => (path, None),
	}
}

/// Extract rvt-* params from a raw query string, decoding their values
/// using form-urlencoded rules (`+` as space, then percent-decode).
fn extract_rvt_params_from_raw_query(raw_query: &str) -> Result<Vec<(String, String)>> {
	let mut params = Vec::new();
	for part in raw_query.split('&') {
		let (raw_key, raw_value) = match part.find('=') {
			Some(pos) => (&part[..pos], &part[pos + 1..]),
			None => (part, ""),
		};
		if raw_key.starts_with(RVT_PREFIX) {
			let decoded_value = decode_form_value(raw_value).map_err(|_| {
				errors::QueryInvalidPercentEncoding {
					name: raw_key.to_string(),
				}
				.build()
			})?;
			params.push((raw_key.to_string(), decoded_value));
		}
	}
	Ok(params)
}

/// Decode a form-urlencoded value: treat `+` as space, then percent-decode.
fn decode_form_value(raw: &str) -> std::result::Result<String, std::string::FromUtf8Error> {
	let with_spaces = raw.replace('+', " ");
	urlencoding::decode(&with_spaces).map(|s| s.into_owned())
}

/// Strip rvt-* params from a raw query string, preserving actor params
/// byte-for-byte without re-encoding.
fn strip_rvt_query_params(raw_query: &str) -> String {
	let actor_parts: Vec<&str> = raw_query
		.split('&')
		.filter(|part| {
			if part.is_empty() {
				return false;
			}
			let key = part.split('=').next().unwrap_or("");
			!key.starts_with(RVT_PREFIX)
		})
		.collect();

	if actor_parts.is_empty() {
		String::new()
	} else {
		format!("?{}", actor_parts.join("&"))
	}
}

fn build_remaining_path(base_path: &str, query_string: &str, consumed_segments: usize) -> String {
	let segments: Vec<&str> = base_path
		.split('/')
		.filter(|segment| !segment.is_empty())
		.collect();

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
