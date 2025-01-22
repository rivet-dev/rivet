use anyhow::*;
use fs_extra::dir::{copy, CopyOptions};
use merkle_hash::MerkleTree;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
	let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
	let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

	let mut js_utils_path = PathBuf::from(manifest_dir.clone());
	js_utils_path.push("js");

	// Copy js-utils directory to out_dir
	let out_js_utils_path = out_dir.join("js-utils");
	let out_js_utils_path_dist = out_js_utils_path.join("dist");

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

	// Install deno
	let deno_dir = out_dir.join("deno");
	let deno_exec = deno_embed::get_executable(&deno_dir).await?;

	// Prepare the directory for `include_dir!`
	let status = Command::new(&deno_exec.executable_path)
		.env("DENO_NO_UPDATE_CHECK", "1")
		.arg("run")
		.arg("-A")
		.arg("build.ts")
		// Deno runs out of memory on Windows
		.env(
			"DENO_V8_FLAGS",
			"--max-heap-size=8192,--max-old-space-size=8192",
		)
		.current_dir(&out_js_utils_path)
		.status()?;
	if !status.success() {
		panic!("build js utils");
	}

	// TODO: This doesn't work
	// Removes files that are not cross-platform & deletes
	// broken symlinks.
	// strip_cross_platform(&out_js_utils_path)?;

	println!("cargo:rerun-if-changed={}", js_utils_path.display());
	println!(
		"cargo:rustc-env=JS_UTILS_DIST_PATH={}",
		out_js_utils_path_dist.display()
	);
	println!(
		"cargo:rustc-env=JS_UTILS_DIST_HASH={}",
		hash_directory(&out_js_utils_path_dist)?
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
