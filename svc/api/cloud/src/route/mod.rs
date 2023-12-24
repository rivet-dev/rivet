use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;
use uuid::Uuid;

pub mod auth;
pub mod bootstrap;
pub mod devices;
pub mod games;
pub mod groups;
pub mod logs;
pub mod tiers;
pub mod uploads;

pub async fn handle(
	shared_client: chirp_client::SharedClientHandle,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	request: Request<Body>,
) -> Result<Response<Body>, http::Error> {
	let response = Response::builder();

	// Handle route
	Router::handle(shared_client, pools, cache, ray_id, request, response).await
}

define_router! {
	cors: CorsConfigBuilder::hub().build(),
	routes: {
		"bootstrap": {
			GET: bootstrap::get(
				opt_auth: true,
			),
		},

		// Auth
		"auth" / "inspect": {
			GET: auth::inspect(),
		},

		// Uploads
		"uploads" / Uuid / "complete": {
			POST: uploads::complete(body: serde_json::Value),
		},

		// Games
		"games": {
			GET: games::list(),
			POST: games::create(body: models::CloudGamesCreateGameRequest),
		},
		"games" / Uuid: {
			GET: games::get(),
		},
		"games" / Uuid / "versions": {
			POST: games::versions::create(body: models::CloudGamesCreateGameVersionRequest),
		},
		"games" / Uuid / "versions" / "reserve-name": {
			POST: games::versions::reserve_name(body: serde_json::Value),
		},
		"games" / Uuid / "versions" / Uuid: {
			GET: games::versions::get(),
		},
		"games" / Uuid / "namespaces": {
			POST: games::namespaces::create(body: models::CloudGamesNamespacesCreateGameNamespaceRequest),
		},
		"games" / Uuid / "namespaces" / Uuid: {
			GET: games::namespaces::get(),
		},
		"games" / Uuid / "namespaces" / Uuid / "version": {
			PUT: games::namespaces::update_version(body: models::CloudGamesNamespacesUpdateGameNamespaceVersionRequest),
		},
		"games" / Uuid / "namespaces" / Uuid / "tokens" / "public": {
			POST: games::namespaces::create_token_public(
				body: serde_json::Value
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "tokens" / "development": {
			POST: games::namespaces::create_token_development(
				body: models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "domains": {
			POST: games::namespaces::add_namespace_domain(
				body: models::CloudGamesNamespacesAddNamespaceDomainRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "domains" / String: {
			DELETE: games::namespaces::remove_namespace_domain(),
		},
		"games" / Uuid / "namespaces" / Uuid / "domain-public-auth": {
			PUT: games::namespaces::toggle_domain_public_auth(
				body: models::CloudGamesNamespacesToggleNamespaceDomainPublicAuthRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "auth-user": {
			POST: games::namespaces::update_namespace_cdn_auth_user(
				body: models::CloudGamesNamespacesUpdateNamespaceCdnAuthUserRequest,
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "auth-user"/ String: {
			DELETE: games::namespaces::remove_namespace_cdn_auth_user(),
		},
		"games" / Uuid / "namespaces" / Uuid / "cdn-auth": {
			PUT: games::namespaces::set_cdn_auth_type(
				body: models::CloudGamesNamespacesSetNamespaceCdnAuthTypeRequest,
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "mm-config": {
			POST: games::namespaces::update_mm_config(
				body: models::CloudGamesNamespacesUpdateGameNamespaceMatchmakerConfigRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "version-history": {
			GET: games::namespaces::get_version_history(
				query: games::namespaces::GetGameNamespaceGetVersionHistoryQuery,
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "analytics" / "matchmaker" / "live": {
			GET: games::namespaces::analytics::matchmaker_live(),
		},
		"games" / Uuid / "namespaces" / Uuid / "logs" / "lobbies": {
			GET: games::namespaces::logs::list_lobbies(
				query: games::namespaces::logs::ListNamespaceLobbiesQuery,
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "logs" / "lobbies" / Uuid: {
			GET: games::namespaces::logs::get_lobby(),
		},
		"games" / Uuid / "cdn" / "sites": {
			GET: games::cdn::get_sites(),
			POST: games::cdn::create_site(body: models::CloudGamesCreateGameCdnSiteRequest),
		},
		"games" / Uuid / "builds": {
			GET: games::builds::get_builds(),
			POST: games::builds::create_build(body: models::CloudGamesCreateGameBuildRequest),
		},
		"games" / Uuid / "avatars": {
			GET: games::avatars::get_custom_avatars(),
		},
		"games" / Uuid / "avatar-upload" / "prepare": {
			POST: games::avatars::prepare_avatar_upload(body: models::CloudGamesPrepareCustomAvatarUploadRequest),
		},
		"games" / Uuid / "avatar-upload" / Uuid / "complete": {
			POST: games::avatars::complete_avatar_upload(body: serde_json::Value),
		},
		"games" / Uuid / "tokens" / "cloud": {
			POST: games::tokens::create_cloud_token(body: serde_json::Value),
		},
		"games" / Uuid / "matchmaker" / "lobbies" / "export-history": {
			POST: games::matchmaker::export_history(body: models::CloudGamesExportMatchmakerLobbyHistoryRequest),
		},
		"games" / Uuid / "matchmaker" / "lobbies" / Uuid: {
			DELETE: games::matchmaker::delete_lobby(),
		},
		"games" / Uuid / "matchmaker" / "lobbies" / Uuid / "logs": {
			GET: games::matchmaker::get_lobby_logs(
				query: games::matchmaker::GetLobbyLogsQuery,
			),
		},
		"games" / Uuid / "matchmaker" / "lobbies" / Uuid / "logs" / "export": {
			POST: games::matchmaker::export_lobby_logs(
				body: models::CloudGamesExportLobbyLogsRequest,
			),
		},
		"games" / "validate": {
			POST: games::validate(body: models::CloudGamesValidateGameRequest),
		},
		"games" / Uuid / "logo-upload" / "prepare": {
			POST: games::prepare_logo_upload(body: models::CloudGamesGameLogoUploadPrepareRequest),
		},
		"games" / Uuid / "logo-upload" / Uuid / "complete": {
			POST: games::complete_logo_upload(body: serde_json::Value),
		},
		"games" / Uuid / "banner-upload" / "prepare": {
			POST: games::prepare_banner_upload(body: models::CloudGamesGameBannerUploadPrepareRequest),
		},
		"games" / Uuid / "banner-upload" / Uuid / "complete": {
			POST: games::complete_banner_upload(body: serde_json::Value),
		},
		"games" / Uuid / "versions" / "validate": {
			POST: games::versions::validate(body: models::CloudGamesValidateGameVersionRequest),
		},
		"games" / Uuid / "namespaces" / "validate": {
			POST: games::namespaces::validate(body: models::CloudGamesNamespacesValidateGameNamespaceRequest),
		},
		"games" / Uuid / "namespaces" / Uuid / "tokens" / "development" / "validate": {
			POST: games::namespaces::validate_token_development(
				body: models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "mm-config" / "validate": {
			POST: games::namespaces::validate_mm_config(
				body: models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigRequest
			),
		},

		// Groups
		"groups" / Uuid / "convert": {
			POST: groups::convert(body: serde_json::Value),
		},
		"groups" / "validate": {
			POST: groups::validate(body: models::CloudValidateGroupRequest),
		},

		// Tiers
		"region-tiers": {
			GET: tiers::list_tiers(opt_auth: true),
		},

		// Logs
		"rays" / Uuid / "perf": {
			GET: logs::get_ray_perf(),
		},

		// Device links
		"devices" / "links": {
			GET: devices::links::get(
				query: devices::links::GetQuery,
				opt_auth: true,
			),
			POST: devices::links::prepare(
				body: serde_json::Value,
				opt_auth: true,
			),
		},
		"devices" / "links" / "complete": {
			POST: devices::links::complete(body: models::CloudDevicesCompleteDeviceLinkRequest),
		},
	},
}
