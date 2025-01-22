use anyhow::*;
use fs_extra::dir::{copy, CopyOptions};
use merkle_hash::MerkleTree;
use std::{path::{Path, PathBuf}, fs, process::Stdio};

#[tokio::main]
async fn main() -> Result<()> {
	let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
	let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

	let src_path = PathBuf::from(manifest_dir.clone()).join("js");

	let dst_path = out_dir.join("js-utils");

	// Remove old dir
	if dst_path.is_dir() {
		fs::remove_dir_all(&dst_path).context("fs::remove_dir_all")?;
	}

	// Copy source files to target directory first (ignore vendored files)
	let copy_options = CopyOptions::new().overwrite(true).copy_inside(true);

	copy(&src_path.join("src"), &dst_path.join("src"), &copy_options)?;

	fs::copy(&src_path.join("deno.json"), &dst_path.join("deno.json")).context("copy deno.json")?;
	fs::copy(&src_path.join("deno.lock"), &dst_path.join("deno.lock")).context("copy deno.lock")?;

	//// Run deno install in the new directory
	//let data_dir = tempfile::tempdir()?;
	//let deno = deno_embed::get_executable(&data_dir.path().to_owned()).await?;
	////let status = tokio::process::Command::new(deno.executable_path)
	////	.args(["install", "--allow-scripts"])
	////	.current_dir(&out_js_utils_path)
	////	.stdout(Stdio::inherit())
	////	.stderr(Stdio::inherit())
	////	.status()
	////	.await?;
	////let status = tokio::process::Command::new(deno.executable_path)
	////	.args(["run", "-A", "--node-modules-dir=auto", "--allow-scripts", "npm:yarn", "install"])
	////	.current_dir(&out_js_utils_path)
	////	.stdout(Stdio::inherit())
	////	.stderr(Stdio::inherit())
	////	.status()
	////	.await?;
	//let status = tokio::process::Command::new("touch")
	//	.arg("yarn.lock")
	//	.current_dir(&out_js_utils_path)
	//	.stdout(Stdio::inherit())
	//	.stderr(Stdio::inherit())
	//	.status()
	//	.await?;
	//let status = tokio::process::Command::new("yarn")
	//	.arg("install")
	//	.current_dir(&out_js_utils_path)
	//	.stdout(Stdio::inherit())
	//	.stderr(Stdio::inherit())
	//	.status()
	//	.await?;
	//ensure!(status.success(), "deno install command failed");

	println!("cargo:rerun-if-changed={}", src_path.display());
	println!(
		"cargo:rustc-env=JS_UTILS_PATH={}",
		dst_path.display()
	);
	println!(
		"cargo:rustc-env=JS_UTILS_HASH={}",
		hash_directory(&dst_path)?
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
