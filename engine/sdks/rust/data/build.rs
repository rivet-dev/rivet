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

	fs::write(
		out_dir.join("converters.rs"),
		converters::generate(&schema_dir)?,
	)?;

	Ok(())
}

/// Generates the version family table from `families.list`. Every top-level schema must have
/// exactly one stable ID, and every ID must name a schema, so adding a schema without an ID
/// fails the build. A protocol whose schema lives outside the data directory, such as the
/// envoy protocol, is listed with the oldest and newest version this binary reads instead.
fn generate_families(
	list: &str,
	versions: &[(String, u32)],
	min_versions: &HashMap<String, u32>,
) -> Result<String, Box<dyn std::error::Error>> {
	let mut ids = std::collections::BTreeMap::<u16, String>::new();
	let mut by_schema = std::collections::BTreeMap::<String, u16>::new();
	// Protocol families whose schemas live outside the data directory, with the versions this
	// binary reads and writes stated in the list.
	let mut external = std::collections::BTreeMap::<String, (u16, u16)>::new();
	for line in list.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let mut fields = line.split_whitespace();
		let (Some(id), Some(schema)) = (fields.next(), fields.next()) else {
			return Err(format!("malformed families.list line: {line}").into());
		};
		let id: u16 = id.parse()?;
		let schema = schema.to_string();
		match (fields.next(), fields.next(), fields.next()) {
			(None, None, None) => {}
			(Some(min_read), Some(max_read), None) => {
				let range = (min_read.parse::<u16>()?, max_read.parse::<u16>()?);
				if range.0 == 0 || range.0 > range.1 {
					return Err(format!("invalid version range for {schema}: {line}").into());
				}
				external.insert(schema.clone(), range);
			}
			_ => return Err(format!("malformed families.list line: {line}").into()),
		}
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
		let has_schema = schemas.contains(schema.as_str());
		let is_external = external.contains_key(schema);
		if has_schema && is_external {
			return Err(format!(
				"families.list gives {schema} a version range, but its schema is in the data directory"
			)
			.into());
		}
		if !has_schema && !is_external {
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
		if let Some((min_read, max_read)) = external.get(schema) {
			out.push_str(&format!(
				"\tFamilyInfo {{ family: VersionFamily({id}), schema: {schema:?}, min_read: {min_read}, max_read: {max_read}, max_write: {max_read}, external: true }},\n"
			));
			continue;
		}
		let version_const = schema_identifier_to_const(schema);
		let min_read = min_versions.get(schema).copied().unwrap_or(1);
		out.push_str(&format!(
			"\tFamilyInfo {{ family: VersionFamily({id}), schema: {schema:?}, min_read: {min_read}, max_read: crate::generated::{version_const}, max_write: crate::generated::{version_const}, external: false }},\n"
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

/// Generates `From` impls between consecutive versions of every schema for the types whose
/// definitions are identical in both versions.
///
/// A new schema version is a new file, so every type it defines is a new Rust type, including
/// the types that did not change. A version converter must map each field by hand, and most of
/// that mapping would be field-for-field moves between identical types. This module writes
/// those moves: for every pair of consecutive versions of a schema, a type defined in both
/// whose definition is the same and whose referenced types are either the same Rust type (an
/// import at one version) or themselves identical gets `From` in both directions. Aliases
/// (`type X data`, `type X void`) are the same Rust type at every version and get nothing.
/// The hand-written converters in `src/versioned` cover the types that changed and call into
/// these for the parts that did not.
mod converters {
	use std::{
		collections::{BTreeMap, BTreeSet},
		fmt::Write as _,
		fs,
		path::{Path, PathBuf},
	};
	use vbare_gen::{
		ResolvedSchema, SchemaLoader,
		ast::{AnyType, PrimitiveType},
		module_name, naming,
	};

	type Error = Box<dyn std::error::Error>;

	pub(crate) fn generate(schema_dir: &Path) -> Result<String, Error> {
		let mut versions: BTreeMap<String, BTreeMap<u32, PathBuf>> = BTreeMap::new();
		for subdir in ["", "shared"] {
			let dir = schema_dir.join(subdir);
			for entry in fs::read_dir(&dir)?.flatten() {
				let path = entry.path();
				if path.is_dir() {
					continue;
				}
				let file_name = path
					.file_name()
					.and_then(|name| name.to_str())
					.unwrap_or_default();
				let Some(stem) = file_name.strip_suffix(".bare") else {
					continue;
				};
				let Some(dot_v) = stem.rfind(".v") else {
					continue;
				};
				let Ok(version) = stem[dot_v + 2..].parse::<u32>() else {
					continue;
				};
				let identifier = Path::new(subdir).join(&stem[..dot_v]);
				versions
					.entry(identifier.to_string_lossy().into_owned())
					.or_default()
					.insert(version, Path::new(subdir).join(file_name));
			}
		}

		let mut out = String::new();
		let mut loader = SchemaLoader::new(schema_dir);
		for files in versions.values() {
			let files: Vec<&PathBuf> = files.values().collect();
			for pair in files.windows(2) {
				emit_pair(&mut loader, pair[0], pair[1], &mut out)?;
			}
		}
		Ok(out)
	}

	/// The two versions of one schema with the types that are identical between them.
	struct Pair {
		older: ResolvedSchema,
		newer: ResolvedSchema,
		identical: BTreeSet<String>,
	}

	impl Pair {
		/// Whether `name` is the same Rust type in both versions: it comes from one import that
		/// both versions share.
		fn same(&self, name: &str) -> bool {
			match (self.older.origins.get(name), self.newer.origins.get(name)) {
				(Some(a), Some(b)) => a == b,
				_ => false,
			}
		}

		/// Whether `name` is a type alias, which the generator renders as the aliased Rust type.
		fn alias(&self, name: &str) -> bool {
			!matches!(
				self.older.registry.get(name),
				Some(AnyType::Struct(_) | AnyType::Enum(_) | AnyType::Union(_))
			)
		}

		/// Whether a value of `definition` in the older version converts to the newer one with
		/// a move alone.
		fn moves(&self, definition: &AnyType) -> bool {
			match definition {
				AnyType::Primitive(_) => true,
				AnyType::TypeReference(name) => {
					self.same(name)
						|| (self.alias(name)
							&& self
								.older
								.registry
								.get(name)
								.is_some_and(|inner| self.moves(inner)))
				}
				AnyType::List { inner, .. } | AnyType::Optional(inner) => self.moves(inner),
				AnyType::Map { key, value } => self.moves(key) && self.moves(value),
				AnyType::Struct(_) | AnyType::Enum(_) | AnyType::Union(_) => false,
			}
		}

		/// The expression converting `value`, of `definition` in the older version, to the newer
		/// version.
		fn convert(&self, value: &str, definition: &AnyType) -> String {
			if self.moves(definition) {
				return value.to_owned();
			}
			match definition {
				AnyType::TypeReference(name) if self.alias(name) => {
					let inner = &self.older.registry[name];
					self.convert(value, inner)
				}
				AnyType::TypeReference(_) => format!("{value}.into()"),
				AnyType::List {
					inner,
					length: None,
				} => format!(
					"{value}.into_iter().map(|item| {}).collect()",
					self.convert("item", inner)
				),
				AnyType::List {
					inner,
					length: Some(_),
				} => format!("{value}.map(|item| {})", self.convert("item", inner)),
				AnyType::Optional(inner) => {
					format!("{value}.map(|item| {})", self.convert("item", inner))
				}
				AnyType::Map { key, value: item } => format!(
					"{value}.into_iter().map(|(key, item)| ({}, {})).collect()",
					self.convert("key", key),
					self.convert("item", item)
				),
				AnyType::Primitive(_)
				| AnyType::Struct(_)
				| AnyType::Enum(_)
				| AnyType::Union(_) => {
					unreachable!("identical types never hold anonymous aggregates")
				}
			}
		}

		/// Whether `name` resolves, through aliases, to `void`.
		fn is_void(&self, name: &str) -> bool {
			match self.older.registry.get(name) {
				Some(AnyType::Primitive(PrimitiveType::Void)) => true,
				Some(AnyType::TypeReference(inner)) => self.is_void(inner),
				_ => false,
			}
		}
	}

	/// Whether two definitions have the same shape, comparing referenced types by name.
	fn same_shape(a: &AnyType, b: &AnyType) -> bool {
		match (a, b) {
			(AnyType::Primitive(a), AnyType::Primitive(b)) => format!("{a:?}") == format!("{b:?}"),
			(
				AnyType::List {
					inner: a,
					length: la,
				},
				AnyType::List {
					inner: b,
					length: lb,
				},
			) => la == lb && same_shape(a, b),
			(AnyType::Struct(a), AnyType::Struct(b)) => {
				a.len() == b.len()
					&& a.iter()
						.zip(b)
						.all(|(a, b)| a.name == b.name && same_shape(&a.type_r, &b.type_r))
			}
			(AnyType::Enum(a), AnyType::Enum(b)) => a == b,
			(AnyType::Map { key: ka, value: va }, AnyType::Map { key: kb, value: vb }) => {
				same_shape(ka, kb) && same_shape(va, vb)
			}
			(AnyType::Union(a), AnyType::Union(b)) => {
				a.len() == b.len()
					&& a.iter()
						.zip(b)
						.all(|((a, ta), (b, tb))| ta == tb && same_shape(a, b))
			}
			(AnyType::Optional(a), AnyType::Optional(b)) => same_shape(a, b),
			(AnyType::TypeReference(a), AnyType::TypeReference(b)) => a == b,
			_ => false,
		}
	}

	/// Every type a definition references, by name.
	fn references(definition: &AnyType, out: &mut Vec<String>) {
		match definition {
			AnyType::Primitive(_) | AnyType::Enum(_) => {}
			AnyType::TypeReference(name) => out.push(name.clone()),
			AnyType::List { inner, .. } | AnyType::Optional(inner) => references(inner, out),
			AnyType::Struct(fields) => {
				for field in fields {
					references(&field.type_r, out);
				}
			}
			AnyType::Map { key, value } => {
				references(key, out);
				references(value, out);
			}
			AnyType::Union(members) => {
				for (member, _) in members {
					references(member, out);
				}
			}
		}
	}

	/// Whether a union member can be converted: a named type or a primitive.
	fn supported_member(member: &AnyType) -> bool {
		matches!(member, AnyType::TypeReference(_) | AnyType::Primitive(_))
	}

	fn emit_pair(
		loader: &mut SchemaLoader,
		older_path: &Path,
		newer_path: &Path,
		out: &mut String,
	) -> Result<(), Error> {
		let mut pair = Pair {
			older: loader.resolve_schema(older_path)?,
			newer: loader.resolve_schema(newer_path)?,
			identical: BTreeSet::new(),
		};
		// Start from every type defined in both versions, in different files, with the same
		// shape, then drop the ones that reference a type that changed until nothing drops.
		for (name, older) in &pair.older.registry {
			let Some(newer) = pair.newer.registry.get(name) else {
				continue;
			};
			if pair.same(name) || !same_shape(older, newer) {
				continue;
			}
			if let AnyType::Union(members) = older
				&& !members.iter().all(|(member, _)| supported_member(member))
			{
				continue;
			}
			pair.identical.insert(name.clone());
		}
		loop {
			let before = pair.identical.len();
			let keep: BTreeSet<String> = pair
				.identical
				.iter()
				.filter(|name| {
					let mut referenced = Vec::new();
					references(&pair.older.registry[*name], &mut referenced);
					referenced
						.iter()
						.all(|reference| pair.same(reference) || pair.identical.contains(reference))
				})
				.cloned()
				.collect();
			pair.identical = keep;
			if pair.identical.len() == before {
				break;
			}
		}

		let older_module = module_name(older_path)?;
		let newer_module = module_name(newer_path)?;
		for name in &pair.identical {
			if pair.alias(name)
				|| !pair.older.local.contains_key(name)
				|| !pair.newer.local.contains_key(name)
			{
				continue;
			}
			let definition = &pair.older.registry[name];
			emit_from(&pair, &older_module, &newer_module, name, definition, out)?;
			emit_from(&pair, &newer_module, &older_module, name, definition, out)?;
		}
		Ok(())
	}

	/// Writes `impl From<from::name> for to::name`. The definition is identical in both
	/// versions, so one shape serves both directions.
	fn emit_from(
		pair: &Pair,
		from: &str,
		to: &str,
		name: &str,
		definition: &AnyType,
		out: &mut String,
	) -> Result<(), Error> {
		let from_type = format!("crate::generated::{from}::{name}");
		let to_type = format!("crate::generated::{to}::{name}");
		writeln!(
			out,
			"impl ::std::convert::From<{from_type}> for {to_type} {{\n\tfn from(value: {from_type}) -> Self {{"
		)?;
		match definition {
			AnyType::Struct(fields) => {
				writeln!(out, "\t\tSelf {{")?;
				for field in fields {
					let field_name = naming::rust_field_name(&field.name)?;
					writeln!(
						out,
						"\t\t\t{field_name}: {},",
						pair.convert(&format!("value.{field_name}"), &field.type_r)
					)?;
				}
				writeln!(out, "\t\t}}")?;
			}
			AnyType::Enum(values) => {
				writeln!(out, "\t\tmatch value {{")?;
				for (value, _) in values {
					let variant = naming::rust_enum_variant_name(value)?;
					writeln!(out, "\t\t\t{from_type}::{variant} => Self::{variant},")?;
				}
				writeln!(out, "\t\t}}")?;
			}
			AnyType::Union(members) => {
				writeln!(out, "\t\tmatch value {{")?;
				for (index, (member, _)) in members.iter().enumerate() {
					let variant = naming::rust_union_variant_name(member, index, false);
					let void = match member {
						AnyType::TypeReference(name) => pair.is_void(name),
						AnyType::Primitive(PrimitiveType::Void) => true,
						_ => false,
					};
					if void {
						writeln!(out, "\t\t\t{from_type}::{variant} => Self::{variant},")?;
					} else {
						writeln!(
							out,
							"\t\t\t{from_type}::{variant}(inner) => Self::{variant}({}),",
							pair.convert("inner", member)
						)?;
					}
				}
				writeln!(out, "\t\t}}")?;
			}
			AnyType::Primitive(_)
			| AnyType::List { .. }
			| AnyType::Map { .. }
			| AnyType::Optional(_)
			| AnyType::TypeReference(_) => unreachable!("aliases get no conversion"),
		}
		writeln!(out, "\t}}\n}}")?;
		Ok(())
	}
}
