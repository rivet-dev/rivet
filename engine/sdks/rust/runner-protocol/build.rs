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
		.arg(src_dir.join("index.ts"))
		.output()
		.expect("Failed to execute bare compiler for TypeScript");

		if !output.status.success() {
			panic!(
				"BARE TypeScript generation failed: {}",
				String::from_utf8_lossy(&output.stderr),
			);
		}

		// Post-process the generated TypeScript file to replace Node.js assert
		post_process_typescript(&src_dir.join("index.ts"));
	}

	fn post_process_typescript(file_path: &Path) {
		let content =
			fs::read_to_string(file_path).expect("Failed to read generated TypeScript file");

		// Remove Node.js assert import
		let content = content.replace("import assert from \"node:assert\"\n", "");

		// Add custom assert function at the end
		let assert_function = r#"
function assert(condition: boolean, message?: string): asserts condition {
    if (!condition) throw new Error(message ?? "Assertion failed")
}

"#;
		let content = format!("{}\n{}", content, assert_function);

		fs::write(file_path, content).expect("Failed to write post-processed TypeScript file");
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
