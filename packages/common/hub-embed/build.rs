use std::{env, fs, path::Path, path::PathBuf, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Check if yarn is installed
	let yarn_check = Command::new("yarn").arg("--version").status();
	assert!(
		yarn_check.is_ok() && yarn_check.unwrap().success(),
		"yarn is not installed, please install yarn to build this project"
	);

	// Get the output directory from the cargo environment variable
	let target_dir = env::var("OUT_DIR")?;
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let out_dir = Path::new(&target_dir);

	let project_root = PathBuf::from(manifest_dir.clone()).join("../../..");
	let hub_path = project_root.join("frontend/apps/hub");

	// Build hub
	println!("Running yarn install");
	let output = Command::new("yarn")
		.arg("install")
		.arg("--immutable")
		.current_dir(&hub_path)
		.output()?;
	println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
	println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
	assert!(output.status.success(), "yarn install failed");

	println!("Running yarn build");
	let output = Command::new("yarn")
		.current_dir(&hub_path)
		.args(["dlx", "turbo", "run", "build:embedded"])
		.env("VITE_APP_API_URL", "__APP_API_URL__")
		.output()?;
	println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
	println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));
	assert!(output.status.success(), "hub build failed");

	// Copy dist directory to out_dir
	let dist_path = hub_path.join("dist");
	if out_dir.exists() {
		fs::remove_dir_all(out_dir)?;
	}
	fs_extra::dir::copy(
		dist_path.clone(),
		out_dir,
		&fs_extra::dir::CopyOptions::new().content_only(true),
	)?;

	// Set the path in the env
	println!("cargo:rustc-env=HUB_PATH={}", out_dir.display());

	println!("cargo:rerun-if-changed={}", hub_path.display());

	// Relevant package tokens
	println!("cargo:rerun-if-env-changed=FONTAWESOME_PACKAGE_TOKEN");

	Ok(())
}
