use std::{
	fs,
	path::Path,
	process::{Child, Command},
	sync::mpsc::Receiver,
	time::Duration,
};

use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

use super::mock_vector;

pub struct Setup {
	pub actor_id: String,
	pub actor_dir: tempfile::TempDir,
	pub socket_dir: tempfile::TempDir,
	pub child: Child,
	pub msg_rx: Receiver<super::mock_vector::VectorMessage>,
}

impl Setup {
	pub fn signal_child(&self, signal: &str) {
		let kill = Command::new("kill")
			.args(["-s", signal, &self.child.id().to_string()])
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null())
			.status()
			.unwrap();
		assert!(kill.success());
	}
}

impl Drop for Setup {
	fn drop(&mut self) {
		println!("Deleting container");
		Command::new("runc")
			.arg("delete")
			.arg("--force")
			.arg(&self.actor_id)
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null())
			.status()
			.unwrap();

		println!("Killing child process");
		self.child.kill().unwrap();
	}
}

pub fn setup(command: &str) -> Setup {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

	let actor_id = Uuid::new_v4().to_string();
	let actor_dir = tempdir().unwrap();

	// Spawn listener, wait for start
	let (msg_tx, msg_rx) = std::sync::mpsc::channel();
	let socket_dir = tempfile::tempdir().unwrap();
	let socket_port = portpicker::pick_unused_port().expect("no free ports");
	std::thread::spawn(move || mock_vector::listener(socket_port, msg_tx).unwrap());
	std::thread::sleep(Duration::from_secs(1));

	// Extract OCI bundle
	let status = Command::new("skopeo")
		.arg("copy")
		.arg("docker://debian:12.4")
		.arg("oci:oci-image:latest")
		.current_dir(actor_dir.path())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.unwrap();
	assert!(status.success());

	let status = Command::new("umoci")
		.arg("unpack")
		.arg("--image")
		.arg("oci-image:latest")
		.arg("fs")
		.current_dir(actor_dir.path())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.unwrap();
	assert!(status.success());

	// Generate runc container
	let oci_bundle_path = actor_dir.path().join("fs");
	let oci_config_path = oci_bundle_path.join("config.json");
	let mut config =
		serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&oci_config_path).unwrap())
			.unwrap();
	config["process"]["terminal"] = json!(false);
	config["process"]["args"] = json!(["/bin/sh", "-c", command]);
	fs::write(
		&oci_config_path,
		serde_json::to_string_pretty(&config).unwrap(),
	)
	.unwrap();

	// Spawn runner
	let child = Command::new(
		manifest_dir
			.join("..")
			.join("..")
			.join("..")
			.join("target")
			.join("debug")
			.join(env!("CARGO_PKG_NAME")),
	)
	.arg(actor_dir.path())
	.env(
		"PEGBOARD_META_vector_socket_addr",
		format!("127.0.0.1:{socket_port}"),
	)
	.env("PEGBOARD_META_root_user_enabled", "1")
	.env("PEGBOARD_META_owner", "dynamic_server")
	.env("PEGBOARD_META_server_id", "")
	.spawn()
	.expect("Failed to spawn child process");

	// Give the process time to start so it will be able to catch SIGTERM signals
	std::thread::sleep(Duration::from_secs(1));

	Setup {
		actor_id,
		actor_dir,
		socket_dir,
		child,
		msg_rx,
	}
}
