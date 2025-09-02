use std::{fs, path::Path};
use utoipa::OpenApi;

fn main() {
	let openapi = rivet_api_public::router::ApiDoc::openapi();

	// Create out directory at workspace root
	let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
		.map(|dir| {
			Path::new(&dir)
				.parent()
				.unwrap()
				.parent()
				.unwrap()
				.parent()
				.unwrap()
				.to_path_buf()
		})
		.unwrap();
	let out_dir = workspace_root.join("out");
	fs::create_dir_all(&out_dir).unwrap();

	// Write pretty-formatted JSON to out/openapi.json
	let json = serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI spec");
	fs::write(out_dir.join("openapi.json"), json).expect("Failed to write openapi.json");
}
