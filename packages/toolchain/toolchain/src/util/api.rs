use anyhow::*;
use regex::Regex;
use url::Url;

/// Normalize an API endpoint to a standardized URL.
///
/// This allows us to pass shorthand URLs like:
///
/// * api.staging2.gameinc.io -> https://api.staging2.gameinc.io
/// * 127.0.0.1:8080 -> http://127.0.0.1:8080
pub fn normalize_api_endpoint(endpoint: &str) -> Result<String> {
	// Attempt to parse URL
	let url = match Url::parse(endpoint) {
		Result::Ok(url) => url,
		Err(url::ParseError::RelativeUrlWithoutBase) => {
			// No scheme was provided, determine if to use HTTP or HTTPS based on if the URL is
			// localhost.
			let localhost_regex = Regex::new(r"^(127\.0\.0\.1|\[::1\])(:\d+)?$")?;
			let proto = if localhost_regex.is_match(endpoint) {
				"http"
			} else {
				"https"
			};

			// Attempt to parse endpoint
			match url::Url::parse(format!("{proto}://{endpoint}").as_str()) {
				Result::Ok(url) => url,
				Err(_) => {
					bail!("failed to parse endpoint: {}", endpoint)
				}
			}
		}
		Err(_) => {
			bail!("failed to parse endpoint: {}", endpoint)
		}
	};

	let url_str = url.to_string();

	// Remove trailing path
	let url_str = if let Some(x) = url_str.strip_suffix('/') {
		x.to_string()
	} else {
		url_str
	};

	Ok(url_str)
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn test_normalize_api_endpoint() {
		assert_eq!(
			"https://api.example.com",
			normalize_api_endpoint("https://api.example.com").unwrap()
		);
		assert_eq!(
			"https://api.example.com",
			normalize_api_endpoint("api.example.com").unwrap()
		);
		assert_eq!(
			"http://127.0.0.1",
			normalize_api_endpoint("127.0.0.1").unwrap()
		);
		assert_eq!(
			"http://127.0.0.1:8080",
			normalize_api_endpoint("127.0.0.1:8080").unwrap()
		);
		assert_eq!("http://[::1]", normalize_api_endpoint("[::1]").unwrap());
		assert_eq!(
			"http://[::1]:8080",
			normalize_api_endpoint("[::1]:8080").unwrap()
		);
	}
}
