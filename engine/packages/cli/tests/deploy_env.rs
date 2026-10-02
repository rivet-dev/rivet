#![cfg(unix)]

use std::{
	fs,
	io::{Read, Write},
	net::{TcpListener, TcpStream},
	os::unix::fs::PermissionsExt,
	process::Command,
	thread,
	time::Duration,
};

#[test]
fn malformed_env_has_no_deployment_side_effects() {
	for reuse_image in [false, true] {
		for existing_credentials in [false, true] {
			for (env, error) in [
				("PORT", "--env must be KEY=VAL, got PORT"),
				("=value", "--env key cannot be empty"),
				("", "--env must be KEY=VAL, got "),
			] {
				let temp = tempfile::tempdir().unwrap();
				let credentials = temp.path().join(".rivet/credentials");
				let original = b"{\"rivet_cloud_token\":\"original-test-token\"}";
				if existing_credentials {
					fs::create_dir(credentials.parent().unwrap()).unwrap();
					fs::write(&credentials, original).unwrap();
				}
				fs::write(temp.path().join("Dockerfile"), "FROM scratch\n").unwrap();
				let docker = temp.path().join("docker");
				fs::write(
					&docker,
					"#!/bin/sh\nprintf called > docker-called\nexit 1\n",
				)
				.unwrap();
				fs::set_permissions(&docker, fs::Permissions::from_mode(0o755)).unwrap();

				let listener = TcpListener::bind("127.0.0.1:0").unwrap();
				let address = listener.local_addr().unwrap();
				let recorder = thread::spawn(move || {
					let mut requests = 0;
					for stream in listener.incoming() {
						let mut stream = stream.unwrap();
						stream
							.set_read_timeout(Some(Duration::from_secs(5)))
							.unwrap();
						let mut request = [0; 4];
						stream.read_exact(&mut request).unwrap();
						if &request == b"stop" {
							break;
						}
						requests += 1;
						stream
							.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
							.unwrap();
					}
					requests
				});

				let mut command = Command::new(env!("CARGO_BIN_EXE_rivet"));
				command
					.current_dir(temp.path())
					.env("HOME", temp.path())
					.env("PATH", temp.path())
					.env_remove("BASH_ENV")
					.env_remove("RIVET_CLOUD_TOKEN")
					.env("NO_PROXY", "*")
					.env("RUST_BACKTRACE", "0")
					.args(["deploy", "--token", "disposable-test-token", "--yes"])
					.args(["--cloud-api", &format!("http://{address}")])
					.args(["--env", "BEFORE=ok", "--env", env, "--env", "AFTER=ok"]);
				if reuse_image {
					command.arg("--reuse-image");
				}
				let output = command.output().unwrap();
				TcpStream::connect(address)
					.unwrap()
					.write_all(b"stop")
					.unwrap();
				let requests = recorder.join().unwrap();
				assert!(!output.status.success());
				assert_eq!(
					String::from_utf8(output.stderr).unwrap(),
					format!("Error: {error}\n")
				);
				assert_eq!(requests, 0);
				assert!(!temp.path().join("docker-called").exists());
				if existing_credentials {
					assert_eq!(fs::read(&credentials).unwrap(), original);
				} else {
					assert!(!credentials.parent().unwrap().exists());
				}
			}
		}
	}
}

#[test]
fn malformed_env_precedes_missing_token() {
	let temp = tempfile::tempdir().unwrap();
	let output = Command::new(env!("CARGO_BIN_EXE_rivet"))
		.current_dir(temp.path())
		.env("HOME", temp.path())
		.env_remove("RIVET_CLOUD_TOKEN")
		.env("RUST_BACKTRACE", "0")
		.args(["deploy", "--env", "PORT"])
		.output()
		.unwrap();
	assert!(!output.status.success());
	assert_eq!(
		String::from_utf8(output.stderr).unwrap(),
		"Error: --env must be KEY=VAL, got PORT\n"
	);
}
