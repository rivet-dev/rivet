use std::{
	net::{IpAddr, SocketAddr},
	time::Duration,
};

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::pkg::*;
use rivet_api::{
	apis::{configuration::Configuration, *},
	models,
};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::connect_async;

use crate::auth::Auth;

// MARK: GET /matchmaker
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusQuery {
	region: String,
}

pub async fn status(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: StatusQuery,
) -> GlobalResult<serde_json::Value> {
	// Find namespace ID
	let game_res = op!([ctx] game_resolve_name_id {
		name_ids: vec!["sandbox".into()],
	})
	.await?;
	let game_id = unwrap_with!(
		game_res.games.first().and_then(|x| x.game_id),
		INTERNAL_STATUS_CHECK_FAILED,
		error = "missing sandbox game"
	);
	let ns_resolve = op!([ctx] game_namespace_resolve_name_id {
		game_id: Some(game_id),
		name_ids: vec!["prod".into()],
	})
	.await?;
	let ns_id = unwrap_with!(
		ns_resolve.namespaces.first().and_then(|x| x.namespace_id),
		INTERNAL_STATUS_CHECK_FAILED,
		error = "missing prod namespace"
	);

	// Create bypass token
	let ns_token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(15),
		}),
		refresh_token_config: None,
		issuer: "api-status".to_owned(),
		client: Some(ctx.client_info()),
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(proto::claims::entitlement::Kind::GameNamespacePublic(
						proto::claims::entitlement::GameNamespacePublic {
							namespace_id: Some(ns_id),
						},
					))
				},
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::Bypass(proto::claims::entitlement::Bypass {})
					)
				}
			],
		})),
		label: Some("ns_pub".to_owned()),
		..Default::default()
	})
	.await?;
	let ns_token = unwrap_ref!(ns_token_res.token).token.clone();

	// Bypass token
	let bypass_token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(15),
		}),
		refresh_token_config: None,
		issuer: "api-status".to_owned(),
		client: Some(ctx.client_info()),
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::Bypass(proto::claims::entitlement::Bypass {})
					)
				}
			],
		})),
		label: Some("byp".to_owned()),
		..Default::default()
	})
	.await?;
	let bypass_token = unwrap_ref!(bypass_token_res.token).token.clone();

	// Create client
	let mut headers = reqwest::header::HeaderMap::new();
	headers.insert("host", util::env::host_api().parse()?);
	headers.insert(
		"cf-connecting-ip",
		reqwest::header::HeaderValue::from_str("127.0.0.1")?,
	);
	headers.insert(
		"x-coords",
		reqwest::header::HeaderValue::from_str("0.0,0.0")?,
	);
	headers.insert(
		"x-bypass-token",
		reqwest::header::HeaderValue::from_str(&bypass_token)?,
	);

	let client = reqwest::ClientBuilder::new()
		.default_headers(headers)
		.build()?;
	let config = Configuration {
		base_path: "http://traefik.traefik.svc.cluster.local:80".into(),
		bearer_access_token: Some(ns_token),
		client,
		..Default::default()
	};

	tracing::info!("finding lobby");
	let res = matchmaker_lobbies_api::matchmaker_lobbies_create(
		&config,
		models::MatchmakerLobbiesCreateRequest {
			game_mode: "custom".into(),
			region: Some(query.region.clone()),
			..Default::default()
		},
	)
	.await;
	let res = match res {
		Ok(x) => x,
		Err(err) => {
			bail_with!(
				INTERNAL_STATUS_CHECK_FAILED,
				error = format!("find lobby: {:?}", err)
			)
		}
	};

	// Test connection, defer error
	let lobby_id = res.lobby.lobby_id.clone();
	let test_res = tokio::time::timeout(
		Duration::from_secs(15),
		test_lobby_connection(res.lobby, res.player),
	)
	.await;

	// Shut down lobby regardless of connection status
	//
	// This way if the connection fails to connect, we still clean up the lobby instead of spamming
	// lobbies with unconnected players
	msg!([ctx] mm::msg::lobby_stop(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;

	// Unwrap res
	match test_res {
		Ok(Ok(())) => {}
		Ok(Err(err)) => {
			return Err(err);
		}
		Err(_) => {
			bail_with!(
				INTERNAL_STATUS_CHECK_FAILED,
				error = "test lobby connection timed out"
			)
		}
	}
	// if let Err(err) = test_res {
	// 	return Err(err);
	// }

	Ok(serde_json::json!({}))
}

async fn test_lobby_connection(
	lobby: Box<models::MatchmakerJoinLobby>,
	player: Box<models::MatchmakerJoinPlayer>,
) -> GlobalResult<()> {
	let port_default = unwrap!(lobby.ports.get("default"));
	let host = unwrap_ref!(port_default.host);
	let hostname = &port_default.hostname;
	let token = &player.token;

	// Look up IP for GG nodes
	let gg_ips = lookup_dns(hostname).await?;
	ensure_with!(
		!gg_ips.is_empty(),
		INTERNAL_STATUS_CHECK_FAILED,
		error = format!("no IPs found for GG host {hostname}")
	);

	// Test HTTP connectivity. This gives a verbose error if the LB returns a non-200 response.
	let test_http_res = futures_util::future::join_all(
		gg_ips
			.iter()
			.cloned()
			.map(|x| test_http(hostname.clone(), (x, 443).into())),
	)
	.await;
	let failed_tests = gg_ips
		.iter()
		.zip(test_http_res.iter())
		.filter_map(|(ip, res)| {
			if let Err(err) = res {
				Some((ip, err))
			} else {
				None
			}
		})
		.collect::<Vec<_>>();
	if !failed_tests.is_empty() {
		bail_with!(
			INTERNAL_STATUS_CHECK_FAILED,
			error = format!(
				"{} http out of {} failed: {:?}",
				failed_tests.len(),
				gg_ips.len(),
				failed_tests
			)
		);
	}

	// Test WebSocket connectivity. This will pass the player token to the server & call player
	// connected + disconnected. This will shut down the lobby once the socket closes.
	test_ws(host, token).await.map_err(|err| {
		err_code!(
			INTERNAL_STATUS_CHECK_FAILED,
			error = format!("ws failed: {err:?}")
		)
	})?;

	Ok(())
}

/// Returns the IP addresses for a given hostname.
async fn lookup_dns(hostname: &str) -> GlobalResult<Vec<IpAddr>> {
	let resolver = trust_dns_resolver::TokioAsyncResolver::tokio_from_system_conf()?;
	let addrs = resolver
		.lookup_ip(hostname)
		.await?
		.into_iter()
		.collect::<Vec<IpAddr>>();

	Ok(addrs)
}

/// Tests HTTP connectivity to a hostname for a given address.
///
/// This lets us isolate of a specific GG IP address is not behaving correctly.
async fn test_http(host: String, addr: SocketAddr) -> Result<(), reqwest::Error> {
	// Resolve the host to the specific IP addr
	let client = reqwest::Client::builder().resolve(&host, addr).build()?;

	// Test HTTP connectivity
	client
		.get(format!("https://{host}/health"))
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}

/// Tests WebSocket connectivity to a hostname.
async fn test_ws(host: &str, token: &str) -> GlobalResult<()> {
	let (mut socket, _) = connect_async(format!("wss://{host}/?token={token}")).await?;

	tokio::time::sleep(Duration::from_millis(500)).await;

	socket.close(None).await?;

	Ok(())
}
