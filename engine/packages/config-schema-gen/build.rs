use std::{fs, path::Path};

fn main() {
	let schema = schemars::schema_for!(rivet_config::config::Root);

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
	let out_dir = workspace_root.join("engine").join("artifacts");
	fs::create_dir_all(&out_dir).unwrap();

	// Write pretty-formatted JSON to out/config-schema.json
	let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize JSON Schema");
	fs::write(out_dir.join("config-schema.json"), json)
		.expect("Failed to write config-schema.json");
}
