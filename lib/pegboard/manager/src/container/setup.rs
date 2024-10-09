use std::{
	os::unix::process::CommandExt,
	path::{Path, PathBuf},
	process::Stdio,
};

use anyhow::*;
use futures_util::StreamExt;
use indoc::indoc;
use lz4_flex::frame::FrameDecoder;
use nix::{
	sys::wait::{waitpid, WaitStatus},
	unistd::{fork, pipe, read, write, ForkResult, Pid},
};
use pegboard::protocol;
use serde_json::{json, Value};
use tar::Archive;
use tokio::{
	fs::{self, File},
	io::AsyncWriteExt,
	process::Command,
	task,
};
use uuid::Uuid;

use super::oci_config;
use crate::ctx::Ctx;

pub async fn cni_bundle(
	container_id: Uuid,
	config: &protocol::ContainerConfig,
	ctx: &Ctx,
) -> Result<()> {
	let container_path = ctx.container_path(container_id);
	let oci_bundle_path = container_path.join("oci-bundle");
	let netns_path = netns_path(container_id, &config.network_mode);

	// Write base config
	fs::write(
		container_path.join("oci-bundle-config.base.json"),
		serde_json::to_vec(&oci_config::config(
			config.resources.cpu,
			config.resources.memory,
			config.resources.memory_max,
			vec!["RIVET_API_ENDPOINT".to_string(), {
				ctx.api_endpoint
					.read()
					.await
					.clone()
					.context("missing api endpoint")?
			}],
		))?,
	)
	.await?;

	let mut stream = reqwest::get(&config.image.artifact_url)
		.await?
		.error_for_status()?
		.bytes_stream();

	match config.image.kind {
		protocol::ImageKind::DockerImage => {
			let docker_image_path = container_path.join("docker-image.tar");
			let oci_image_path = container_path.join("oci-image");

			let mut output_file = File::create(&docker_image_path).await?;

			// Write from stream to file
			while let Some(chunk) = stream.next().await {
				output_file.write_all(&chunk?).await?;
			}

			if let protocol::ImageCompression::Lz4 = config.image.compression {
				// Rename to tmp name
				let tmp_path = container_path.join("tmp.tar");
				fs::rename(&docker_image_path, &tmp_path).await?;

				let docker_image_path = docker_image_path.clone();
				let tmp_path2 = tmp_path.clone();
				task::spawn_blocking(move || {
					let input_file = std::fs::File::open(tmp_path2)?;
					let mut output_file = std::fs::File::create(docker_image_path)?;

					let mut decoder = FrameDecoder::new(input_file);
					std::io::copy(&mut decoder, &mut output_file)
				})
				.await??;

				// Delete tmp file
				fs::remove_file(&tmp_path).await?;
			}

			// We need to convert the Docker image to an OCI bundle in order to run it.
			// Allows us to work with the build with umoci
			tracing::debug!("Converting Docker image -> OCI image");
			Command::new("skopeo")
				.arg("copy")
				.arg(format!("docker-archive:{}", docker_image_path.display()))
				.arg(format!("oci:{}:default", oci_image_path.display()))
				.output()
				.await?;

			// Allows us to run the bundle natively with runc
			tracing::debug!("Converting OCI image -> OCI bundle");

			Command::new("umoci")
				.arg("unpack")
				.arg("--image")
				.arg(format!("{}:default", oci_image_path.display()))
				.arg(&oci_bundle_path)
				.output()
				.await?;
		}
		protocol::ImageKind::OciBundle => {
			tracing::debug!("Downloading OCI bundle");

			let tmp_path = container_path.join("tmp-bundle");
			let mut output_file = File::create(&tmp_path).await?;

			// Write from stream to file
			while let Some(chunk) = stream.next().await {
				output_file.write_all(&chunk?).await?;
			}

			fs::create_dir(&oci_bundle_path).await?;

			let oci_bundle_path = oci_bundle_path.clone();
			let tmp_path2 = tmp_path.clone();
			let compression = config.image.compression;
			task::spawn_blocking(move || {
				let input_file = std::fs::File::open(tmp_path2)?;

				match compression {
					protocol::ImageCompression::None => {
						let mut archive = Archive::new(input_file);

						archive.unpack(oci_bundle_path)?;
					}
					protocol::ImageCompression::Lz4 => {
						let decoder = FrameDecoder::new(input_file);
						let mut archive = Archive::new(decoder);

						archive.unpack(oci_bundle_path)?;
					}
				}

				Ok(())
			})
			.await??;

			// Delete tmp file
			fs::remove_file(&tmp_path).await?;
		}
	}

	// resolv.conf
	//
	// See also rivet-job.conflist in lib/bolt/core/src/dep/terraform/install_scripts/files/nomad.sh
	fs::write(
		container_path.join("resolv.conf"),
		indoc!(
			"
			nameserver 8.8.8.8
			nameserver 8.8.4.4
			nameserver 2001:4860:4860::8888
			nameserver 2001:4860:4860::8844
			options rotate
			options edns0
			options attempts:2
			"
		),
	)
	.await?;

	// MARK: Config
	//
	// Sanitize the config.json by copying safe properties from the provided bundle in to our base config.
	//
	// This way, we enforce our own capabilities on the container instead of trusting the
	// provided config.json
	tracing::debug!("Templating config.json");
	let config_path = oci_bundle_path.join("config.json");
	let override_config_path = container_path.join("oci-bundle-config.overrides.json");
	fs::rename(&config_path, &override_config_path).await?;

	// TODO: get new envb in here somehow
	// TODO: check bounds

	// Template new config
	let base_config_path = container_path.join("oci-bundle-config.base.json");
	let base_config_json = fs::read_to_string(&base_config_path).await?;
	let mut base_config = serde_json::from_str::<Value>(&base_config_json)?;

	let override_config_json = fs::read_to_string(&override_config_path).await?;
	let override_config = serde_json::from_str::<Value>(&override_config_json)?;
	let override_process = override_config["process"].clone();

	base_config["process"]["args"] = override_process["args"].clone();
	base_config["process"]["env"] = Value::Object(
		config
			.env
			.iter()
			.map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
			.chain(
				override_process["env"]
					.as_object()
					.context("override_process.env")?
					.clone()
					.into_iter(),
			)
			.chain(
				base_config["process"]["env"]
					.as_object()
					.context("process.env")?
					.clone()
					.into_iter(),
			)
			.collect(),
	);
	base_config["process"]["user"] = override_process["user"].clone();
	base_config["process"]["cwd"] = override_process["cwd"].clone();
	base_config["linux"]["namespaces"]
		.as_array_mut()
		.context("config.linux.namespaces")?
		.push(json!({
			"type": "network",
			"path": netns_path.to_str().context("netns_path")?,
		}));
	base_config["mounts"]
		.as_array_mut()
		.context("config.mounts")?
		.push(json!({
			"destination": "/etc/resolv.conf",
			"type": "bind",
			"source": container_path.join("resolv.conf").to_str().context("resolv.conf path")?,
			"options": ["rbind", "rprivate"]
		}));

	fs::write(&config_path, serde_json::to_vec(&base_config)?).await?;

	Ok(())
}

