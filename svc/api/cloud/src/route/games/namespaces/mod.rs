use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use rivet_api::{apis::*, models};
use rivet_claims::ClaimsDecode;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::{assert, auth::Auth};

pub mod analytics;
pub mod logs;

// MARK: GET /games/{}/namespaces/{}
pub async fn get(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::CloudGamesNamespacesGetGameNamespaceByIdResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;
	let game_namespace = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	// Fetch the namespace with config
	let (ns_res, games_res, custom_hostnames_res) = tokio::try_join!(
		op!([ctx] cloud_namespace_get {
			namespace_ids: vec![namespace_id.into()],
		}),
		op!([ctx] game_get {
			game_ids: vec![game_id.into()],
		}),
		op!([ctx] cf_custom_hostname_list_for_namespace_id {
			namespace_ids: vec![namespace_id.into()],
			pending_only: false,
		})
	)?;
	let ns_config = internal_unwrap_owned!(ns_res.namespaces.first());
	let ns_config = internal_unwrap!(ns_config.config).clone();

	// Fetch domain verification statuses from cloudflare
	let cdn_ns_config = {
		let cdn = internal_unwrap!(ns_config.cdn).clone();
		let game = internal_unwrap_owned!(games_res.games.first());
		let custom_hostname_namespace =
			internal_unwrap_owned!(custom_hostnames_res.namespaces.first());
		let client = reqwest::Client::new();

		let hostnames_res =
			futures_util::stream::iter(custom_hostname_namespace.identifiers.iter().cloned().map(
				|identifier| {
					let chirp = ctx.chirp().clone();
					let client = client.clone();

					async move {
						if util::feature::cf_custom_hostname() {
							let game_zone_id =
								internal_unwrap_owned!(util::env::cloudflare::zone::game::id());
							let res = client
								.get(format!(
							"https://api.cloudflare.com/client/v4/zones/{game_zone_id}/custom_hostnames/{identifier}",
							identifier = identifier,
						))
								.header(
									reqwest::header::AUTHORIZATION,
									format!("Bearer {}", util::env::cloudflare::auth_token()),
								)
								.send()
								.await?;
							let status = res.status();

							if status.is_success() {
								let res = res
									.json::<rivet_convert::cloud::cloudflare::CloudflareResponse>()
									.await?;

								// Send message to update the hostname's status in our DB
								msg!([chirp] cf_custom_hostname::msg::status_set(identifier) {
									identifier: Some(identifier),
									status: ApiInto::<backend::cf::custom_hostname::Status>::api_into(res.result.status.clone()) as i32,
								})
								.await?;

								GlobalResult::Ok(Some(res.result))
							} else {
								let body = res.text().await?;
								tracing::error!(?body, ?status, "failed to fetch custom hostname");
								Ok(None)
							}
						} else {
							tracing::trace!("custom hostnames disabled");
							Ok(None)
						}
					}
				},
			))
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		let domains = cdn
			.domains
			.into_iter()
			.map(|domain| {
				// Find matching custom hostname record
				if let Some(custom_hostname) = hostnames_res
					.iter()
					.find(|res| res.hostname == domain.domain)
				{
					let domain_cdn = internal_unwrap_owned!(util::env::domain_cdn());

					// Create CNAME url
					let cname_url = if game_namespace.name_id.as_str() == "prod" {
						format!("{}.{domain_cdn}", game.name_id)
					} else {
						format!("{}--{}.{domain_cdn}", game.name_id, game_namespace.name_id,)
					};

					Ok(models::CloudCdnNamespaceDomain {
						domain: domain.domain,
						create_ts: util::timestamp::to_string(domain.create_ts)?,
						verification_status: custom_hostname.status.clone().api_into(),
						verification_method: Box::new(
							models::CloudCdnNamespaceDomainVerificationMethod {
								http: Some(Box::new(
									models::CloudCdnNamespaceDomainVerificationMethodHttp {
										cname_record: cname_url,
									},
								)),
								invalid: None,
							},
						),
						verification_errors: custom_hostname.verification_errors.clone(),
					})
				} else {
					tracing::warn!(hostname=%domain.domain, "missing hostname");

					Ok(models::CloudCdnNamespaceDomain {
						domain: domain.domain,
						create_ts: util::timestamp::to_string(domain.create_ts)?,
						verification_status:
							models::CloudCdnNamespaceDomainVerificationStatus::Failed,
						verification_method: Box::new(
							models::CloudCdnNamespaceDomainVerificationMethod {
								invalid: Some(serde_json::json!({})),
								http: None,
							},
						),
						verification_errors: vec!["Custom hostname record not found.".to_string()],
					})
				}
			})
			.collect::<GlobalResult<Vec<_>>>()?;

		models::CloudCdnNamespaceConfig {
			enable_domain_public_auth: cdn.enable_domain_public_auth,
			domains,
			auth_type: internal_unwrap_owned!(backend::cdn::namespace_config::AuthType::from_i32(
				cdn.auth_type
			))
			.api_into(),
			auth_user_list: cdn
				.auth_user_list
				.into_iter()
				.map(ApiInto::api_into)
				.collect::<Vec<_>>(),
		}
	};

	let summary = ApiTryInto::<models::CloudNamespaceSummary>::try_into(game_namespace)?;
	let namespace = models::CloudNamespaceFull {
		namespace_id: summary.namespace_id,
		create_ts: summary.create_ts,
		display_name: summary.display_name,
		version_id: summary.version_id,
		name_id: summary.name_id,

		config: Box::new(models::CloudNamespaceConfig {
			cdn: Box::new(cdn_ns_config),
			matchmaker: Box::new(internal_unwrap!(ns_config.matchmaker).clone().try_into()?),
			kv: serde_json::json!({}),
			identity: serde_json::json!({}),
		}),
	};

	Ok(models::CloudGamesNamespacesGetGameNamespaceByIdResponse {
		namespace: Box::new(namespace),
	})
}

