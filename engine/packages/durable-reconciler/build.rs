fn main() -> Result<(), Box<dyn std::error::Error>> {
	let schemas =
		std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("examples/schemas");
	let config = vbare_compiler::Config {
		vbare: vbare_gen::Config::default().with_serde(true),
	};
	vbare_compiler::process_schemas_with_config(&schemas, &config)
}