pub async fn cni_network(
	container_id: Uuid,
	config: &protocol::ContainerConfig,
	ctx: &Ctx,
) -> Result<()> {
	let container_path = ctx.container_path(container_id);
	let netns_path = netns_path(container_id, &config.network_mode);

	let cni_port_mappings = config
		.ports
		.iter()
		.map(|(_, port)| {
			// Pick random port that isn't taken
			let host_port = todo!();

			json!({
				"HostPort": host_port,
				"ContainerPort": port.internal_port,
				"Protocol": port.proxy_protocol.to_string(),
			})
		})
		.collect::<Vec<_>>();

	// MARK: Generate CNI parameters
	//
	// See https://github.com/containernetworking/cni/blob/b62753aa2bfa365c1ceaff6f25774a8047c896b5/cnitool/cnitool.go#L31

	// See Nomad capabilities equivalent:
	// https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_cni.go#L105C46-L105C46
	//
	// See supported args:
	// https://github.com/containerd/go-cni/blob/6603d5bd8941d7f2026bb5627f6aa4ff434f859a/namespace_opts.go#L22
	let cni_params = json!({
		"portMappings": cni_port_mappings,
	});
	let cni_params_json = serde_json::to_string(&cni_params)?;
	fs::write(
		container_path.join("cni-cap-args.json"),
		cni_params_json.as_bytes(),
	)
	.await?;

	// MARK: Create network
	//
	// See Nomad network creation:
	// https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/network_manager_linux.go#L119

	// Name of the network in /opt/cni/config/$NETWORK_NAME.conflist
	let network_name = "rivet-pegboard";

	tracing::debug!("Creating network {}", container_id);

	Command::new("ip")
		.arg("netns")
		.arg("add")
		.arg(container_id.to_string())
		.output()
		.await
		.context("Failed to create network namespace")?;

	tracing::debug!(
		"Adding network {} to namespace {}",
		network_name,
		netns_path.display(),
	);
	Command::new("cnitool")
		.arg("add")
		.arg(network_name)
		.arg(netns_path)
		.env("CNI_PATH", "/opt/cni/bin")
		.env("NETCONFPATH", "/opt/cni/config")
		.env("CNI_IFNAME", "eth0")
		.env("CAP_ARGS", cni_params_json)
		.output()
		.await
		.context("Failed to add network to namespace")?;

	tracing::debug!("Finished setting up CNI network");

	Ok(())
}

