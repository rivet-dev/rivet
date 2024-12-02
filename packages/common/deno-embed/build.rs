use dirs::cache_dir;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{env, fs, path::Path};
use zip::ZipArchive;

const GITHUB_API_URL: &str = "https://api.github.com/repos/denoland/deno";
const DENO_VERSION: &str = "2.0.6";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn main() -> Result<(), Box<dyn std::error::Error>> {
	for (key, value) in std::env::vars() {
		println!("{}: {}", key, value);
	}

	let target = if cfg!(feature = "ignore-override-target") {
		env::var("TARGET").unwrap()
	} else {
		env::var("OVERRIDE_TARGET").unwrap_or_else(|_| env::var("TARGET").unwrap())
	};
	println!("cargo::rerun-if-env-changed=OVERRIDE_TARGET");

	let out_dir = env::var("OUT_DIR")?;
	let cache_dir = get_cache_dir()?;

	let release_data = fetch_release_data()?;
	let asset = find_matching_asset(&release_data, &target)?;
	let zip_path = download_binary_if_needed(&asset, &cache_dir)?;
	let output_path = extract_and_save_binary(&zip_path, &out_dir)?;

	println!("cargo:rustc-env=DENO_BINARY_PATH={}", output_path.display());
	println!("cargo:rustc-env=DENO_VERSION={DENO_VERSION}");

	Ok(())
}

fn fetch_release_data() -> Result<Value, Box<dyn std::error::Error>> {
	let release_url = format!("{}/releases/tags/v{}", GITHUB_API_URL, DENO_VERSION);
	println!("Fetching release information from: {}", release_url);

	let client = Client::new();
	let mut request = client
		.get(&release_url)
		.header(reqwest::header::USER_AGENT, USER_AGENT);

	if let Ok(token) = env::var("GITHUB_TOKEN") {
		request = request.header(reqwest::header::AUTHORIZATION, format!("token {}", token));
	}

	let response = request.send()?;
	let status = response.status();
	if !status.is_success() {
		let error_text = response.text()?;
		eprintln!("Error response: {}", error_text);
		return Err(format!("HTTP request failed with status {}: {}", status, error_text).into());
	}
	Ok(response.json()?)
}

fn find_matching_asset<'a>(
	release_data: &'a Value,
	target: &str,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
	let assets = release_data["assets"].as_array().ok_or("No assets found")?;
	let deno_target = map_rust_target_to_deno(target);
	assets
		.iter()
		.find(|asset| {
			let name = asset["name"].as_str().unwrap();
			name == deno_target
		})
		.ok_or_else(|| format!("No matching asset found for the target: {}", deno_target).into())
}

fn map_rust_target_to_deno(target: &str) -> &'static str {
	match target {
		x if x.starts_with("x86_64-unknown-linux-") => "deno-x86_64-unknown-linux-gnu.zip",
		x if x.starts_with("aarch64-unknown-linux-") => "deno-aarch64-unknown-linux-gnu.zip",
		x if x.starts_with("x86_64-pc-windows-") => "deno-x86_64-pc-windows-msvc.zip",
		"x86_64-apple-darwin" => "deno-x86_64-apple-darwin.zip",
		"aarch64-apple-darwin" => "deno-aarch64-apple-darwin.zip",
		_ => panic!(
			"Unsupported target: {}. Set OVERRIDE_TARGET if needed.",
			target
		),
	}
}

fn download_binary_if_needed(
	asset: &Value,
	cache_dir: &Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
	let download_url = asset["browser_download_url"].as_str().unwrap();
	let file_name = asset["name"].as_str().unwrap();
	let zip_path = cache_dir.join(file_name);

	if !zip_path.exists() {
		println!("Downloading Deno binary from: {}", download_url);

		let client = Client::new();
		let mut request = client
			.get(download_url)
			.header(reqwest::header::USER_AGENT, USER_AGENT);

		if let Ok(token) = env::var("GITHUB_TOKEN") {
			request = request.header(reqwest::header::AUTHORIZATION, format!("token {}", token));
		}

		let response = request.send()?.error_for_status()?;

		let mut file = fs::File::create(&zip_path)?;
		std::io::copy(&mut response.bytes()?.as_ref(), &mut file)?;
	} else {
		println!("Using cached Deno binary: {}", zip_path.display());
	}

	Ok(zip_path)
}

fn extract_and_save_binary(
	zip_path: &Path,
	out_dir: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
	let file = fs::File::open(zip_path)?;
	let mut archive = ZipArchive::new(file)?;
	let mut file = archive.by_index(0)?;
	let output_path = Path::new(out_dir).join("deno");

	let mut output_file = fs::File::create(&output_path)?;
	std::io::copy(&mut file, &mut output_file)?;

	Ok(output_path)
}

fn get_cache_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
	let system_cache_dir = cache_dir().ok_or("Failed to get system cache directory")?;
	let deno_cache_dir = system_cache_dir.join("deno-embed").join(DENO_VERSION);
	fs::create_dir_all(&deno_cache_dir)?;
	Ok(deno_cache_dir)
}
