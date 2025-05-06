use std::{
	collections::HashMap,
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
	region: Uuid,
	build: StatusQueryBuild,
	project: Option<String>,
	environment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
enum StatusQueryBuild {
	#[serde(rename = "ws-isolate")]
	WsIsolate,
	#[serde(rename = "ws-container")]
	WsContainer,
}

impl StatusQueryBuild {
	fn build_name(&self) -> &str {
		match self {
			StatusQueryBuild::WsIsolate => "ws-isolate",
			StatusQueryBuild::WsContainer => "ws-container",
		}
	}
}

#[tracing::instrument(skip_all, fields(?query))]
pub async fn status(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: StatusQuery,
) -> GlobalResult<serde_json::Value> {
	let status_config = ctx.config().server()?.rivet.status()?;
	let system_test_project = unwrap!(
		query
			.project
			.as_ref()
			.or(status_config.system_test_project.as_ref()),
		"system test project not configured"
	);
	let system_test_env = query
		.environment
		.clone()
		.or(status_config.system_test_environment.clone())
		.unwrap_or_else(|| "prod".to_string());

	// Find namespace ID
	let game_res = op!([ctx] game_resolve_name_id {
		name_ids: vec![system_test_project.clone()],
	})
	.await?;
	let game_id = unwrap_with!(
		game_res.games.first().and_then(|x| x.game_id),
		INTERNAL_STATUS_CHECK_FAILED,
		error = "missing {system_test_project} game"
	);
	let ns_resolve = op!([ctx] game_namespace_resolve_name_id {
		game_id: Some(game_id),
		name_ids: vec![system_test_env.clone()],
	})
	.await?;
	let ns_id = unwrap_with!(
		ns_resolve.namespaces.first().and_then(|x| x.namespace_id),
		INTERNAL_STATUS_CHECK_FAILED,
		error = "missing prod namespace"
	);

	// Resolve the region
	let datacenters_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![query.region],
		})
		.await?;
	let dc = unwrap!(datacenters_res.datacenters.first());

	// Create service token
	let service_token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(15)
		}),
		refresh_token_config: None,
		issuer: "api-status".to_owned(),
		client: None,
		kind: Some(token::create::request::Kind::New(
			token::create::request::KindNew { entitlements: vec![proto::claims::Entitlement {
				kind: Some(proto::claims::entitlement::Kind::EnvService(
					proto::claims::entitlement::EnvService {
						env_id: Some(ns_id.into()),
					}
				)),
			}]},
		)),
		label: Some("env_svc".to_owned()),
		..Default::default()
	})
	.await?;
	let service_token = unwrap_ref!(service_token_res.token).token.clone();

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
	headers.insert("host", ctx.config().server()?.rivet.api_host()?.parse()?);
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
		bearer_access_token: Some(service_token),
		client,
		..Default::default()
	};

	tracing::info!("creating actor");
	let res = actors_api::actors_create(
		&config,
		models::ActorsCreateActorRequest {
			tags: Some(serde_json::json!({
				"name": query.build.build_name(),
			})),
			build_tags: Some(Some(serde_json::json!({
				"name": query.build.build_name(),
				"current": "true",
			}))),
			region: Some(dc.name_id.clone()),
			network: Some(Box::new(models::ActorsCreateActorNetworkRequest {
				ports: Some(HashMap::from([(
					"http".to_string(),
					models::ActorsCreateActorPortRequest {
						protocol: models::ActorsPortProtocol::Https,
						routing: Some(Box::new(models::ActorsPortRouting {
							guard: Some(serde_json::json!({})),
							host: None,
						})),
						..Default::default()
					},
				)])),
				..Default::default()
			})),
			lifecycle: Some(Box::new(models::ActorsLifecycle {
				// Don't reboot on failure
				durable: Some(false),
				..Default::default()
			})),
			resources: match &query.build {
				StatusQueryBuild::WsIsolate => None,
				StatusQueryBuild::WsContainer => Some(Box::new(models::ActorsResources {
					cpu: 100,
					memory: 128,
				})),
			},
			..Default::default()
		},
		Some(&system_test_project),
		Some(&system_test_env),
		None,
	)
	.instrument(tracing::info_span!("actor create request"))
	.await?;
	let actor_id = res.actor.id;

	tracing::info!(?actor_id, "created actor");

	let port = unwrap!(res.actor.network.ports.get("http"), "missing http protocol");

	let protocol = match port.protocol {
		models::ActorsPortProtocol::Http | models::ActorsPortProtocol::Tcp => "http",
		models::ActorsPortProtocol::Https => "https",
		_ => bail!("unsupported protocol"),
	};
	let hostname = unwrap_ref!(port.hostname);
	let port = unwrap!(port.port);
	let actor_origin = format!("{protocol}://{hostname}:{port}");

	// Test connection, defer error
	let test_res = tokio::time::timeout(
		Duration::from_secs(15),
		test_actor_connection(hostname, port.try_into()?, &actor_origin),
	)
	.await;

	// Destroy actor regardless of connection status
	actors_api::actors_destroy(
		&config,
		&actor_id.to_string(),
		Some(&system_test_project),
		Some(&system_test_env),
		None,
	)
	.instrument(tracing::info_span!("actor destroy request"))
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
				error = "test actor connection timed out"
			)
		}
	}

	Ok(serde_json::json!({}))
}

#[tracing::instrument]
async fn test_actor_connection(hostname: &str, port: u16, actor_origin: &str) -> GlobalResult<()> {
	// Look up IP for the actor's host
	let gg_ips = lookup_dns(hostname).await?;
	ensure_with!(
		!gg_ips.is_empty(),
		INTERNAL_STATUS_CHECK_FAILED,
		error = format!("no IPs found for host {hostname}")
	);

	// Test HTTP connectivity
	let test_http_res = futures_util::future::join_all(gg_ips.iter().cloned().map(|x| {
		test_http(
			actor_origin.to_string(),
			hostname.to_string(),
			(x, port).into(),
		)
	}))
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

	// Test WebSocket connectivity
	test_ws(actor_origin).await.map_err(|err| {
		err_code!(
			INTERNAL_STATUS_CHECK_FAILED,
			error = format!("ws failed: {err:?}")
		)
	})?;

	Ok(())
}

/// Returns the IP addresses for a given hostname.
#[tracing::instrument]
async fn lookup_dns(hostname: &str) -> GlobalResult<Vec<IpAddr>> {
	let resolver = hickory_resolver::Resolver::builder_tokio()?.build();
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
#[tracing::instrument]
async fn test_http(
	actor_origin: String,
	hostname: String,
	addr: SocketAddr,
) -> Result<(), reqwest::Error> {
	// Resolve the host to the specific IP addr
	let client = reqwest::Client::builder()
		.resolve(&hostname, addr)
		.build()?;

	// Test HTTP connectivity
	client
		.get(format!("{actor_origin}/health"))
		.send()
		.instrument(tracing::info_span!("health request"))
		.await?
		.error_for_status()?;

	Ok(())
}

/// Tests WebSocket connectivity to a hostname.
#[tracing::instrument]
async fn test_ws(actor_origin: &str) -> GlobalResult<()> {
	let actor_origin = actor_origin
		.replace("http://", "ws://")
		.replace("https://", "wss://");

	let (mut socket, _) = connect_async(format!("{actor_origin}/ws")).await?;

	tokio::time::sleep(Duration::from_millis(500)).await;

	socket.close(None).await?;

	Ok(())
}
