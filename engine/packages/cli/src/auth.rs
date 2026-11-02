use std::{
	env,
	process::{Command as StdCommand, Stdio},
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;
use tokio::time::sleep;
use url::Url;

use crate::credentials::{Session, SessionUser, read_credentials, session_key, write_credentials};

const CLIENT_ID: &str = "rivet-cli";
const SCOPE: &str = "rivet:cloud:read rivet:cloud:write";
const REFRESH_SKEW_MS: i64 = 60_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
	Flag,
	Env,
	Login,
	LegacyFile,
}

impl AuthSource {
	pub fn describe(self) -> &'static str {
		match self {
			AuthSource::Flag => "--token flag",
			AuthSource::Env => "RIVET_CLOUD_TOKEN environment variable",
			AuthSource::Login => "rivet login",
			AuthSource::LegacyFile => "Cloud API token in ~/.rivet/credentials",
		}
	}
}

pub struct Auth {
	pub token: String,
	pub source: AuthSource,
	pub user: Option<SessionUser>,
}

impl Auth {
	pub fn is_login(&self) -> bool {
		self.source == AuthSource::Login
	}
}

/// Resolves credentials in order: the flag, `RIVET_CLOUD_TOKEN`, the `rivet
/// login` session for this endpoint, then the legacy token in
/// `~/.rivet/credentials`.
pub async fn resolve_auth(flag: Option<&str>, cloud_api: &str) -> Result<Auth> {
	if let Some(token) = flag {
		return Ok(Auth {
			token: token.to_string(),
			source: AuthSource::Flag,
			user: None,
		});
	}
	if let Ok(token) = env::var("RIVET_CLOUD_TOKEN") {
		if !token.trim().is_empty() {
			return Ok(Auth {
				token,
				source: AuthSource::Env,
				user: None,
			});
		}
	}

	let credentials = read_credentials()?;
	if let Some(session) = credentials.sessions.get(&session_key(cloud_api)) {
		let session = if session.expires_at - REFRESH_SKEW_MS <= now_ms() {
			refresh(cloud_api, session).await?
		} else {
			session.clone()
		};
		return Ok(Auth {
			token: session.access_token,
			source: AuthSource::Login,
			user: Some(session.user),
		});
	}

	if !credentials.rivet_cloud_token.trim().is_empty() {
		return Ok(Auth {
			token: credentials.rivet_cloud_token,
			source: AuthSource::LegacyFile,
			user: None,
		});
	}

	bail!(
		"not signed in to Rivet Cloud; run `rivet login`, or pass --token / set RIVET_CLOUD_TOKEN for CI"
	)
}

/// The access token of the saved `rivet login` session for `cloud_api`,
/// refreshed first when it is about to expire, or always when `force` is set.
/// A login access token lasts minutes, so long commands call this before each
/// request instead of reusing the token they started with.
pub async fn login_access_token(cloud_api: &str, force: bool) -> Result<String> {
	let credentials = read_credentials()?;
	let session = credentials
		.sessions
		.get(&session_key(cloud_api))
		.context("not signed in to Rivet Cloud; run `rivet login`")?;
	let session = if force || session.expires_at - REFRESH_SKEW_MS <= now_ms() {
		refresh(cloud_api, session).await?
	} else {
		session.clone()
	};
	Ok(session.access_token)
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
	device_code: String,
	user_code: String,
	verification_uri: String,
	verification_uri_complete: Option<String>,
	expires_in: u64,
	interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
	access_token: String,
	refresh_token: String,
	expires_in: i64,
	user: SessionUser,
}

#[derive(Deserialize)]
struct OAuthError {
	error: String,
	error_description: Option<String>,
}

