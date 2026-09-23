use std::{fs, path::Path};

fn main() {
	let mut schema = schemars::schema_for!(rivet_config::config::Root);
	// Config loading keeps root fields optional so environment overrides can merge, but a
	// running Engine requires authentication. Reflect that invariant in the published schema.
	let object = schema.schema.object.as_mut().expect("root object schema");
	object.required.insert("auth".to_owned());
	object.properties.insert(
		"auth".to_owned(),
		schemars::schema::SchemaObject {
			reference: Some("#/definitions/Auth".to_owned()),
			..Default::default()
		}
		.into(),
	);

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