// Path to the created namespace
fn netns_path(container_id: Uuid, network_mode: &protocol::NetworkMode) -> PathBuf {
	if let protocol::NetworkMode::Bridge = network_mode {
		// Host network
		Path::new("/proc/1/ns/net").to_path_buf()
	} else {
		// CNI network that will be created
		Path::new("/var/run/netns").join(container_id.to_string())
	}
}

pub fn spawn_orphaned_container_runner(
	container_runner_path: PathBuf,
	container_path: PathBuf,
	env: &[(&str, String)],
) -> Result<Pid> {
	// Prepare the arguments for the container runner
	let runner_args = vec![container_path.to_str().context("bad path")?];

	// Pipe communication between processes
	let (pipe_read, pipe_write) = pipe()?;

	// NOTE: This is why we fork the process twice: https://stackoverflow.com/a/5386753
	match unsafe { fork() }.context("process first fork failed")? {
		ForkResult::Parent { child } => {
			// Close the writing end of the pipe in the parent
			nix::unistd::close(pipe_write)?;

			// Ensure that the child process spawned successfully
			match waitpid(child, None).context("waitpid failed")? {
				WaitStatus::Exited(_, 0) => {
					// Read the second child's PID from the pipe
					let mut buf = [0u8; 4];
					read(pipe_read, &mut buf)?;
					let second_child_pid = Pid::from_raw(i32::from_le_bytes(buf));

					Ok(second_child_pid)
				}
				WaitStatus::Exited(_, status) => {
					bail!("Child process exited with status {}", status)
				}
				_ => bail!("Unexpected wait status for child process"),
			}
		}
		ForkResult::Child => {
			// Child process
			match unsafe { fork() } {
				Result::Ok(ForkResult::Parent { child }) => {
					// Write the second child's PID to the pipe
					let child_pid_bytes = child.as_raw().to_le_bytes();
					write(pipe_write, &child_pid_bytes)?;

					// Exit the intermediate child
					std::process::exit(0);
				}
				Result::Ok(ForkResult::Child) => {
					// Exit immediately on fail in order to not leak process
					let err = std::process::Command::new(&container_runner_path)
						.args(&runner_args)
						.envs(env.iter().cloned())
						.stdin(Stdio::null())
						.stdout(Stdio::null())
						.stderr(Stdio::null())
						.exec();
					eprintln!("exec failed: {err:?}");
					std::process::exit(1);
				}
				Err(err) => {
					// Exit immediately in order to not leak child process.
					//
					// The first fork doesn't need to exit on error since it
					eprintln!("process second fork failed: {err:?}");
					std::process::exit(1);
				}
			}
		}
	}
}
