use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
	let workspace_root = Path::new(&manifest_dir)
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.ok_or("Failed to find workspace root")?;

	let schema_dir = workspace_root.join("sdks").join("schemas").join("data");

	let cfg = vbare_compiler::Config::with_hashable_map();
	vbare_compiler::process_schemas_with_config(&schema_dir, &cfg)?;

	// Append per-schema version constants to generated file
	let versions = find_schema_versions(&schema_dir);
	let combined_imports_path = out_dir.join("combined_imports.rs");
	let mut combined = fs::read_to_string(&combined_imports_path)?;
	for (identifier, version) in &versions {
		let const_name = schema_identifier_to_const(identifier);
		combined.push_str(&format!("\npub const {}: u16 = {};\n", const_name, version));
	}
	fs::write(combined_imports_path, combined)?;

	Ok(())
}

/// Parses schema files named `{identifier}.v{N}.bare`, groups by identifier, and returns
/// each identifier with its highest version number, sorted alphabetically.
fn find_schema_versions(schema_dir: &Path) -> Vec<(String, u32)> {
	let mut versions: HashMap<String, u32> = HashMap::new();

	for entry in fs::read_dir(schema_dir).unwrap().flatten() {
		let path = entry.path();
		if path.is_dir() {
			continue;
		}

		let file_name = path
			.file_name()
			.and_then(|n| n.to_str())
			.unwrap_or_default();

		let Some(stem) = file_name.strip_suffix(".bare") else {
			continue;
		};

		// Find the last `.v{digits}` segment to extract version
		if let Some(dot_v_pos) = stem.rfind(".v") {
			let version_str = &stem[dot_v_pos + 2..];
			if let Ok(version) = version_str.parse::<u32>() {
				let identifier = stem[..dot_v_pos].to_string();
				let entry = versions.entry(identifier).or_insert(0);
				if version > *entry {
					*entry = version;
				}
			}
		}
	}

	let mut result: Vec<(String, u32)> = versions.into_iter().collect();
	result.sort_by(|(a, _), (b, _)| a.cmp(b));
	result
}

/// Converts a schema identifier like `pegboard.namespace.actor_by_key` to `PEGBOARD_NAMESPACE_ACTOR_BY_KEY_VERSION`.
fn schema_identifier_to_const(identifier: &str) -> String {
	format!(
		"{}_VERSION",
		identifier
			.to_uppercase()
			.replace('.', "_")
			.replace('-', "_")
	)
}
