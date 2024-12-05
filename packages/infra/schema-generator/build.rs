use std::{fs::File, io::Write};

use anyhow::*;
use schemars::gen::SchemaSettings;
use rivet_config;

fn main() -> Result<()> {
	let cwd = std::env::current_dir()?;
	let output_path = cwd.join("../../../docs/src/content/docs/self-hosting/");
	
	let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let generator = settings.into_generator();
    
	// Server config
	let schema = generator.clone().into_root_schema_for::<rivet_config::config::server::Server>();
	let schema_text = serde_json::to_string_pretty(&schema)?;

	let mut file = File::create(output_path.join("server-spec.json"))?;
	file.write_all(schema_text.as_bytes())?;

	// Client config
	let schema = generator.into_root_schema_for::<pegboard_config::Client>();
	let schema_text = serde_json::to_string_pretty(&schema)?;

	let mut file = File::create(output_path.join("client-spec.json"))?;
	file.write_all(schema_text.as_bytes())?;

	Ok(())
}
