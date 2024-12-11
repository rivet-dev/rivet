use anyhow::*;
use merkle_hash::MerkleTree;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = std::env::var("OUT_DIR")?;

	let mut js_utils_path = PathBuf::from(manifest_dir.clone());
	js_utils_path.push("js");

	// Copy js-utils directory to out_dir
	let out_js_utils_path = Path::new(&out_dir).join("js-utils");

	// Remove old dir
	if out_js_utils_path.is_dir() {
		fs::remove_dir_all(&out_js_utils_path).context("fs::remove_dir_all")?;
	}

	// Copy js-utils directory to out_dir
	// TODO: This breaks    deno check``
	// let mut copy_options = CopyOptions::new();
	// copy_options.overwrite = true;
	// copy_options.copy_inside = true;
	// copy(&js_utils_path, &out_js_utils_path, &copy_options).with_context(|| {
	// 	format!(
	// 		"failed to copy directory from {} to {}",
	// 		js_utils_path.display(),
	// 		out_js_utils_path.display()
	// 	)
	// })?;

	let status = std::process::Command::new("cp")
		.arg("-R")
		.arg(&js_utils_path)
		.arg(&out_js_utils_path)
		.status()
		.with_context(|| {
			format!(
				"failed to copy directory from {} to {}",
				js_utils_path.display(),
				out_js_utils_path.display()
			)
		})?;
	if !status.success() {
		return Err(anyhow!("cp command failed"));
	}

	// Install deno
	let deno_dir = Path::new(&out_dir).join("deno");
	let deno_exec = deno_embed::get_executable(&deno_dir).await?;

	// Prepare the directory for `include_dir!`
	let status = Command::new(&deno_exec.executable_path)
		.env("DENO_NO_UPDATE_CHECK", "1")
		.arg("task")
		.arg("prepare")
		// Deno runs out of memory on Windows
		.env(
			"DENO_V8_FLAGS",
			"--max-heap-size=8192,--max-old-space-size=8192",
		)
		.current_dir(&out_js_utils_path)
		.status()?;
	if !status.success() {
		panic!("cache dependencies failed");
	}

	// TODO: This doesn't work
	// Removes files that are not cross-platform & deletes
	// broken symlinks.
	// strip_cross_platform(&out_js_utils_path)?;

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

// fn strip_cross_platform(path: &Path) -> Result<()> {
// 	// Remove directories starting with "@esbuild+"
// 	let esbuild_path = path.join("node_modules").join(".deno");
// 	let output = Command::new("find")
// 		.arg(&esbuild_path)
// 		.arg("-type")
// 		.arg("d")
// 		.arg("-name")
// 		.arg("@esbuild+*")
// 		.arg("-exec")
// 		.arg("rm")
// 		.arg("-rf")
// 		.arg("{}")
// 		.arg("+")
// 		.output()
// 		.context("Failed to execute 'find' command to remove @esbuild+ directories")?;
//
// 	if !output.status.success() {
// 		return Err(anyhow!(
// 			"Failed to remove @esbuild+ directories. Path: {}, Status: {}, Stdout: {}, Stderr: {}",
// 			esbuild_path.display(),
// 			output.status,
// 			String::from_utf8_lossy(&output.stdout),
// 			String::from_utf8_lossy(&output.stderr)
// 		));
// 	}
//
// 	// Remove broken symlinks
// 	let output = Command::new("find")
// 		.arg(path)
// 		.arg("-type")
// 		.arg("l")
// 		.arg("-exec")
// 		.arg("sh")
// 		.arg("-c")
// 		.arg("for x; do [ -e \"$x\" ] || rm \"$x\"; done")
// 		.arg("{}")
// 		.arg("+")
// 		.output()
// 		.context("Failed to execute 'find' command to remove broken symlinks")?;
//
// 	if !output.status.success() {
// 		return Err(anyhow!(
// 			"Failed to remove broken symlinks. Status: {}, Stdout: {}, Stderr: {}",
// 			output.status,
// 			String::from_utf8_lossy(&output.stdout),
// 			String::from_utf8_lossy(&output.stderr)
// 		));
// 	}
//
// 	Ok(())
// }
