#![allow(
	clippy::disallowed_methods,
	clippy::disallowed_types,
	reason = "build-time code generation reads and writes host files outside the runtime environment"
)]

use std::{
	fs,
	path::{Path, PathBuf},
	process::Command,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
	let schema_dir = manifest_dir.join("schemas");
	let repo_root = manifest_dir
		.parent()
		.and_then(|path| path.parent())
		.and_then(|path| path.parent())
		.and_then(|path| path.parent())
		.ok_or("failed to find repository root")?;

	println!("cargo:rerun-if-changed={}", schema_dir.display());
	vbare_compiler::process_schemas_with_config(&schema_dir, &Default::default())?;
	typescript::generate_versions(repo_root, &schema_dir, "actor-runtime-socket-protocol");
	Ok(())
}

mod typescript {
	use super::*;

	pub fn generate_versions(repo_root: &Path, schema_dir: &Path, protocol_name: &str) {
		let cli_js_path = repo_root.join("node_modules/@bare-ts/tools/dist/bin/cli.js");
		if !cli_js_path.exists() {
			println!(
				"cargo:warning=TypeScript codec generation skipped: cli.js not found at {}. Run `pnpm install` to install.",
				cli_js_path.display()
			);
			return;
		}

		let output_dir = repo_root
			.join("rivetkit-typescript/packages/rivetkit/src/common/bare/generated")
			.join(protocol_name);
		let _ = fs::remove_dir_all(&output_dir);
		fs::create_dir_all(&output_dir)
			.expect("failed to create generated TypeScript codec directory");

		for schema_path in schema_paths(schema_dir) {
			let version = schema_path
				.file_stem()
				.and_then(|stem| stem.to_str())
				.expect("schema has valid UTF-8 file stem");
			let output_path = output_dir.join(format!("{version}.ts"));
			let output = Command::new(&cli_js_path)
				.arg("compile")
				.arg("--generator")
				.arg("ts")
				.arg(&schema_path)
				.arg("-o")
				.arg(&output_path)
				.output()
				.expect("failed to execute BARE TypeScript compiler");
			if !output.status.success() {
				panic!(
					"BARE TypeScript generation failed for {}: {}",
					schema_path.display(),
					String::from_utf8_lossy(&output.stderr),
				);
			}
			post_process_generated_ts(&output_path);
		}
	}

	fn schema_paths(schema_dir: &Path) -> Vec<PathBuf> {
		let mut paths = fs::read_dir(schema_dir)
			.expect("failed to read schema directory")
			.flatten()
			.map(|entry| entry.path())
			.filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("bare"))
			.collect::<Vec<_>>();
		paths.sort();
		paths
	}

	fn post_process_generated_ts(path: &Path) {
		const MARKER: &str = "// @generated - post-processed by build.rs\n";
		let content = fs::read_to_string(path).expect("failed to read generated TypeScript file");
		let content = content
			.replace("@bare-ts/lib", "@rivetkit/bare-ts")
			.replace("import assert from \"assert\"", "")
			.replace("import assert from \"node:assert\"", "");
		let assert_function = r#"
function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
"#;
		let content = format!("{MARKER}{content}\n{assert_function}");
		assert!(!content.contains("@bare-ts/lib"));
		assert!(!content.contains("import assert from"));
		fs::write(path, content).expect("failed to write generated TypeScript file");
	}
}
