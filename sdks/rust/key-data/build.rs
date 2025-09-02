use std::{env, fs, path::Path};

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
				.0
				.replace(".", "_");

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

fn main() {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
	let workspace_root = Path::new(&manifest_dir)
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("Failed to find workspace root");

	let schema_dir = workspace_root.join("sdks").join("schemas").join("key-data");

	println!("cargo:rerun-if-changed={}", schema_dir.display());

	rust::generate_sdk(&schema_dir);
}
