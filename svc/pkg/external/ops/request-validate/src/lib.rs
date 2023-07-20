use http::{uri::Scheme, HeaderName, HeaderValue, Uri};
use std::{net::IpAddr, str::FromStr};

use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "external-request-validate")]
async fn handle(
	ctx: OperationContext<external::request_validate::Request>,
) -> GlobalResult<external::request_validate::Response> {
	let config = internal_unwrap!(ctx.config);
	let mut errors = Vec::new();

	// Parse URL
	if let Ok(url) = config.url.parse::<Uri>() {
		if let Some(host) = url.host() {
			// Validate as IP
			if let Ok(ip) = IpAddr::from_str(host) {
				// Validate that IP is global
				if !is_global_ip(ip) {
					errors.push(util::err_path!["url", "invalid-ip"]);
				}
			}
			// Validate that URL is not internal
			else if host.ends_with(".consul") {
				errors.push(util::err_path!["url", "invalid"]);
			}

			// Validate URL scheme
			if let Some(scheme) = url.scheme() {
				if scheme != &Scheme::HTTP && scheme != &Scheme::HTTPS {
					errors.push(util::err_path!["url", "invalid-scheme"]);
				}
			} else {
				errors.push(util::err_path!["url", "invalid-scheme"]);
			}
		} else {
			errors.push(util::err_path!["url", "no-host"]);
		}
	} else {
		errors.push(util::err_path!["url", "invalid"]);
	}

	// Validate DNS
	if errors.is_empty() {
		let req = reqwest::Client::new()
			.get(&config.url)
			.timeout(std::time::Duration::from_secs(5));

		if let Err(err) = req.send().await {
			if err.is_connect() {
				errors.push(util::err_path!["url", "dns-failed"]);
			} else if err.is_timeout() {
				errors.push(util::err_path!["url", "dns-timeout"]);
			} else {
				// Throw error if its not a connection or timeout problem
				return Err(err.into());
			}
		}
	}

	if config.headers.len() > 32 {
		errors.push(util::err_path!["headers-meta", "too-many"]);
	}

	// Validate headers
	for (k, v) in config.headers.iter().take(32) {
		let header_label = format!("*{}*", k);

		if HeaderName::from_str(k).is_err() {
			errors.push(util::err_path!["headers", header_label, "invalid-name"]);
		}
		if HeaderValue::from_str(v).is_err() {
			errors.push(util::err_path!["headers", header_label, "invalid-value"]);
		}
	}

	Ok(external::request_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| external::request_validate::response::Error { path })
			.collect::<Vec<_>>(),
	})
}

// TODO: Replace with `IpAddr::is_global` whenever that stabilizes
#[allow(clippy::manual_range_contains)]
fn is_global_ip(ip: IpAddr) -> bool {
	match ip {
		IpAddr::V4(ip) => {
			!(ip.octets()[0] == 0 // "This network"
		|| ip.is_private()
		|| (ip.octets()[0] == 100 && (ip.octets()[1] & 0b1100_0000 == 0b0100_0000))
		|| ip.is_loopback()
		|| ip.is_link_local()
		// addresses reserved for future protocols (`192.0.0.0/24`)
		||(ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
		|| ip.is_documentation()
		|| (ip.octets()[0] == 198 && (ip.octets()[1] & 0xfe) == 18)
		|| (ip.octets()[0] & 240 == 240 && !ip.is_broadcast())
		|| ip.is_broadcast())
		}
		IpAddr::V6(ip) => {
			!(ip.is_unspecified()
		|| ip.is_loopback()
		// IPv4-mapped Address (`::ffff:0:0/96`)
		|| matches!(ip.segments(), [0, 0, 0, 0, 0, 0xffff, _, _])
		// IPv4-IPv6 Translat. (`64:ff9b:1::/48`)
		|| matches!(ip.segments(), [0x64, 0xff9b, 1, _, _, _, _, _])
		// Discard-Only Address Block (`100::/64`)
		|| matches!(ip.segments(), [0x100, 0, 0, 0, _, _, _, _])
		// IETF Protocol Assignments (`2001::/23`)
		|| (matches!(ip.segments(), [0x2001, b, _, _, _, _, _, _] if b < 0x200)
			&& !(
				// Port Control Protocol Anycast (`2001:1::1`)
				u128::from_be_bytes(ip.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0001
				// Traversal Using Relays around NAT Anycast (`2001:1::2`)
				|| u128::from_be_bytes(ip.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0002
				// AMT (`2001:3::/32`)
				|| matches!(ip.segments(), [0x2001, 3, _, _, _, _, _, _])
				// AS112-v6 (`2001:4:112::/48`)
				|| matches!(ip.segments(), [0x2001, 4, 0x112, _, _, _, _, _])
				// ORCHIDv2 (`2001:20::/28`)
				|| matches!(ip.segments(), [0x2001, b, _, _, _, _, _, _] if b >= 0x20 && b <= 0x2F)
			))
		|| ((ip.segments()[0] == 0x2001) && (ip.segments()[1] == 0xdb8))
		|| ((ip.segments()[0] & 0xfe00) == 0xfc00)
		|| ((ip.segments()[0] & 0xffc0) == 0xfe80))
		}
	}
}
