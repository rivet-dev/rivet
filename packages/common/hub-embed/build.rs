use std::{env, fs, path::Path, process::Command, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Get the output directory from the cargo environment variable
	let target_dir = env::var("OUT_DIR")?;
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = Path::new(&target_dir);

	let project_root = PathBuf::from(manifest_dir.clone()).join("../../..");
	let hub_path = project_root.join("frontend/apps/hub");

	// Build hub
	println!("Running yarn install");
	let status = Command::new("yarn")
		.arg("install")
		.arg("--immutable")
		.current_dir(&hub_path)
		.status()?;
	assert!(status.success(), "yarn install failed");

	println!("Running yarn build");
	let status = Command::new("yarn")
		.current_dir(&hub_path)
		.args(["dlx", "turbo", "run", "build:embedded"])
        // This will be substituted at runtime
        .env("VITE_APP_API_URL", "__APP_API_URL__")
		.status()?;
	assert!(status.success(), "hub build failed");

	// Copy dist directory to out_dir
	let dist_path = hub_path.join("dist");
	fs::create_dir_all(out_dir)?;
	fs_extra::dir::copy(
		dist_path,
		out_dir,
		&fs_extra::dir::CopyOptions::new().overwrite(true),
	)?;

	// Set the path in the env
	println!("cargo:rustc-env=HUB_PATH={}", out_dir.display());

	println!("cargo:rerun-if-changed={}", hub_path.display());

	Ok(())
}
