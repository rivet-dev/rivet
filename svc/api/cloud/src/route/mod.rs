use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models as new_models;
use rivet_cloud_server::models;
use uuid::Uuid;

mod auth;
mod bootstrap;
mod devices;
mod games;
mod groups;
mod logs;
mod tiers;
mod uploads;

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
			GET: bootstrap::get(),
		},

		// Auth
		"auth" / "inspect": {
			GET: auth::inspect(),
		},

		// Uploads
		"uploads" / Uuid / "complete": {
			POST: uploads::complete(body: models::CompleteUploadRequest),
		},

		// Games
		"games": {
			GET: games::list(),
			POST: games::create(body: models::CreateGameRequest),
		},
		"games" / Uuid: {
			GET: games::get(),
		},
		"games" / Uuid / "versions": {
			POST: games::versions::create(body: new_models::CloudGamesCreateGameVersionRequest),
		},
		"games" / Uuid / "versions" / Uuid: {
			GET: games::versions::get(),
		},
		"games" / Uuid / "namespaces": {
			POST: games::namespaces::create(body: new_models::CloudGamesNamespacesCreateGameNamespaceRequest),
		},
		"games" / Uuid / "namespaces" / Uuid: {
			GET: games::namespaces::get(),
		},
		"games" / Uuid / "namespaces" / Uuid / "version": {
			PUT: games::namespaces::update_version(body: new_models::CloudGamesNamespacesUpdateGameNamespaceVersionRequest),
		},
		"games" / Uuid / "namespaces" / Uuid / "tokens" / "public": {
			POST: games::namespaces::create_token_public(
				body: serde_json::Value
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "tokens" / "development": {
			POST: games::namespaces::create_token_development(
				body: new_models::CloudGamesNamespacesCreateGameNamespaceTokenDevelopmentRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "domains": {
			POST: games::namespaces::add_namespace_domain(
				body: new_models::CloudGamesNamespacesAddNamespaceDomainRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "domains" / String: {
			DELETE: games::namespaces::remove_namespace_domain(),
		},
		"games" / Uuid / "namespaces" / Uuid / "domain-public-auth": {
			PUT: games::namespaces::toggle_domain_public_auth(
				body: new_models::CloudGamesNamespacesToggleNamespaceDomainPublicAuthRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "auth-user": {
			POST: games::namespaces::update_namespace_cdn_auth_user(
				body: new_models::CloudGamesNamespacesUpdateNamespaceCdnAuthUserRequest,
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "auth-user"/ String: {
			DELETE: games::namespaces::remove_namespace_cdn_auth_user(),
		},
		"games" / Uuid / "namespaces" / Uuid / "cdn-auth": {
			PUT: games::namespaces::set_cdn_auth_type(
				body: new_models::CloudGamesNamespacesSetNamespaceCdnAuthTypeRequest,
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "mm-config": {
			POST: games::namespaces::update_mm_config(
				body: new_models::CloudGamesNamespacesUpdateGameNamespaceMatchmakerConfigRequest
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
			POST: games::cdn::create_site(body: new_models::CloudGamesCreateGameCdnSiteRequest),
		},
		"games" / Uuid / "builds": {
			GET: games::builds::get_builds(),
			POST: games::builds::create_build(body: new_models::CloudGamesCreateGameBuildRequest),
		},
		"games" / Uuid / "avatars": {
			GET: games::avatars::get_custom_avatars(),
		},
		"games" / Uuid / "avatar-upload" / "prepare": {
			POST: games::avatars::prepare_avatar_upload(body: new_models::CloudGamesPrepareCustomAvatarUploadRequest),
		},
		"games" / Uuid / "avatar-upload" / Uuid / "complete": {
			POST: games::avatars::complete_avatar_upload(body: models::CompleteCustomAvatarUploadRequest),
		},
		"games" / Uuid / "tokens" / "cloud": {
			POST: games::tokens::create_cloud_token(body: models::CreateCloudTokenRequest),
		},
		"games" / Uuid / "matchmaker" / "lobbies" / "export-history": {
			POST: games::matchmaker::export_history(body: models::ExportMatchmakerLobbyHistoryRequest),
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
				body: models::ExportLobbyLogsRequest,
			),
		},
		"games" / "validate": {
			POST: games::validate(body: models::ValidateGameRequest),
		},
		"games" / Uuid / "logo-upload" / "prepare": {
			POST: games::prepare_logo_upload(body: new_models::CloudGamesGameLogoUploadPrepareRequest),
		},
		"games" / Uuid / "logo-upload" / Uuid / "complete": {
			POST: games::complete_logo_upload(body: models::GameLogoUploadCompleteRequest),
		},
		"games" / Uuid / "banner-upload" / "prepare": {
			POST: games::prepare_banner_upload(body: new_models::CloudGamesGameBannerUploadPrepareRequest),
		},
		"games" / Uuid / "banner-upload" / Uuid / "complete": {
			POST: games::complete_banner_upload(body: models::GameBannerUploadCompleteRequest),
		},
		"games" / Uuid / "versions" / "validate": {
			POST: games::versions::validate(body: new_models::CloudGamesValidateGameVersionRequest),
		},
		"games" / Uuid / "namespaces" / "validate": {
			POST: games::namespaces::validate(body: new_models::CloudGamesNamespacesValidateGameNamespaceRequest),
		},
		"games" / Uuid / "namespaces" / Uuid / "tokens" / "development" / "validate": {
			POST: games::namespaces::validate_token_development(
				body: new_models::CloudGamesNamespacesValidateGameNamespaceTokenDevelopmentRequest
			),
		},
		"games" / Uuid / "namespaces" / Uuid / "mm-config" / "validate": {
			POST: games::namespaces::validate_mm_config(
				body: new_models::CloudGamesNamespacesValidateGameNamespaceMatchmakerConfigRequest
			),
		},

		// Groups
		"groups" / Uuid / "convert": {
			POST: groups::convert(body: models::ConvertGroupRequest),
		},
		"groups" / "validate": {
			POST: groups::validate(body: models::ValidateGroupRequest),
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
			POST: devices::links::complete(body: new_models::CloudDevicesCompleteDeviceLinkRequest),
		},
	},
}
