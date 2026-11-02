#![allow(
	clippy::disallowed_methods,
	clippy::disallowed_types,
	reason = "build-time code generation reads and writes host files outside the runtime environment"
)]

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

	let cfg = vbare_compiler::Config {
		vbare: vbare_gen::Config::default().with_serde(true),
	};
	vbare_compiler::process_schemas_with_config(&schema_dir, &cfg)?;

	// Append per-schema version constants to generated file
	let versions = find_schema_versions(&schema_dir);
	let combined_imports_path = out_dir.join("combined_imports.rs");
	let mut combined = fs::read_to_string(&combined_imports_path)?;
	for (identifier, version) in &versions {
		let const_name = schema_identifier_to_const(identifier);
		combined.push_str(&format!("\npub const {}: u16 = {};\n", const_name, version));
	}
	combined.push_str(&format!(
		"\n/// SHA-256 of the normalized `depot.log_shard` disk schema and every schema it imports.\n/// A log shard's FORMAT records it, and recovery refuses a folder written under another.\npub const DEPOT_LOG_SHARD_SCHEMA_FINGERPRINT: [u8; 32] = {:?};\n",
		schema_fingerprint(&schema_dir, "depot.log_shard.v1.bare")?
	));
	fs::write(combined_imports_path, combined)?;

	let families_path = Path::new(&manifest_dir).join("families.list");
	println!("cargo:rerun-if-changed={}", families_path.display());
	let min_versions = find_min_schema_versions(&schema_dir);
	let families = generate_families(
		&fs::read_to_string(&families_path)?,
		&versions,
		&min_versions,
	)?;
	fs::write(out_dir.join("families.rs"), families)?;

	Ok(())
}

/// Generates the version family table from `families.list`. Every top-level schema must have
/// exactly one stable ID, and every ID must name a schema, so adding a schema without an ID
/// fails the build.
fn generate_families(
	list: &str,
	versions: &[(String, u32)],
	min_versions: &HashMap<String, u32>,
) -> Result<String, Box<dyn std::error::Error>> {
	let mut ids = std::collections::BTreeMap::<u16, String>::new();
	let mut by_schema = std::collections::BTreeMap::<String, u16>::new();
	for line in list.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let (id, schema) = line
			.split_once(char::is_whitespace)
			.ok_or_else(|| format!("malformed families.list line: {line}"))?;
		let id: u16 = id.parse()?;
		let schema = schema.trim().to_string();
		if id == 0 {
			return Err(format!("family ID 0 is reserved: {schema}").into());
		}
		if ids.insert(id, schema.clone()).is_some() {
			return Err(format!("family ID {id} is listed twice").into());
		}
		if by_schema.insert(schema.clone(), id).is_some() {
			return Err(format!("schema {schema} has two family IDs").into());
		}
	}

	let schemas: std::collections::BTreeSet<&str> = versions
		.iter()
		.map(|(identifier, _)| identifier.as_str())
		.collect();
	for (identifier, _) in versions {
		if !by_schema.contains_key(identifier) {
			return Err(format!("schema {identifier} has no family ID in families.list").into());
		}
	}
	for schema in by_schema.keys() {
		if !schemas.contains(schema.as_str()) {
			return Err(format!("families.list names {schema}, which has no schema").into());
		}
	}

	let mut out = String::new();
	out.push_str("impl VersionFamily {\n");
	for (schema, id) in &by_schema {
		out.push_str(&format!(
			"\t/// The `{schema}` schema.\n\tpub const {}: Self = Self({id});\n",
			family_const_name(schema)
		));
	}
	out.push_str("}\n\n");
	out.push_str(
		"/// Every family this binary knows, ordered by ID.\npub const FAMILIES: &[FamilyInfo] = &[\n",
	);
	for (id, schema) in &ids {
		let version_const = schema_identifier_to_const(schema);
		let min_read = min_versions.get(schema).copied().unwrap_or(1);
		out.push_str(&format!(
			"\tFamilyInfo {{ family: VersionFamily({id}), schema: {schema:?}, min_read: {min_read}, max_read: crate::generated::{version_const}, max_write: crate::generated::{version_const} }},\n"
		));
	}
	out.push_str("];\n");
	Ok(out)
}

/// The oldest schema file version kept for each identifier. A binary decodes from it, since
/// every kept version has a decoder.
fn find_min_schema_versions(schema_dir: &Path) -> HashMap<String, u32> {
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
		if let Some(dot_v_pos) = stem.rfind(".v") {
			if let Ok(version) = stem[dot_v_pos + 2..].parse::<u32>() {
				let identifier = stem[..dot_v_pos].to_string();
				let entry = versions.entry(identifier).or_insert(version);
				if version < *entry {
					*entry = version;
				}
			}
		}
	}
	versions
}

/// Converts a schema identifier like `depot.log_shard` to `DEPOT_LOG_SHARD`.
fn family_const_name(identifier: &str) -> String {
	identifier
		.to_uppercase()
		.replace('.', "_")
		.replace('-', "_")
}

/// Hashes `root` and every schema it imports, transitively, in first-import order. Comments
/// and whitespace are removed first, so editing documentation keeps the fingerprint, while any
/// change to a type, field, variant or their order changes it.
fn schema_fingerprint(
	schema_dir: &Path,
	root: &str,
) -> Result<[u8; 32], Box<dyn std::error::Error>> {
	use sha2::{Digest, Sha256};

	let mut hasher = Sha256::new();
	let mut pending = vec![root.to_string()];
	let mut seen = std::collections::BTreeSet::new();
	while let Some(name) = pending.pop() {
		if !seen.insert(name.clone()) {
			continue;
		}
		let path = schema_dir.join(&name);
		println!("cargo:rerun-if-changed={}", path.display());
		let source = fs::read_to_string(&path)?;

		let mut normalized = String::new();
		let mut imports = Vec::new();
		for line in source.lines() {
			let line = line.trim();
			if let Some(import) = line.strip_prefix("#import") {
				let import = import.trim().trim_matches('"').to_string();
				normalized.push_str(&format!("import {import}\n"));
				imports.push(import);
				continue;
			}
			let code = line.split('#').next().unwrap_or_default();
			let tokens = code.split_whitespace().collect::<Vec<_>>();
			if !tokens.is_empty() {
				normalized.push_str(&tokens.join(" "));
				normalized.push('\n');
			}
		}

		hasher.update(name.as_bytes());
		hasher.update(b"\0");
		hasher.update(normalized.as_bytes());
		hasher.update(b"\0");
		// Visit imports in source order.
		pending.extend(imports.into_iter().rev());
	}
	Ok(hasher.finalize().into())
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
