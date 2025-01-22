use anyhow::*;
use fs_extra::dir::{copy, CopyOptions};
use merkle_hash::MerkleTree;
use std::fs;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
	let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
	let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

	let mut js_utils_path = PathBuf::from(manifest_dir.clone());
	js_utils_path.push("js");

	// Copy js-utils directory to out_dir
	let out_js_utils_path = out_dir.join("js-utils");

	// Remove old dir
	if out_js_utils_path.is_dir() {
		fs::remove_dir_all(&out_js_utils_path).context("fs::remove_dir_all")?;
	}

	// Create the target directory first
	let copy_options = CopyOptions::new().overwrite(true).copy_inside(true);

	copy(&js_utils_path, &out_js_utils_path, &copy_options).with_context(|| {
		format!(
			"failed to copy directory from {} to {}",
			js_utils_path.display(),
			out_js_utils_path.display()
		)
	})?;

	println!("cargo:rerun-if-changed={}", js_utils_path.display());
	println!(
		"cargo:rustc-env=JS_UTILS_PATH={}",
		out_js_utils_path.display()
	);
	println!(
		"cargo:rustc-env=JS_UTILS_HASH={}",
		hash_directory(&out_js_utils_path)?
	);

	Ok(())
}

fn hash_directory<P: AsRef<Path>>(path: P) -> Result<String> {
	let tree = MerkleTree::builder(&path.as_ref().display().to_string()).build()?;
	let hash = tree
		.root
		.item
		.hash
		.iter()
		.map(|b| format!("{:02x}", b))
		.collect::<Vec<String>>()
		.join("");
	Ok(hash)
}
