use anyhow::*;
use merkle_hash::MerkleTree;
use serde_json::json;
use std::{
	fs,
	path::{Path, PathBuf},
	process::Stdio,
};

#[tokio::main]
async fn main() -> Result<()> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = std::env::var("OUT_DIR")?;

	let sdk_path = PathBuf::from(manifest_dir.clone()).join("../../../sdks/actor");

	// Copy SDK directory to out_dir
	let dist_path = Path::new(&out_dir).join("actor-sdk");

	// Remove old dir
	if dist_path.is_dir() {
		fs::remove_dir_all(&dist_path).context("fs::remove_dir_all")?;
	}

	// Build manager
	build_backend_command_raw(CommandOpts {
		task_path: "src/tasks/build/mod.ts",
		input: json!({
			"projectRoot": sdk_path.join("manager"),
			"entryPoint": sdk_path.join("manager/src/mod.ts"),
			"outDir": dist_path.join("manager"),
			"bundle": {
				"minify": true,
				"analyzeResult": false,
				"logLevel": "debug"
			}
		}),
	})
	.await?;

	println!("cargo:rerun-if-changed={}", sdk_path.display());
	println!(
		"cargo:rustc-env=ACTOR_SDK_DIST_PATH={}",
		dist_path.display()
	);
	println!(
		"cargo:rustc-env=ACTOR_SDK_DIST_HASH={}",
		hash_directory(&dist_path)?
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

pub struct CommandOpts {
	pub task_path: &'static str,
	pub input: serde_json::Value,
}

// TODO: Split toolchain's js_utils in to a separate crate so we can share this code
pub async fn build_backend_command_raw(opts: CommandOpts) -> Result<()> {
	// We can't use tempdir because it breaks something with deno
	let data_dir = tempfile::tempdir()?;

	// Get Deno executable
	let deno = deno_embed::get_executable(&data_dir.path().to_owned()).await?;

	// Get JS utils
	let base = rivet_js_utils_embed::src_path(&data_dir.path().to_owned()).await?;

	// Serialize command
	let input_json = serde_json::to_string(&opts.input)?;

	// Run backend
	let status = tokio::process::Command::new(deno.executable_path)
		.args([
			"run",
			"--allow-all",
			"--unstable-sloppy-imports",
			"--vendor", // Required for unenv files to be readable
		])
		.arg(&opts.task_path)
		.arg("--input")
		.arg(input_json)
		.env("DENO_NO_UPDATE_CHECK", "1")
		.current_dir(&base)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()
		.await?;
	ensure!(status.success(), "command failed");

	Ok(())
}