pub async fn login(cloud_api: &str, open_browser: bool) -> Result<Session> {
	let http = reqwest::Client::new();
	let response = http
		.post(endpoint(cloud_api, "api/auth/device/code")?)
		.json(&json!({ "client_id": CLIENT_ID, "scope": SCOPE }))
		.send()
		.await
		.context("request device code")?;
	let status = response.status();
	let text = response.text().await.unwrap_or_default();
	if !status.is_success() {
		bail!("could not start sign-in ({status}): {text}");
	}
	let device: DeviceCodeResponse =
		serde_json::from_str(&text).context("invalid device code response")?;

	let user_code = format_user_code(&device.user_code);
	let url = device
		.verification_uri_complete
		.clone()
		.unwrap_or_else(|| device.verification_uri.clone());

	eprintln!();
	eprintln!("  Your one-time code: {user_code}");
	eprintln!();
	if open_browser && try_open_browser(&url) {
		eprintln!("  Opened {url} in your browser.");
	} else {
		eprintln!("  Open {url} in a browser to continue.");
	}
	eprintln!();
	eprintln!("waiting for approval...");

	let deadline = SystemTime::now() + Duration::from_secs(device.expires_in);
	let mut interval = Duration::from_secs(device.interval.max(1));
	loop {
		sleep(interval).await;
		if SystemTime::now() > deadline {
			bail!("the sign-in code expired; run `rivet login` again");
		}

		let response = http
			.post(endpoint(cloud_api, "api/auth/device/token")?)
			.json(&json!({
				"grant_type": "urn:ietf:params:oauth:grant-type:device_code",
				"device_code": device.device_code,
				"client_id": CLIENT_ID,
			}))
			.send()
			.await
			.context("poll device token")?;
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		if status.is_success() {
			let token: TokenResponse =
				serde_json::from_str(&text).context("invalid token response")?;
			let session = session_from(token);
			save_session(cloud_api, Some(&session))?;
			return Ok(session);
		}

		let error: OAuthError = serde_json::from_str(&text)
			.with_context(|| format!("sign-in failed ({status}): {text}"))?;
		match error.error.as_str() {
			"authorization_pending" => {}
			"slow_down" => interval += Duration::from_secs(5),
			"access_denied" => bail!("sign-in was denied in the browser"),
			"expired_token" => bail!("the sign-in code expired; run `rivet login` again"),
			_ => bail!(
				"sign-in failed: {}",
				error.error_description.unwrap_or(error.error)
			),
		}
	}
}

/// Revokes the session server-side on a best-effort basis and removes it
/// locally. Returns whether a session existed.
pub async fn logout(cloud_api: &str) -> Result<bool> {
	let credentials = read_credentials()?;
	let Some(session) = credentials.sessions.get(&session_key(cloud_api)) else {
		return Ok(false);
	};
	let revoke = reqwest::Client::new()
		.post(endpoint(cloud_api, "api/auth/device/logout")?)
		.json(&json!({ "refresh_token": session.refresh_token }))
		.send()
		.await;
	if let Err(error) = revoke {
		tracing::warn!(
			?error,
			"could not revoke session on the server; removing it locally"
		);
	}
	save_session(cloud_api, None)?;
	Ok(true)
}

async fn refresh(cloud_api: &str, session: &Session) -> Result<Session> {
	let response = reqwest::Client::new()
		.post(endpoint(cloud_api, "api/auth/device/refresh")?)
		.json(&json!({ "refresh_token": session.refresh_token }))
		.send()
		.await
		.context("refresh Rivet Cloud session")?;
	let status = response.status();
	let text = response.text().await.unwrap_or_default();
	if status.is_success() {
		let token: TokenResponse = serde_json::from_str(&text).context("invalid token response")?;
		let session = session_from(token);
		save_session(cloud_api, Some(&session))?;
		return Ok(session);
	}
	if status.is_client_error() {
		save_session(cloud_api, None)?;
		bail!("your Rivet Cloud session expired; run `rivet login` again");
	}
	bail!("could not refresh Rivet Cloud session ({status}): {text}")
}

fn session_from(token: TokenResponse) -> Session {
	Session {
		access_token: token.access_token,
		refresh_token: token.refresh_token,
		expires_at: now_ms() + token.expires_in * 1000,
		user: token.user,
	}
}

fn save_session(cloud_api: &str, session: Option<&Session>) -> Result<()> {
	let mut credentials = read_credentials()?;
	let key = session_key(cloud_api);
	match session {
		Some(session) => {
			credentials.sessions.insert(key, session.clone());
		}
		None => {
			credentials.sessions.remove(&key);
		}
	}
	write_credentials(&credentials)
}

fn endpoint(cloud_api: &str, path: &str) -> Result<Url> {
	let base = format!("{}/", cloud_api.trim_end_matches('/'));
	Ok(Url::parse(&base)
		.context("invalid Cloud API endpoint")?
		.join(path)?)
}

fn format_user_code(code: &str) -> String {
	if code.len() == 8 {
		format!("{}-{}", &code[..4], &code[4..])
	} else {
		code.to_string()
	}
}

fn try_open_browser(url: &str) -> bool {
	let mut command = if cfg!(target_os = "macos") {
		let mut command = StdCommand::new("open");
		command.arg(url);
		command
	} else if cfg!(target_os = "windows") {
		let mut command = StdCommand::new("cmd");
		command.args(["/C", "start", "", url]);
		command
	} else {
		if env::var_os("DISPLAY").is_none() && env::var_os("WAYLAND_DISPLAY").is_none() {
			return false;
		}
		let mut command = StdCommand::new("xdg-open");
		command.arg(url);
		command
	};
	command
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.is_ok_and(|status| status.success())
}

fn now_ms() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis() as i64
}
