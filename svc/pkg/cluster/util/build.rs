use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tokio::{fs, process::Command};

// NOTE: This only gets the hash of the folder. Any template variables changed in the install scripts
// will not update the hash.
// Get a hash of the server install worker
#[tokio::main]
async fn main() {
	let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
	let current_dir = std::env::current_dir().unwrap();
	let server_install_path = {
		let mut dir = current_dir.clone();
		dir.pop();

		dir.join("worker")
			.join("src")
			.join("workers")
			.join("server_install")
	};

	// Add rereun statement
	println!("cargo:rerun-if-changed={}", server_install_path.display());

	let mut util_path = current_dir.clone();
	util_path.pop();
	let util_path = util_path
		.join("worker")
		.join("src")
		.join("workers")
		.join("server_install");

	// Compute the git diff between the current branch and the local changes
	let cmd = Command::new("git")
		.arg("diff")
		.arg("--minimal")
		.arg("HEAD")
		.arg("--")
		.arg(util_path)
		.output()
		.await
		.unwrap();

	if !cmd.status.success() {
		panic!(
			"failed to get git diff ({}):\n{}",
			cmd.status,
			String::from_utf8(cmd.stderr).unwrap()
		);
	}

	let source_diff = String::from_utf8(cmd.stdout).unwrap();

	// If there is no diff, use the git commit hash
	let source_hash = if source_diff.is_empty() {
		let cmd = Command::new("git")
			.arg("rev-parse")
			.arg("HEAD:svc/pkg/cluster/worker/src/workers/server_install")
			.output()
			.await
			.unwrap();

		if !cmd.status.success() {
			panic!(
				"failed to get git diff ({}):\n{}",
				cmd.status,
				String::from_utf8(cmd.stderr).unwrap()
			);
		}

		String::from_utf8(cmd.stdout).unwrap()
	} else {
		// Get hash of diff
		hex::encode(Sha256::digest(source_diff.as_bytes()))
	};

	fs::write(out_dir.join("hash.txt"), source_hash.trim())
		.await
		.unwrap();
}
