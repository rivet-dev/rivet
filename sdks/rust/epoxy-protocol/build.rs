use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
	let workspace_root = Path::new(&manifest_dir)
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.ok_or("Failed to find workspace root")?;

	let schema_dir = workspace_root
		.join("sdks")
		.join("schemas")
		.join("epoxy-protocol");

	let cfg = vbare_compiler::Config::with_hashable_map();
	vbare_compiler::process_schemas_with_config(&schema_dir, &cfg)
}
