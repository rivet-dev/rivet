use std::{fmt, time::Duration};

use rand::{distributions::Alphanumeric, Rng};
use reqwest::header;
use rivet_operation::prelude::*;
use serde::{de::DeserializeOwned, Deserialize};

pub mod consts;
pub mod api;

#[derive(Clone)]
pub struct Client {
	// Safe to clone, has inner Arc
	inner: reqwest::Client,
	max_retries: u8,
}

impl Client {
	pub async fn new(api_token: Option<String>) -> GlobalResult<Self> {
		let api_token = if let Some(api_token) = api_token {
			api_token
		} else {
			util::env::read_secret(&["linode", "token"]).await?
		};

		let auth = format!("Bearer {}", api_token);
		let mut headers = header::HeaderMap::new();
		headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&auth)?);

		let client = reqwest::Client::builder()
			.default_headers(headers)
			.build()?;

		Ok(Client {
			inner: client,
			max_retries: 8,
		})
	}

	pub async fn new_with_headers(api_token: Option<String>, mut headers: header::HeaderMap) -> GlobalResult<Self> {
		let api_token = if let Some(api_token) = api_token {
			api_token
		} else {
			util::env::read_secret(&["linode", "token"]).await?
		};
		
		let api_token = util::env::read_secret(&["linode", "token"]).await?;
		let auth = format!("Bearer {}", api_token);
		headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&auth)?);

		let client = reqwest::Client::builder()
			.default_headers(headers)
			.build()?;

		Ok(Client {
			inner: client,
			max_retries: 8,
		})
	}

	pub fn inner(&self) -> &reqwest::Client {
		&self.inner
	}

	async fn request(
		&self,
		req: reqwest::RequestBuilder,
		body: Option<serde_json::Value>,
		skip_404: bool,
	) -> GlobalResult<reqwest::Response> {
		let mut retries = 0;

		loop {
			let req = if let Some(body) = &body {
				unwrap!(req.try_clone()).json(body)
			} else {
				unwrap!(req.try_clone())
			};
			let res = req.send().await?;

			if !res.status().is_success() {
				match res.status() {
					reqwest::StatusCode::TOO_MANY_REQUESTS => {
						if retries >= self.max_retries {
							tracing::info!("all retry attempts failed");
						} else {
							tracing::info!("being rate limited, retrying");

							retries += 1;

							let retry_time = res
								.headers()
								.get("Retry-After")
								.map(|x| x.to_str())
								.transpose()?
								.map(|x| x.parse::<u64>())
								.transpose()?
								.unwrap_or(5);
							tokio::time::sleep(Duration::from_secs(retry_time)).await;

							continue;
						}
					}
					reqwest::StatusCode::NOT_FOUND => {
						if skip_404 {
							tracing::info!("resource not found");
							break Ok(res);
						}
					}
					_ => {}
				}

				tracing::info!(status=?res.status(), "api request failed");
				bail!(res.json::<ApiErrorResponse>().await?.to_string());
			}

			break Ok(res);
		}
	}

	pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> GlobalResult<T> {
		let res = self
			.request(
				self.inner
					.get(&format!("https://api.linode.com/v4{endpoint}")),
				None,
				false,
			)
			.await?;

		res.json::<T>().await.map_err(|err| err.into())
	}

	pub async fn delete(&self, endpoint: &str) -> GlobalResult<()> {
		self.request(
			self.inner
				.delete(&format!("https://api.linode.com/v4{endpoint}")),
			None,
			true,
		)
		.await?;

		Ok(())
	}

	pub async fn post<T: DeserializeOwned>(
		&self,
		endpoint: &str,
		body: serde_json::Value,
	) -> GlobalResult<T> {
		let res = self
			.request(
				self.inner
					.post(&format!("https://api.linode.com/v4{endpoint}"))
					.header("content-type", "application/json"),
				Some(body),
				false,
			)
			.await?;

		res.json::<T>().await.map_err(|err| err.into())
	}

	pub async fn post_no_res(&self, endpoint: &str, body: serde_json::Value) -> GlobalResult<()> {
		self.request(
			self.inner
				.post(&format!("https://api.linode.com/v4{endpoint}"))
				.header("content-type", "application/json"),
			Some(body),
			false,
		)
		.await?;

		Ok(())
	}
}

#[derive(Deserialize)]
pub struct ApiErrorResponse {
	errors: Vec<ApiError>,
}

impl fmt::Display for ApiErrorResponse {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for error in &self.errors {
			if let Some(field) = &error.field {
				write!(f, "{:?}: ", field)?;
			}

			writeln!(f, "{}", error.reason)?;
		}

		Ok(())
	}
}

#[derive(Deserialize)]
struct ApiError {
	field: Option<String>,
	reason: String,
}

/// Generates a random string for a secret.
pub(crate) fn generate_password(length: usize) -> String {
	rand::thread_rng()
		.sample_iter(&Alphanumeric)
		.take(length)
		.map(char::from)
		.collect()
}
