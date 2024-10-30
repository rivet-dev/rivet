use proto::backend::pkg::*;
use rivet_operation::prelude::*;

// TODO: Should this token live forever or a shorter period of time?
// This token is printed on startup. It should be accessible if a dev checks the logs much later.
const TOKEN_TTL: i64 = util::duration::hours(24 * 7);

const DEFAULT_USERNAME: &'static str = "admin";

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	// Check if enabled
	let auth_config = &config.server()?.rivet.auth;
	if !auth_config.access_token_login || !auth_config.print_login_url {
		tracing::debug!("skipping print admin login url");
		return Ok(());
	}

	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("admin-default-login-url");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"admin-default-login-url".into(),
		std::time::Duration::from_secs(60),
		config,
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
	);

	let token_res = op!([ctx] token_create {
		issuer: "admin-default-login-url".to_string(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: TOKEN_TTL,
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::AccessToken(proto::claims::entitlement::AccessToken {
							name: DEFAULT_USERNAME.to_string(),
						})
					)
				}
			],
		})),
		label: Some("access".to_string()),
		..Default::default()
	})
	.await?;

	let access_token_token = unwrap!(token_res.token).token;
	let access_token_link_url = util::route::access_token_link(ctx.config(), &access_token_token);

	tracing::info!(url = ?access_token_link_url, "admin login url");

	Ok(())
}
