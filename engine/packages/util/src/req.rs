use std::time::Duration;

use anyhow::*;
use async_trait::async_trait;

#[async_trait]
pub trait SendRetry {
	/// Retries the request upon receiving a 429 response.
	async fn send_retry(self, mut retries: usize) -> Result<reqwest::Response>;
}

#[async_trait]
impl SendRetry for reqwest::RequestBuilder {
	async fn send_retry(self, mut retries: usize) -> Result<reqwest::Response> {
		loop {
			let req = self.try_clone().context("failed cloning req")?;
			let res = req.send().await?;

			if let reqwest::StatusCode::TOO_MANY_REQUESTS = res.status() {
				if retries != 0 {
					retries -= 1;

					// TODO: Parse all valid Retry-After formats. Currently only parses duration
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

			break Ok(res);
		}
	}
}