// MARK: POST /games/{}/namespaces
pub async fn create(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::CloudGamesNamespacesCreateGameNamespaceRequest,
) -> GlobalResult<models::CloudGamesNamespacesCreateGameNamespaceResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	assert::version_for_game(&ctx, game_id, body.version_id).await?;

	let create_ns_res = op!([ctx] game_namespace_create {
		game_id: Some(game_id.into()),
		display_name: body.display_name.clone(),
		version_id: Some(body.version_id.into()),
		name_id: body.name_id.clone(),
	})
	.await?;
	let namespace_id = internal_unwrap!(create_ns_res.namespace_id).as_uuid();

	op!([ctx] cloud_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(models::CloudGamesNamespacesCreateGameNamespaceResponse { namespace_id })
}

// MARK: POST /games/{}/namespaces/{}/version
pub async fn update_version(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesUpdateGameNamespaceVersionRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;
	assert::version_for_game(&ctx, game_id, body.version_id).await?;

	let user_id = ctx.auth().claims()?.as_user().ok();

	// Set the version
	let prev_version_id = internal_unwrap!(ns_data.version_id).as_uuid();
	if prev_version_id != body.version_id {
		op!([ctx] game_namespace_version_set {
			namespace_id: Some(namespace_id.into()),
			version_id: Some(body.version_id.into()),
			creator_user_id: user_id.as_ref().map(|x| x.user_id.into()),
		})
		.await?;
	} else {
		// Do nothing since version is already set
	}

	Ok(serde_json::json!({}))
}

