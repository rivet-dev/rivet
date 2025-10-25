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
	let cli_js_path = workspace_root.join("node_modules/@vbare/compiler/dist/cli.js");
	let vbare_compiler_dir = workspace_root.join("node_modules/@vbare/compiler");

	// Rerun build script if @vbare/compiler changes
	if vbare_compiler_dir.exists() {
		println!("cargo:rerun-if-changed={}", vbare_compiler_dir.display());
	}

	if cli_js_path.exists() {
		typescript::generate_sdk(&schema_dir, &cli_js_path);
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

	pub fn generate_sdk(schema_dir: &Path, cli_js_path: &Path) {
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

		let output = Command::new(cli_js_path)
			.arg("--generator")
			.arg("ts")
			.arg("-o")
			.arg(src_dir.join("index.ts"))
			.arg(highest_version_path)
			.output()
			.expect("Failed to execute bare compiler for TypeScript");

		if !output.status.success() {
			panic!(
				"BARE TypeScript generation failed: {}",
				String::from_utf8_lossy(&output.stderr),
			);
		}
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
