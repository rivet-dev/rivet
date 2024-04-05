pub mod workers;

#[derive(thiserror::Error, Debug)]
#[error("cloudflare: {source}")]
pub struct CloudflareError {
	#[from]
	source: anyhow::Error,
}
