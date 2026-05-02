use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
	let schema_dir = manifest_dir.join("schemas");

	let cfg = vbare_compiler::Config::with_hash_map();
	vbare_compiler::process_schemas_with_config(&schema_dir, &cfg)?;

	Ok(())
}
