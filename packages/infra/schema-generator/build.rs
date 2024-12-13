use std::{fs::File, io::Write, path::PathBuf};

use anyhow::*;
use schemars::{
	gen::{SchemaGenerator, SchemaSettings},
	JsonSchema,
};

fn main() -> Result<()> {
	let cwd = std::env::current_dir()?;
	let docs_output_path = cwd.join("../../../docs/src/content/docs/");

	if !std::fs::metadata(&docs_output_path).is_ok() {
		eprintln!(
			"output path does not exist: {:?}",
			docs_output_path.display()
		);
		return Ok(());
	}

	let settings = SchemaSettings::draft07().with(|s| {
		s.option_add_null_type = false;
	});
	let generator = settings.into_generator();

	generate_spec::<rivet_config::config::server::Server>(
		generator.clone(),
		docs_output_path.join("self-hosting/server-spec.json"),
	)?;
	generate_spec::<pegboard_config::Client>(
		generator.clone(),
		docs_output_path.join("self-hosting/client-spec.json"),
	)?;
	generate_spec::<rivet_toolchain::config::Root>(
		generator.clone(),
		docs_output_path.join("toolchain-spec.json"),
	)?;

	Ok(())
}

fn generate_spec<T: JsonSchema>(generator: SchemaGenerator, path: PathBuf) -> Result<()> {
	let schema = generator.into_root_schema_for::<T>();
	let schema_text = serde_json::to_string_pretty(&schema)?;

	let mut file = File::create(path)?;
	file.write_all(schema_text.as_bytes())?;

	Ok(())
}
