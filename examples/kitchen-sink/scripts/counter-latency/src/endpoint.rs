// Endpoint URL parsing + raw WebSocket URL builder. Mirrors the top-level
// constants `NAMESPACE`, `TOKEN`, `WS_ORIGIN`, `RVT_RUNNER` and
// `buildRawWebSocketUrl` in scripts/counter-latency.ts.

use anyhow::{Context, Result, anyhow};
use url::Url;

pub struct Endpoint {
	pub namespace: String,
	pub token: String,
	pub ws_origin: String,
	pub display_origin: String,
	pub rvt_runner: String,
}

impl Endpoint {
	pub fn parse(raw_endpoint: &str, rivet_pool: String) -> Result<Self> {
		let url = Url::parse(raw_endpoint).context("invalid endpoint URL")?;
		let namespace = percent_decode(url.username())?;
		let token = match url.password() {
			Some(p) => percent_decode(p)?,
			None => String::new(),
		};
		let ws_proto = if url.scheme() == "https" { "wss" } else { "ws" };
		let host = url
			.host_str()
			.ok_or_else(|| anyhow!("endpoint missing host"))?;
		let port = url.port();
		let host_with_port = match port {
			Some(p) => format!("{}:{}", host, p),
			None => host.to_string(),
		};
		let ws_origin = format!("{}://{}", ws_proto, host_with_port);
		let display_origin = format!("{}://{}", url.scheme(), host_with_port);
		Ok(Self {
			namespace,
			token,
			ws_origin,
			display_origin,
			rvt_runner: rivet_pool,
		})
	}

	pub fn build_raw_ws_url(&self, actor_name: &str, key: &str, skip_ready_wait: bool) -> String {
		let mut params = Vec::<(String, String)>::new();
		params.push(("rvt-namespace".into(), self.namespace.clone()));
		params.push(("rvt-method".into(), "getOrCreate".into()));
		params.push(("rvt-runner".into(), self.rvt_runner.clone()));
		params.push(("rvt-key".into(), key.into()));
		params.push(("rvt-crash-policy".into(), "sleep".into()));
		if !self.token.is_empty() {
			params.push(("rvt-token".into(), self.token.clone()));
		}
		if skip_ready_wait {
			params.push(("rvt-skip-ready-wait".into(), "true".into()));
		}
		let qs = params
			.iter()
			.map(|(k, v)| format!("{}={}", encode_query(k), encode_query(v)))
			.collect::<Vec<_>>()
			.join("&");
		format!(
			"{}/gateway/{}/websocket?{}",
			self.ws_origin,
			encode_path(actor_name),
			qs,
		)
	}
}

fn percent_decode(s: &str) -> Result<String> {
	let decoded = urlencoding_decode(s)?;
	Ok(decoded)
}

// Minimal URL-encoding helpers using percent-encoding semantics compatible
// with `encodeURIComponent` for the characters we care about
// (alphanumerics + `-._~` left as-is; everything else percent-encoded).
fn encode_query(s: &str) -> String {
	encode_uri_component(s)
}

fn encode_path(s: &str) -> String {
	encode_uri_component(s)
}

fn encode_uri_component(s: &str) -> String {
	let mut buf = String::with_capacity(s.len());
	for b in s.bytes() {
		if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
			buf.push(b as char);
		} else {
			buf.push_str(&format!("%{:02X}", b));
		}
	}
	buf
}

fn urlencoding_decode(s: &str) -> Result<String> {
	let bytes = s.as_bytes();
	let mut out = Vec::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		match bytes[i] {
			b'%' if i + 2 < bytes.len() => {
				let hex = std::str::from_utf8(&bytes[i + 1..i + 3])?;
				let v = u8::from_str_radix(hex, 16).context("invalid percent-encoding")?;
				out.push(v);
				i += 3;
			}
			b'+' => {
				out.push(b' ');
				i += 1;
			}
			b => {
				out.push(b);
				i += 1;
			}
		}
	}
	Ok(String::from_utf8(out).context("invalid UTF-8 after decode")?)
}
