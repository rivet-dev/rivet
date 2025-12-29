use std::{
	fs,
	path::{Path, PathBuf},
	process::Command,
};

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
		.join("runner-protocol");

	// Rust SDK generation
	let cfg = vbare_compiler::Config::with_hashable_map();
	vbare_compiler::process_schemas_with_config(&schema_dir, &cfg)?;

	// TypeScript SDK generation
	let cli_js_path = workspace_root
		.parent()
		.unwrap()
		.join("node_modules/@bare-ts/tools/dist/bin/cli.js");
	if cli_js_path.exists() {
		typescript::generate_sdk(&schema_dir);
	} else {
		println!(
			"cargo:warning=TypeScript SDK generation skipped: cli.js not found at {}. Run `pnpm install` to install.",
			cli_js_path.display()
		);
	}

	Ok(())
}

mod typescript {
	use super::*;

	pub fn generate_sdk(schema_dir: &Path) {
		let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
		let workspace_root = Path::new(&manifest_dir)
			.parent()
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.expect("Failed to find workspace root");

		let sdk_dir = workspace_root
			.join("sdks")
			.join("typescript")
			.join("runner-protocol");
		let src_dir = sdk_dir.join("src");

		let highest_version_path = super::find_highest_version(schema_dir);

		let _ = fs::remove_dir_all(&src_dir);
		if let Err(e) = fs::create_dir_all(&src_dir) {
			panic!("Failed to create SDK directory: {}", e);
		}

		let output_path = src_dir.join("index.ts");

		let output = Command::new(
			workspace_root
				.parent()
				.unwrap()
				.join("node_modules/@bare-ts/tools/dist/bin/cli.js"),
		)
		.arg("compile")
		.arg("--generator")
		.arg("ts")
		.arg(highest_version_path)
		.arg("-o")
		.arg(&output_path)
		.output()
		.expect("Failed to execute bare compiler for TypeScript");

		if !output.status.success() {
			panic!(
				"BARE TypeScript generation failed: {}",
				String::from_utf8_lossy(&output.stderr),
			);
		}

		// Post-process the generated TypeScript file
		// IMPORTANT: Keep this in sync with rivetkit-typescript/packages/rivetkit/scripts/compile-bare.ts
		post_process_generated_ts(&output_path);
	}

	/// Post-process the generated TypeScript file to:
	/// 1. Replace @bare-ts/lib import with @rivetkit/bare-ts
	/// 2. Replace Node.js assert import with a custom assert function
	///
	/// IMPORTANT: Keep this in sync with rivetkit-typescript/packages/rivetkit/scripts/compile-bare.ts
	fn post_process_generated_ts(path: &Path) {
		let content = fs::read_to_string(path).expect("Failed to read generated TypeScript file");

		// Replace @bare-ts/lib with @rivetkit/bare-ts
		let content = content.replace("@bare-ts/lib", "@rivetkit/bare-ts");

		// Replace Node.js assert import with custom assert function
		let content = content.replace("import assert from \"assert\"", "");
		let content = content.replace("import assert from \"node:assert\"", "");

		// Append custom assert function
		let assert_function = r#"
function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}
"#;
		let content = format!("{}\n{}", content, assert_function);

		// Validate post-processing succeeded
		assert!(
			!content.contains("@bare-ts/lib"),
			"Failed to replace @bare-ts/lib import"
		);
		assert!(
			!content.contains("import assert from"),
			"Failed to remove Node.js assert import"
		);
		assert!(
			content.contains("function assert(condition: boolean"),
			"Assert function not found in output"
		);

		fs::write(path, content).expect("Failed to write post-processed TypeScript file");
	}
}

fn find_highest_version(schema_dir: &Path) -> PathBuf {
	let mut highest_version = 0;
	let mut highest_version_path = PathBuf::new();

	for entry in fs::read_dir(schema_dir).unwrap().flatten() {
		if !entry.path().is_dir() {
			let path = entry.path();
			let bare_name = path
				.file_name()
				.unwrap()
				.to_str()
				.unwrap()
				.split_once('.')
				.unwrap()
				.0;

			if let Ok(version) = bare_name[1..].parse::<u32>() {
				if version > highest_version {
					highest_version = version;
					highest_version_path = path;
				}
			}
		}
	}

	highest_version_path
}
