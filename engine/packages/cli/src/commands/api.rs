use std::{
	fs,
	io::{IsTerminal, Read, Write},
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use reqwest::{
	Method,
	header::{HeaderName, HeaderValue},
};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{
	auth::resolve_auth,
	cloud::CloudClient,
	target::{CloudArgs, Link, find_link},
	util::encode,
};

const AFTER_HELP: &str = "\
Discover endpoints from the OpenAPI specs:
  rivet api /v2/openapi.json            API v2 (clusters, builds, runner pools)
  rivet api /doc                        API v1 (projects, namespaces, managed pools)

Examples:
  rivet api /v2/clusters
  rivet api '/v2/clusters/{cluster}/builds'
  rivet api '/v2/clusters/{cluster}/builds/<build-id>' -X DELETE
  rivet api '/projects/{project}/namespaces' -X GET -f 'org={org}' -F limit=10
  rivet api '/v2/clusters/{cluster}/regions/<region>/runner-pools/default' -X PUT \\
    -f build_id=<build-id> -f 'cidr_allowlist[]'
  echo '{\"build_id\":\"<build-id>\",\"cidr_allowlist\":[]}' | rivet api <path> -X PUT --input -

The response body is written to stdout. A failed request exits with status 1
after printing the body.";

#[derive(Parser)]
#[command(after_help = AFTER_HELP)]
pub struct Opts {
	#[command(flatten)]
	cloud: CloudArgs,
	/// Request path, such as `/v2/clusters` or `/projects?org={org}`. The
	/// placeholders `{cluster}`, `{org}` and `{project}` are filled from the
	/// matching flag or the linked directory.
	path: String,
	/// HTTP method. Defaults to GET, or POST when a body is given.
	#[arg(short = 'X', long)]
	method: Option<String>,
	/// String field as key=value, repeatable. Sent as a JSON body, or as query
	/// parameters on a GET. `key[]=value` appends to an array.
	#[arg(short = 'f', long = "raw-field", value_name = "KEY=VALUE")]
	raw_fields: Vec<String>,
	/// Typed field as key=value, repeatable. `true`, `false`, `null` and
	/// numbers become JSON values, and `@file` reads the value from a file.
	#[arg(short = 'F', long = "field", value_name = "KEY=VALUE")]
	fields: Vec<String>,
	/// JSON file to send as the request body, or `-` for stdin.
	#[arg(long, value_name = "FILE", conflicts_with_all = ["raw_fields", "fields"])]
	input: Option<String>,
	/// Extra request header as `Name: value`, repeatable.
	#[arg(short = 'H', long = "header", value_name = "HEADER")]
	headers: Vec<String>,
	/// Print the response status line and headers before the body.
	#[arg(short = 'i', long)]
	include: bool,
}

#[derive(Deserialize)]
struct ApiError {
	message: String,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		let mut fields = Map::new();
		for field in &self.raw_fields {
			let (key, value) = split_field(field)?;
			insert_field(
				&mut fields,
				key,
				value.map(|v| Value::String(v.to_string())),
			)?;
		}
		for field in &self.fields {
			let (key, value) = split_field(field)?;
			let value = value.map(typed_value).transpose()?;
			insert_field(&mut fields, key, value)?;
		}
		let input = self.input.as_deref().map(read_input).transpose()?;

		let has_body = input.is_some() || !fields.is_empty();
		let method = match &self.method {
			Some(method) => Method::from_bytes(method.to_ascii_uppercase().as_bytes())
				.with_context(|| format!("invalid HTTP method: {method}"))?,
			None if has_body => Method::POST,
			None => Method::GET,
		};

		let mut path = fill_placeholders(&self.path, |name| self.placeholder(name))?;
		let body = match input {
			Some(input) => Some(input),
			None if fields.is_empty() => None,
			None if method == Method::GET => {
				append_query(&mut path, &fields)?;
				None
			}
			None => Some(Value::Object(fields)),
		};

		let auth = resolve_auth(self.cloud.token.as_deref(), &self.cloud.cloud_api).await?;
		// A login token names only the user, so API v2 needs the organization
		// when the user belongs to several.
		let cloud = CloudClient::new(&self.cloud.cloud_api, auth.token.clone())?
			.with_login_refresh(&auth, &self.cloud.cloud_api)
			.with_organization(self.cloud.org.clone());
		let mut headers = Vec::with_capacity(self.headers.len());
		for header in &self.headers {
			let (name, value) = header
				.split_once(':')
				.with_context(|| format!("header must be `Name: value`: {header}"))?;
			headers.push((
				HeaderName::from_bytes(name.trim().as_bytes())
					.with_context(|| format!("invalid header name: {name}"))?,
				HeaderValue::from_str(value.trim())
					.with_context(|| format!("invalid value for header {name}"))?,
			));
		}

		let response = cloud
			.send_with_headers(method, &path, body, &headers)
			.await?;
		let status = response.status();
		let mut stdout = std::io::stdout().lock();
		if self.include {
			writeln!(stdout, "{:?} {status}", response.version())?;
			for (name, value) in response.headers() {
				writeln!(
					stdout,
					"{name}: {}",
					String::from_utf8_lossy(value.as_bytes())
				)?;
			}
			writeln!(stdout)?;
		}
		let text = response.text().await.context("read Cloud API response")?;

		let pretty = if stdout.is_terminal() {
			serde_json::from_str::<Value>(&text)
				.ok()
				.and_then(|json| serde_json::to_string_pretty(&json).ok())
		} else {
			None
		};
		match pretty {
			Some(pretty) => writeln!(stdout, "{pretty}")?,
			None if text.is_empty() => {}
			None => writeln!(stdout, "{}", text.trim_end_matches('\n'))?,
		}

		if !status.is_success() {
			match serde_json::from_str::<ApiError>(&text) {
				Ok(error) => bail!("{} (HTTP {})", error.message, status.as_u16()),
				Err(_) => bail!("HTTP {status}"),
			}
		}
		Ok(())
	}

	/// Resolves a placeholder from its flag, then the linked directory.
	fn placeholder(&self, name: &str) -> Result<Option<String>> {
		let flag = match name {
			"cluster" => &self.cloud.cluster,
			"org" => &self.cloud.org,
			"project" => &self.cloud.project,
			_ => return Ok(None),
		};
		if let Some(value) = flag {
			return Ok(Some(value.clone()));
		}
		let linked = match find_link()?.map(|(link, _)| link) {
			Some(Link::Cluster { cluster, .. }) if name == "cluster" => Some(cluster),
			Some(Link::Project { org, .. }) if name == "org" => Some(org),
			Some(Link::Project { project, .. }) if name == "project" => Some(project),
			Some(Link::Cluster { .. }) | Some(Link::Project { .. }) | None => None,
		};
		match linked {
			Some(value) => Ok(Some(value)),
			None => bail!(
				"the path uses {{{name}}}, but none is selected; pass --{name} or link this directory"
			),
		}
	}
}

