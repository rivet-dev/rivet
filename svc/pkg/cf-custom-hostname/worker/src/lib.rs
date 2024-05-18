use serde::Deserialize;

pub mod workers;

#[derive(Debug, Deserialize)]
struct CloudflareError {
	errors: Vec<CloudflareErrorEntry>,
}

#[derive(Debug, Deserialize)]
struct CloudflareErrorEntry {
	code: usize,
	message: String,
}