// MARK: POST /games/{}/namespaces/{}/tokens/public
pub async fn create_token_public(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<models::CloudGamesNamespacesCreateGameNamespaceTokenPublicResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	let create_res = op!([ctx] cloud_namespace_token_public_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(
		models::CloudGamesNamespacesCreateGameNamespaceTokenPublicResponse {
			token: create_res.token,
		},
	)
}

// MARK: POST /games/{}/namespaces/{}/tokens/development
pub async fn create_token_development(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentRequest,
) -> GlobalResult<models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	let lobby_ports = match (body.ports, body.lobby_ports) {
		(Some(_), Some(_)) => panic_with!(
			CLOUD_INVALID_CONFIG,
			error = "can not specify both `ports` and `lobby_ports`"
		),
		(Some(ports), None) => ports
			.into_iter()
			.map(|(label, port)| {
				Ok(backend::matchmaker::lobby_runtime::Port {
					label,
					target_port: if let Some(port) = port.port {
						assert_with!(
							port >= 0,
							CLOUD_INVALID_CONFIG,
							error = "`port` out of bounds"
						);

						Some(port.try_into()?)
					} else {
						None
					},
					port_range: if let Some(port_range) = port.port_range {
						assert_with!(
							port_range.min >= 0,
							CLOUD_INVALID_CONFIG,
							error = "`port_range.min` out of bounds"
						);
						assert_with!(
							port_range.max >= 0,
							CLOUD_INVALID_CONFIG,
							error = "`port_range.max` out of bounds"
						);

						Some(backend::matchmaker::lobby_runtime::PortRange {
							min: port_range.min.try_into()?,
							max: port_range.max.try_into()?,
						})
					} else {
						None
					},
					proxy_protocol:
						(ApiInto::<backend::matchmaker::lobby_runtime::ProxyProtocol>::api_into(
							port.protocol,
						)) as i32,
					proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?,
		(None, Some(ports)) => {
			// Deprecated
			ports
				.into_iter()
				.map(|port| {
					let target_port = unwrap_with_owned!(
						port.target_port,
						CLOUD_INVALID_CONFIG,
						error = "expected `target_port`"
					);
					assert_with!(
						target_port >= 0,
						CLOUD_INVALID_CONFIG,
						error = "`target_port` out of bounds"
					);

					Ok(backend::matchmaker::lobby_runtime::Port {
						label: port.label,
						target_port: Some(target_port.try_into()?),
						port_range: None,
						proxy_protocol:
							(ApiInto::<backend::matchmaker::lobby_runtime::ProxyProtocol>::api_into(
								port.proxy_protocol,
							)) as i32,
						proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
					})
				})
				.collect::<GlobalResult<Vec<_>>>()?
		}
		(None, None) => {
			internal_panic!("no ports provided")
		}
	};

	let create_res = op!([ctx] cloud_namespace_token_development_create {
		namespace_id: Some(namespace_id.into()),
		hostname: body.hostname,
		lobby_ports: lobby_ports,
	})
	.await?;

	Ok(
		models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentResponse {
			token: create_res.token,
		},
	)
}

// MARK: POST /games/{}/namespaces/{}/domains
pub async fn add_namespace_domain(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesAddNamespaceDomainRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: body.domain.to_owned(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: DELETE /games/{}/namespaces/{}/domains/{}
pub async fn remove_namespace_domain(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	domain: String,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	op!([ctx] cdn_namespace_domain_remove {
		namespace_id: Some(namespace_id.into()),
		domain: domain.to_owned(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: PUT /games/{}/namespaces/{}/domain-public-auth
pub async fn toggle_domain_public_auth(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesToggleNamespaceDomainPublicAuthRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	op!([ctx] cdn_ns_enable_domain_public_auth_set {
		namespace_id: ns_data.namespace_id,
		enable_domain_public_auth: body.enabled,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: POST /games/{}/namespaces/{}/cdn-auth-user
pub async fn update_namespace_cdn_auth_user(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesUpdateNamespaceCdnAuthUserRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	op!([ctx] cdn_namespace_auth_user_update {
		namespace_id: Some(namespace_id.into()),
		user: body.user.to_owned(),
		password: body.password.to_owned(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: DELETE /games/{}/namespaces/{}/cdn-auth-user/{}
pub async fn remove_namespace_cdn_auth_user(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	auth_user: String,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	op!([ctx] cdn_namespace_auth_user_remove {
		namespace_id: Some(namespace_id.into()),
		user: auth_user.to_owned(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: PUT /games/{}/namespaces/{}/cdn-auth
pub async fn set_cdn_auth_type(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesSetNamespaceCdnAuthTypeRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	op!([ctx] cdn_ns_auth_type_set {
		namespace_id: ns_data.namespace_id,
		auth_type: ApiInto::<backend::cdn::namespace_config::AuthType>::api_into(
			body.auth_type
		) as i32,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: POST /games/{}/namespaces/{}/mm-config
pub async fn update_mm_config(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesUpdateGameNamespaceMatchmakerConfigRequest,
) -> GlobalResult<serde_json::Value> {
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	assert_with!(
		body.lobby_count_max >= 0,
		CLOUD_INVALID_CONFIG,
		error = "`lobby_count_max` out of bounds"
	);
	assert_with!(
		body.max_players >= 0,
		CLOUD_INVALID_CONFIG,
		error = "`max_players` out of bounds"
	);

	let _res = op!([ctx] mm_config_namespace_config_set {
		namespace_id: Some(namespace_id.into()),
		lobby_count_max: body.lobby_count_max.try_into()?,
		max_players_per_client: body.max_players.try_into()?,
		max_players_per_client_vpn: body.max_players.try_into()?,
		max_players_per_client_proxy: body.max_players.try_into()?,
		max_players_per_client_tor: body.max_players.try_into()?,
		max_players_per_client_hosting: body.max_players.try_into()?,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: GET /games/{}/namespaces/{}/version-history
#[derive(Debug, Deserialize)]
pub struct GetGameNamespaceGetVersionHistoryQuery {
	limit: Option<u32>,
	anchor: Option<String>,
}

pub async fn get_version_history(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GetGameNamespaceGetVersionHistoryQuery,
) -> GlobalResult<models::CloudGamesNamespacesGetGameNamespaceVersionHistoryResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	let version_history_res = op!([ctx] game_namespace_version_history_list {
		namespace_ids: vec![namespace_id.into()],
		limit: query.limit.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let version_history = internal_unwrap_owned!(version_history_res.namespaces.first());

	Ok(
		models::CloudGamesNamespacesGetGameNamespaceVersionHistoryResponse {
			versions: version_history
				.versions
				.iter()
				.map(|version| {
					Ok(models::CloudNamespaceVersion {
						namespace_id: internal_unwrap!(version_history.namespace_id).to_string(),
						version_id: internal_unwrap!(version.version_id).to_string(),
						deploy_ts: util::timestamp::to_chrono(version.deploy_ts)?
							.to_rfc3339_openapi(),
					})
				})
				.collect::<GlobalResult<Vec<_>>>()?,
		},
	)
}

// MARK: POST /games/{}/namespace/validate
pub async fn validate(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::CloudGamesNamespacesValidateGameNamespaceRequest,
) -> GlobalResult<models::CloudGamesNamespacesValidateGameNamespaceResponse> {
	let res = op!([ctx] game_namespace_validate {
		game_id: Some(game_id.into()),
		name_id: body.name_id,
		display_name: body.display_name,
	})
	.await?;

	Ok(models::CloudGamesNamespacesValidateGameNamespaceResponse {
		errors: res
			.errors
			.into_iter()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
	})
}

// MARK: POST /games/{}/namespaces/{}/tokens/development/validate
pub async fn validate_token_development(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentRequest,
) -> GlobalResult<models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentResponse> {
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	let res = op!([ctx] game_token_development_validate {
		hostname: body.hostname,
		lobby_ports: body.lobby_ports
		.clone()
			.into_iter()
			.map(ApiTryInto::try_into)
			.collect::<GlobalResult<_>>()?,
	})
	.await?;

	Ok(
		models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentResponse {
			errors: res
				.errors
				.into_iter()
				.map(ApiInto::api_into)
				.collect::<Vec<_>>(),
		},
	)
}

// MARK: POST /games/{}/namespaces/{}/mm-config/validate
pub async fn validate_mm_config(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	namespace_id: Uuid,
	body: models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigRequest,
) -> GlobalResult<models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigResponse> {
	let _ns_data = assert::namespace_for_game(&ctx, game_id, namespace_id).await?;

	assert_with!(
		body.lobby_count_max >= 0,
		CLOUD_INVALID_CONFIG,
		error = "`lobby_count_max` out of bounds"
	);
	assert_with!(
		body.max_players >= 0,
		CLOUD_INVALID_CONFIG,
		error = "`max_players` out of bounds"
	);

	let res = op!([ctx] mm_config_namespace_config_validate {
		namespace_id: Some(namespace_id.into()),
		lobby_count_max: body.lobby_count_max.try_into()?,
		max_players_per_client: body.max_players.try_into()?,
		max_players_per_client_vpn: body.max_players.try_into()?,
		max_players_per_client_proxy: body.max_players.try_into()?,
		max_players_per_client_tor: body.max_players.try_into()?,
		max_players_per_client_hosting: body.max_players.try_into()?,
	})
	.await?;

	Ok(
		models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigResponse {
			errors: res
				.errors
				.into_iter()
				.map(ApiInto::api_into)
				.collect::<Vec<_>>(),
		},
	)
}
