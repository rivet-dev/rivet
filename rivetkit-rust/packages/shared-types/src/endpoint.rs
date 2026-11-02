//! Rivet endpoint URL handling shared by the RivetKit server and client crates.
//! The TypeScript twins are `tryParseEndpoint` in `utils/endpoint-parser.ts`
//! and `isLocalEngineEndpoint` in `common/engine.ts`; keep the three in step so
//! a Rust process resolves the same connection as a TypeScript one.

use std::fmt;
use std::net::IpAddr;

use url::Url;

/// An endpoint with URL auth credentials split out and stripped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedEndpoint {
	/// The URL-normalized endpoint without credentials.
	pub endpoint: String,
	pub namespace: Option<String>,
	pub token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndpointParseError {
	InvalidUrl {
		endpoint: String,
	},
	QueryString,
	Fragment,
	TokenWithoutNamespace,
	/// The raw credential is left out because it may be a token.
	InvalidCredentialEncoding,
	CredentialsUnsupported,
}

impl fmt::Display for EndpointParseError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidUrl { endpoint } => write!(f, "invalid URL: {endpoint}"),
			Self::QueryString => write!(f, "endpoint cannot contain a query string"),
			Self::Fragment => write!(f, "endpoint cannot contain a fragment"),
			Self::TokenWithoutNamespace => {
				write!(f, "endpoint cannot have a token without a namespace")
			}
			Self::InvalidCredentialEncoding => {
				write!(f, "invalid percent-encoding in endpoint URL auth")
			}
			Self::CredentialsUnsupported => {
				write!(f, "endpoint URL does not support credentials")
			}
		}
	}
}

impl std::error::Error for EndpointParseError {}

/// Parses an endpoint that may carry `https://namespace:token@host` URL auth.
/// The returned endpoint is the URL-normalized string with any credentials
/// stripped. Errors on invalid URLs, query strings, fragments, and a token
/// without a namespace.
pub fn parse_endpoint_auth(endpoint: &str) -> Result<ParsedEndpoint, EndpointParseError> {
	let mut url = Url::parse(endpoint).map_err(|_| EndpointParseError::InvalidUrl {
		endpoint: endpoint.to_owned(),
	})?;
	if url.query().is_some() {
		return Err(EndpointParseError::QueryString);
	}
	if url.fragment().is_some() {
		return Err(EndpointParseError::Fragment);
	}

	let namespace = if url.username().is_empty() {
		None
	} else {
		Some(percent_decode(url.username())?)
	};
	let token = match url.password() {
		None | Some("") => None,
		Some(password) => Some(percent_decode(password)?),
	};
	if token.is_some() && namespace.is_none() {
		return Err(EndpointParseError::TokenWithoutNamespace);
	}

	// Only strip credentials when some were parsed. URLs that carry auth
	// always have an authority component, so clearing cannot fail there,
	// while opaque-scheme URLs without auth would reject the setters.
	if namespace.is_some() || token.is_some() {
		if url.set_username("").is_err() || url.set_password(None).is_err() {
			return Err(EndpointParseError::CredentialsUnsupported);
		}
	}

	Ok(ParsedEndpoint {
		endpoint: url.to_string(),
		namespace,
		token,
	})
}

/// Whether the endpoint points at an engine on this machine. Local engines
/// started by RivetKit use the `default` token, so callers may fall back to it
/// only when this returns true.
pub fn is_local_engine_endpoint(endpoint: &str) -> bool {
	let Ok(url) = Url::parse(endpoint) else {
		return false;
	};
	let Some(host) = url.host_str() else {
		return false;
	};

	if host.eq_ignore_ascii_case("localhost") {
		return true;
	}

	let ip_literal = host.trim_start_matches('[').trim_end_matches(']');
	match ip_literal.parse::<IpAddr>() {
		Ok(ip) => ip.is_loopback() || ip.is_unspecified(),
		Err(_) => false,
	}
}

fn percent_decode(value: &str) -> Result<String, EndpointParseError> {
	let bytes = value.as_bytes();
	let mut out = Vec::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		if bytes[i] == b'%' {
			if i + 2 >= bytes.len() {
				return Err(EndpointParseError::InvalidCredentialEncoding);
			}
			let hi =
				hex_value(bytes[i + 1]).ok_or(EndpointParseError::InvalidCredentialEncoding)?;
			let lo =
				hex_value(bytes[i + 2]).ok_or(EndpointParseError::InvalidCredentialEncoding)?;
			out.push((hi << 4) | lo);
			i += 3;
		} else {
			out.push(bytes[i]);
			i += 1;
		}
	}
	String::from_utf8(out).map_err(|_| EndpointParseError::InvalidCredentialEncoding)
}

fn hex_value(byte: u8) -> Option<u8> {
	match byte {
		b'0'..=b'9' => Some(byte - b'0'),
		b'a'..=b'f' => Some(byte - b'a' + 10),
		b'A'..=b'F' => Some(byte - b'A' + 10),
		_ => None,
	}
}
