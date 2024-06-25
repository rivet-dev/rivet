use std::path::PathBuf;

use tokio::fs;
use merkle_hash::MerkleTree;

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

	let tree = MerkleTree::builder(server_install_path.display().to_string())
		.hash_names(true)
		.build()
		.unwrap();
	let source_hash = hex::encode(tree.root.item.hash);

	fs::write(out_dir.join("hash.txt"), source_hash.trim())
		.await
		.unwrap();
}
