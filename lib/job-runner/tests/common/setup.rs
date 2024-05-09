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
	pub container_id: String,
	pub alloc_dir: tempfile::TempDir,
	pub socket_dir: tempfile::TempDir,
	pub child: Child,
	pub msg_rx: Receiver<super::mock_vector::VectorMessage>,
}

impl Setup {
	pub fn signal_child(&self, signal: &str) {
		let kill = Command::new("kill")
			.args(["-s", signal, &self.child.id().to_string()])
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
			.arg(&self.container_id)
			.status()
			.unwrap();

		println!("Killing child process");
		self.child.kill().unwrap();
	}
}

pub fn setup(command: &str) -> Setup {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

	let container_id = Uuid::new_v4().to_string();
	let job_run_id = Uuid::new_v4().to_string();
	let alloc_dir = tempdir().unwrap();

	// Spawn listener, wait for start
	let (msg_tx, msg_rx) = std::sync::mpsc::channel();
	let socket_dir = tempfile::tempdir().unwrap();
	let socket_port = portpicker::pick_unused_port().expect("no free ports");
	std::thread::spawn(move || mock_vector::listener(socket_port, msg_tx).unwrap());
	std::thread::sleep(Duration::from_secs(1));

	// Build alloc dir
	fs::write(alloc_dir.path().join("container-id"), &container_id).unwrap();

	// Extract OCI bundle
	let status = Command::new("skopeo")
		.arg("copy")
		.arg("docker://debian:12.4")
		.arg("oci:oci-image:latest")
		.current_dir(alloc_dir.path())
		.status()
		.unwrap();
	assert!(status.success());

	let status = Command::new("umoci")
		.arg("unpack")
		.arg("--image")
		.arg("oci-image:latest")
		.arg("oci-bundle")
		.current_dir(alloc_dir.path())
		.status()
		.unwrap();
	assert!(status.success());

	// Generate runc container
	let oci_bundle_path = alloc_dir.path().join("oci-bundle");
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
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.join("target")
			.join("debug")
			.join(env!("CARGO_PKG_NAME")),
	)
	.env(
		"NOMAD_META_vector_socket_addr",
		format!("127.0.0.1:{socket_port}"),
	)
	.env("NOMAD_ALLOC_DIR", alloc_dir.path())
	.env("NOMAD_META_job_run_id", job_run_id)
	.env("NOMAD_TASK_NAME", "main")
	.spawn()
	.expect("Failed to spawn child process");

	// Give the process time to start so it will be able to catch SIGTERM signals
	std::thread::sleep(Duration::from_secs(1));

	Setup {
		container_id,
		alloc_dir,
		socket_dir,
		child,
		msg_rx,
	}
}