/// Replaces each known `{name}` with its URL-encoded value. Braces around an
/// unknown name are left as written.
fn fill_placeholders(
	path: &str,
	resolve: impl Fn(&str) -> Result<Option<String>>,
) -> Result<String> {
	let mut filled = String::with_capacity(path.len());
	let mut rest = path;
	while let Some(open) = rest.find('{') {
		let Some(close) = rest[open..].find('}') else {
			break;
		};
		let name = &rest[open + 1..open + close];
		filled.push_str(&rest[..open]);
		match resolve(name)? {
			Some(value) => filled.push_str(&encode(&value)),
			None => filled.push_str(&rest[open..=open + close]),
		}
		rest = &rest[open + close + 1..];
	}
	filled.push_str(rest);
	Ok(filled)
}

/// Splits `key=value`. A bare `key[]` has no value and makes an empty array.
fn split_field(field: &str) -> Result<(&str, Option<&str>)> {
	match field.split_once('=') {
		Some(("", _)) => bail!("field is missing a key: {field}"),
		Some((key, value)) => Ok((key, Some(value))),
		None if field.ends_with("[]") && field.len() > 2 => Ok((field, None)),
		None => bail!("field must be key=value: {field}"),
	}
}

fn typed_value(value: &str) -> Result<Value> {
	if let Some(path) = value.strip_prefix('@') {
		return Ok(Value::String(read_source(path)?));
	}
	Ok(match value {
		"true" => Value::Bool(true),
		"false" => Value::Bool(false),
		"null" => Value::Null,
		_ => match value.parse::<i64>() {
			Ok(number) => Value::from(number),
			Err(_) => match value
				.parse::<f64>()
				.ok()
				.and_then(serde_json::Number::from_f64)
			{
				Some(number) => Value::Number(number),
				None => Value::String(value.to_string()),
			},
		},
	})
}

fn insert_field(fields: &mut Map<String, Value>, key: &str, value: Option<Value>) -> Result<()> {
	let Some(array_key) = key.strip_suffix("[]") else {
		let value = value.with_context(|| format!("field must be key=value: {key}"))?;
		fields.insert(key.to_string(), value);
		return Ok(());
	};
	let entry = fields
		.entry(array_key.to_string())
		.or_insert_with(|| Value::Array(Vec::new()));
	let Value::Array(items) = entry else {
		bail!("field {array_key} is set both as a value and as an array");
	};
	if let Some(value) = value {
		items.push(value);
	}
	Ok(())
}

fn append_query(path: &mut String, fields: &Map<String, Value>) -> Result<()> {
	for (key, value) in fields {
		let values = match value {
			Value::Array(items) => items.iter().collect(),
			Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => vec![value],
			Value::Object(_) => bail!("field {key} cannot be sent as a query parameter"),
		};
		for value in values {
			let value = match value {
				Value::String(text) => text.clone(),
				Value::Null | Value::Bool(_) | Value::Number(_) => value.to_string(),
				Value::Array(_) | Value::Object(_) => {
					bail!("field {key} cannot be sent as a query parameter")
				}
			};
			let separator = if path.contains('?') { '&' } else { '?' };
			path.push(separator);
			path.push_str(&encode(key));
			path.push('=');
			path.push_str(&encode(&value));
		}
	}
	Ok(())
}

fn read_input(source: &str) -> Result<Value> {
	serde_json::from_str(&read_source(source)?)
		.with_context(|| format!("--input {source} is not valid JSON"))
}

fn read_source(source: &str) -> Result<String> {
	if source == "-" {
		let mut text = String::new();
		std::io::stdin()
			.read_to_string(&mut text)
			.context("read stdin")?;
		return Ok(text);
	}
	fs::read_to_string(source).with_context(|| format!("read {source}"))
}
