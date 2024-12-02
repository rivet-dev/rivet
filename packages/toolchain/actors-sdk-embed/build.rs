use anyhow::*;
use fs_extra::dir::{copy, CopyOptions};
use merkle_hash::MerkleTree;
use std::{
	fs,
	path::{Path, PathBuf},
};

#[tokio::main]
async fn main() -> Result<()> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = std::env::var("OUT_DIR")?;

	let sdk_path = PathBuf::from(manifest_dir.clone()).join("../../../sdks/actors");

	// Copy SDK directory to out_dir
	let out_sdk_path = Path::new(&out_dir).join("actors-sdk");

	// Remove old dir
	if out_sdk_path.is_dir() {
		fs::remove_dir_all(&out_sdk_path).context("fs::remove_dir_all")?;
	}

	// Copy sdk directory to out_dir
	let mut copy_options = CopyOptions::new();
	copy_options.overwrite = true;
	copy_options.copy_inside = true;
	copy(&sdk_path, &out_sdk_path, &copy_options).with_context(|| {
		format!(
			"failed to copy directory from {} to {}",
			sdk_path.display(),
			out_sdk_path.display()
		)
	})?;

	println!("cargo:rerun-if-changed={}", sdk_path.display());
	println!("cargo:rustc-env=ACTORS_SDK_PATH={}", out_sdk_path.display());
	println!(
		"cargo:rustc-env=ACTORS_SDK_HASH={}",
		hash_directory(&out_sdk_path)?
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
