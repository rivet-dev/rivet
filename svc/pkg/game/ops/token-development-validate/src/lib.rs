use proto::backend::{
	matchmaker::lobby_runtime::ProxyProtocol as LobbyRuntimeProxyProtocol, pkg::*,
};
use rivet_operation::prelude::*;

use std::collections::HashSet;

#[operation(name = "game-token-development-validate")]
async fn handle(
	ctx: OperationContext<game::token_development_validate::Request>,
) -> GlobalResult<game::token_development_validate::Response> {
	let mut errors = Vec::new();

	let mut unique_port_labels = HashSet::<String>::new();
	let mut unique_ports = HashSet::<(u32, i32)>::new();
	let mut ranges = Vec::<(u32, u32)>::new();

	if ctx.lobby_ports.len() > 16 {
		errors.push(util::err_path!["ports-too-many"]);
	}

	for (port_index, port) in ctx.lobby_ports.iter().enumerate() {
		if util::check::ident(&port.label) {
			if unique_port_labels.contains(&port.label) {
				errors.push(util::err_path!["ports", port_index, "label-not-unique"]);
			} else {
				unique_port_labels.insert(port.label.clone());
			}
		} else {
			errors.push(util::err_path!["ports", port_index, "label-invalid"]);
		}

		let proxy_protocol = unwrap!(LobbyRuntimeProxyProtocol::from_i32(port.proxy_protocol));

		// Validate ports unique
		if let Some(target_port) = port.target_port {
			if unique_ports.contains(&(target_port, port.proxy_protocol)) {
				errors.push(util::err_path![
					"ports",
					port_index,
					"port-protocol-not-unique",
				]);
			} else {
				unique_ports.insert((target_port, port.proxy_protocol));
			}
		}

		match (proxy_protocol, port.target_port, &port.port_range) {
			// === Port Single ===
			(
				LobbyRuntimeProxyProtocol::Http
				| LobbyRuntimeProxyProtocol::Https
				| LobbyRuntimeProxyProtocol::Tcp
				| LobbyRuntimeProxyProtocol::TcpTls
				| LobbyRuntimeProxyProtocol::Udp,
				Some(_),
				None,
			) => {
				// Valid
			}

			// === Port Range ===
			(
				LobbyRuntimeProxyProtocol::Tcp | LobbyRuntimeProxyProtocol::Udp,
				None,
				Some(port_range),
			) => {
				// Validate port range
				if port_range.min > port_range.max {
					errors.push(util::err_path!["ports", port_index, "range-min-gt-max",]);
				}

				// Validate ranges
				if ranges
					.iter()
					.any(|(min, max)| port_range.max >= *min && port_range.min <= *max)
				{
					errors.push(util::err_path!["ports", port_index, "ranges-overlap",]);
				}

				ranges.push((port_range.min, port_range.max));
			}

			// === Error cases ===
			(
				LobbyRuntimeProxyProtocol::Http
				| LobbyRuntimeProxyProtocol::Https
				| LobbyRuntimeProxyProtocol::TcpTls,
				None,
				Some(_),
			) => {
				errors.push(util::err_path![
					"ports",
					port_index,
					"unsupported-port-range",
				]);
			}
			(_, Some(_), Some(_)) => {
				errors.push(util::err_path![
					"ports",
					port_index,
					"duplicate-port-and-port-range",
				]);
			}
			(_, None, None) => {
				errors.push(util::err_path![
					"ports",
					port_index,
					"missing-port-and-port-range",
				]);
			}
		}
	}

	Ok(game::token_development_validate::Response {
		errors: errors
			.into_iter()
			.map(|path| game::token_development_validate::response::Error { path })
			.collect::<Vec<_>>(),
	})
}
