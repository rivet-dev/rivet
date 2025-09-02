use std::{
	env, fs,
	path::{Path, PathBuf},
	process::Command,
};

use indoc::formatdoc;

mod rust {
	use super::*;

	pub fn generate_sdk(schema_dir: &Path) {
		let out_dir = env::var("OUT_DIR").unwrap();
		let out_path = Path::new(&out_dir);
		let mut all_names = Vec::new();

		for entry in fs::read_dir(schema_dir).unwrap().flatten() {
			let path = entry.path();

			if path.is_dir() {
				continue;
			}

			let bare_name = path
				.file_name()
				.unwrap()
				.to_str()
				.unwrap()
				.rsplit_once('.')
				.unwrap()
				.0;

			let content =
				prettyplease::unparse(&syn::parse2(bare_gen::bare_schema(&path)).unwrap());
			fs::write(out_path.join(format!("{bare_name}_generated.rs")), content).unwrap();

			all_names.push(bare_name.to_string());
		}

		let mut mod_content = String::new();
		mod_content.push_str("// Auto-generated module file for BARE schemas\n\n");

		// Generate module declarations for each version
		for name in all_names {
			mod_content.push_str(&formatdoc!(
				r#"
				pub mod {name} {{
					include!(concat!(env!("OUT_DIR"), "/{name}_generated.rs"));
				}}
				"#,
			));
		}

		let mod_file_path = out_path.join("combined_imports.rs");
		fs::write(&mod_file_path, mod_content).expect("Failed to write combined_imports.rs");
	}
}

mod typescript {
	use super::*;

	pub fn generate_sdk(schema_dir: &Path) {
		let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
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

		let output =
			Command::new(workspace_root.join("node_modules/@bare-ts/tools/dist/bin/cli.js"))
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

fn main() {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
	let workspace_root = Path::new(&manifest_dir)
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("Failed to find workspace root");

	let schema_dir = workspace_root
		.join("sdks")
		.join("schemas")
		.join("runner-protocol");

	println!("cargo:rerun-if-changed={}", schema_dir.display());

	rust::generate_sdk(&schema_dir);

	// Check if cli.js exists before attempting TypeScript generation
	let cli_js_path = workspace_root.join("node_modules/@bare-ts/tools/dist/bin/cli.js");
	if cli_js_path.exists() {
		typescript::generate_sdk(&schema_dir);
	} else {
		println!(
			"cargo:warning=TypeScript SDK generation skipped: cli.js not found at {}. Run `pnpm install` to install.",
			cli_js_path.display()
		);
	}
}
