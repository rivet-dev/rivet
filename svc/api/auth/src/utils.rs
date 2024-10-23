use hyper::header::{self, HeaderName, HeaderValue};
use rivet_operation::prelude::*;
use url::Url;

use crate::route::tokens::{REFRESH_TOKEN_TTL, USER_REFRESH_TOKEN_COOKIE};

#[tracing::instrument(skip(config, refresh_token))]
fn build_cookie_header(
	config: &rivet_config::Config,
	origin: &Url,
	refresh_token: &str,
	max_age: i64,
) -> GlobalResult<String> {
	// Build base header
	//
	// Use `SameSite=None` because the hub is hosted on a separate subdomain than the API
	//
	// Localhost domains are considered secure, so we can leave `Secure` enabled even for local
	// development.
	let api_host = config.server()?.rivet.api_host()?;
	let mut header = format!(
		"{USER_REFRESH_TOKEN_COOKIE}={refresh_token}; Max-Age={max_age}; HttpOnly; Path=/; SameSite=None; Secure; Domain={api_host}",
	);

	tracing::info!(?header, "built cookie header");

	Ok(header)
}

/// We rely on CORS to make sure the origin is valid.
#[tracing::instrument(skip(config, refresh_token))]
pub fn refresh_token_header(
	config: &rivet_config::Config,
	origin: &Url,
	refresh_token: String,
) -> GlobalResult<(HeaderName, HeaderValue)> {
	Ok((
		header::SET_COOKIE,
		header::HeaderValue::from_str(&build_cookie_header(
			config,
			origin,
			&refresh_token,
			REFRESH_TOKEN_TTL / 1000,
		)?)
		.map_err(Into::<GlobalError>::into)?,
	))
}

#[tracing::instrument(skip(config))]
pub fn delete_refresh_token_header(
	config: &rivet_config::Config,
	origin: &Url,
) -> GlobalResult<(HeaderName, HeaderValue)> {
	Ok((
		header::SET_COOKIE,
		header::HeaderValue::from_str(&build_cookie_header(config, origin, "", 0)?)
			.map_err(Into::<GlobalError>::into)?,
	))
}
